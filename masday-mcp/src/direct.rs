//! Direct-call adapter for standalone stdio mode (SQLite)
//!
//! Each function uses rusqlite directly against ~/.masday/data.db.
//! Takes `serde_json::Value` args, returns `Result<Value, Box<dyn Error + Send + Sync>>`.

use rusqlite::params;
use serde_json::{json, Value};
use tracing::{error, info, warn};

/// Load the capability registry from `.claude/registry.json`.
/// Returns the parsed JSON object, or an empty registry on failure.
fn load_registry(project_root: &str) -> Value {
    let path = std::path::Path::new(project_root).join(".claude/registry.json");
    match std::fs::read_to_string(&path) {
        Ok(content) => serde_json::from_str(&content).unwrap_or_else(|e| {
            warn!("load_registry: failed to parse {}: {e}", path.display());
            json!({"version": 1, "components": {"agents": [], "skills": [], "hooks": [], "mcpServers": []}})
        }),
        Err(e) => {
            warn!("load_registry: failed to read {}: {e}", path.display());
            json!({"version": 1, "components": {"agents": [], "skills": [], "hooks": [], "mcpServers": []}})
        }
    }
}

/// Timestamp helper — returns current UTC time as RFC 3339 string.
fn now() -> String {
    chrono::Utc::now().to_rfc3339()
}

/// New UUID string.
fn new_id() -> String {
    uuid::Uuid::new_v4().to_string()
}

/// Error helper — converts any error to the boxed type the registry expects.
fn err(msg: impl std::fmt::Display) -> Box<dyn std::error::Error + Send + Sync> {
    format!("{}", msg).into()
}

/// Generate embedding vector for text content in standalone mode (synchronous).
///
/// In standalone mode, we only support the mock provider (feature hashing) since
/// SQLite doesn't support vector operations anyway. For Ollama/OpenAI embeddings,
/// use HTTP mode (local.rs) which sends embeddings to the PostgreSQL API.
///
/// Returns None on failure (doesn't crash), Some(Vec<f64>) on success.
fn generate_embedding_standalone_sync(text: &str) -> Option<Vec<f64>> {
    // Only mock provider in standalone mode (SQLite has no vector support)
    let vector = crate::embedding::text_to_vector(text);
    Some(vector.into_iter().map(|v| v as f64).collect())
}

/// Parse a JSON text column, return default on failure (with logging).
fn json_col(row: &rusqlite::Row, idx: usize) -> Value {
    match row.get::<_, String>(idx) {
        Ok(s) => serde_json::from_str(&s).unwrap_or_else(|e| {
            warn!("json_col: JSON parse failed at column {idx}: {e}");
            json!({})
        }),
        Err(e) => {
            warn!("json_col: DB read failed at column {idx}: {e}");
            json!({})
        }
    }
}

/// Parse an optional JSON text column (with logging).
fn opt_json(row: &rusqlite::Row, idx: usize) -> Option<Value> {
    match row.get::<_, Option<String>>(idx) {
        Ok(Some(s)) => Some(serde_json::from_str(&s).unwrap_or_else(|e| {
            warn!("opt_json: JSON parse failed at column {idx}: {e}");
            json!({})
        })),
        Ok(None) => None,
        Err(e) => {
            warn!("opt_json: DB read failed at column {idx}: {e}");
            None
        }
    }
}

/// Collect query_map rows, logging any individual row failures.
fn collect_rows(rows: impl Iterator<Item = Result<Value, rusqlite::Error>>) -> Vec<Value> {
    rows.filter_map(|r| {
        r.map_err(|e| {
            warn!("collect_rows: row mapping failed: {e}");
            e
        })
        .ok()
    })
    .collect()
}

// ============================================================================
// Auto-store helpers (SQLite path)
// ============================================================================

/// Auto-store a memory in SQLite (best-effort, logs errors but doesn't fail).
/// Takes a &Connection to avoid deadlocking on the global Mutex.
#[allow(clippy::too_many_arguments)]
fn auto_store_memory_sqlite(
    conn: &rusqlite::Connection,
    workflow_id: &str,
    task_id: Option<&str>,
    memory_type: &str,
    summary: &str,
    content: &str,
    importance: f64,
    tags: &[&str],
) {
    // Dedup: skip if a system memory with the same summary already exists
    let exists: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM memories WHERE workflow_id=?1 AND created_by_agent='system' AND summary=?2",
            params![workflow_id, summary],
            |r| r.get(0),
        )
        .unwrap_or(0);
    if exists > 0 {
        info!(
            "Skipping auto-store: duplicate memory for workflow {}",
            workflow_id
        );
        return;
    }

    let id = new_id();
    let tags_json =
        serde_json::Value::Array(tags.iter().map(|t| serde_json::json!(t)).collect()).to_string();
    let t = now();

    if let Err(e) = conn.execute(
        "INSERT INTO memories (id, workflow_id, task_id, memory_type, summary, content, importance_score, created_by_agent, tags, created_at, updated_at) VALUES (?1,?2,?3,?4,?5,?6,?7,'system',?8,?9,?9)",
        params![id, workflow_id, task_id, memory_type, summary, content, importance, &tags_json, &t],
    ) {
        warn!("auto_store_memory_sqlite: failed for workflow {}: {e}", workflow_id);
    } else {
        info!("Auto-stored {} memory for workflow {}", memory_type, workflow_id);
    }
}

/// Auto-store a context document in SQLite (best-effort).
/// Takes a &Connection to avoid deadlocking on the global Mutex.
fn auto_store_context_document_sqlite(
    conn: &rusqlite::Connection,
    workflow_id: &str,
    source_type: &str,
    title: &str,
    content: &str,
) {
    let id = new_id();
    let t = now();

    if let Err(e) = conn.execute(
        "INSERT INTO context_documents (id, workflow_id, source_type, title, content, metadata, created_at, updated_at) VALUES (?1,?2,?3,?4,?5,'{}',?6,?6)",
        params![id, workflow_id, source_type, title, content, &t],
    ) {
        warn!(
            "auto_store_context_document_sqlite: failed for workflow {}: {e}",
            workflow_id
        );
    } else {
        info!(
            "Auto-stored context document '{}' for workflow {}",
            source_type, workflow_id
        );
    }
}

// ============================================================================
// System Readiness Checks
// ============================================================================

/// Check SQLite connectivity — runs SELECT 1.
fn check_sqlite() -> Value {
    let start = std::time::Instant::now();
    let conn = crate::sqlite::conn();
    match conn.execute_batch("SELECT 1") {
        Ok(_) => json!({
            "name": "sqlite_connectivity",
            "ready": true,
            "message": "SELECT 1 OK",
            "criticality": "critical",
            "duration_ms": start.elapsed().as_millis() as u64
        }),
        Err(e) => json!({
            "name": "sqlite_connectivity",
            "ready": false,
            "message": format!("SELECT 1 failed: {e}"),
            "criticality": "critical",
            "duration_ms": start.elapsed().as_millis() as u64
        }),
    }
}

/// Check SQLite integrity — runs PRAGMA quick_check(10).
fn check_sqlite_integrity() -> Value {
    let start = std::time::Instant::now();
    let conn = crate::sqlite::conn();
    match conn.query_row("PRAGMA quick_check(10)", [], |row| row.get::<_, String>(0)) {
        Ok(result) if result == "ok" => json!({
            "name": "sqlite_integrity",
            "ready": true,
            "message": "PRAGMA quick_check: ok",
            "criticality": "critical",
            "duration_ms": start.elapsed().as_millis() as u64
        }),
        Ok(result) => json!({
            "name": "sqlite_integrity",
            "ready": false,
            "message": format!("Integrity issue: {result}"),
            "criticality": "critical",
            "duration_ms": start.elapsed().as_millis() as u64
        }),
        Err(e) => json!({
            "name": "sqlite_integrity",
            "ready": false,
            "message": format!("PRAGMA quick_check failed: {e}"),
            "criticality": "critical",
            "duration_ms": start.elapsed().as_millis() as u64
        }),
    }
}

/// Check SQLite schema completeness — counts tables in sqlite_master.
fn check_sqlite_schema() -> Value {
    let start = std::time::Instant::now();
    let conn = crate::sqlite::conn();
    let table_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table'",
            [],
            |row| row.get(0),
        )
        .unwrap_or(0);

    let expected = 16i64;
    let ready = table_count >= expected;
    json!({
        "name": "sqlite_schema",
        "ready": ready,
        "message": format!("{table_count} tables found (expected {expected}+)"),
        "criticality": "advisory",
        "duration_ms": start.elapsed().as_millis() as u64
    })
}

/// Check PG pool status — mode-aware, skip in standalone.
async fn check_pg_pool() -> Value {
    let start = std::time::Instant::now();
    let mode = crate::pg::read_mode();
    if mode == "standalone" {
        return json!({
            "name": "pg_pool",
            "ready": true,
            "message": "not configured (standalone mode)",
            "criticality": "advisory",
            "duration_ms": start.elapsed().as_millis() as u64
        });
    }

    let status = crate::pg::pool_status().await;
    let pg_ready = status["postgresql"]["ready"].as_bool().unwrap_or(false);
    let configured = status["postgresql"]["configured"]
        .as_bool()
        .unwrap_or(false);
    let msg = if !configured {
        "not configured".to_string()
    } else if pg_ready {
        "pool connected".to_string()
    } else {
        "pool not ready (sync may be delayed)".to_string()
    };

    json!({
        "name": "pg_pool",
        "ready": !configured || pg_ready,
        "message": msg,
        "criticality": "advisory",
        "duration_ms": start.elapsed().as_millis() as u64
    })
}

/// Check disk usage under ~/.masday/ — advisory, warns if >500MB.
fn check_disk_space() -> Value {
    let start = std::time::Instant::now();
    let home = match home::home_dir() {
        Some(h) => h,
        None => {
            return json!({
                "name": "disk_space",
                "ready": true,
                "message": "cannot determine home directory",
                "criticality": "advisory",
                "duration_ms": 0
            })
        }
    };
    let masday_dir = home.join(".masday");
    if !masday_dir.exists() {
        return json!({
            "name": "disk_space",
            "ready": true,
            "message": "~/.masday/ not found (fresh install)",
            "criticality": "advisory",
            "duration_ms": start.elapsed().as_millis() as u64
        });
    }

    let size_bytes: u64 = std::fs::read_dir(&masday_dir)
        .map(|entries| {
            entries
                .filter_map(|e| e.ok())
                .filter_map(|e| e.metadata().ok().map(|m| m.len()))
                .sum()
        })
        .unwrap_or(0);

    let size_mb = size_bytes as f64 / 1024.0 / 1024.0;
    let threshold_mb = 500.0;
    let ready = size_mb < threshold_mb;

    json!({
        "name": "disk_space",
        "ready": ready,
        "message": format!("~{:.1} MB in ~/.masday/{}", size_mb, if !ready { " (exceeds threshold)" } else { "" }),
        "criticality": "advisory",
        "duration_ms": start.elapsed().as_millis() as u64
    })
}

/// Check API health endpoint if api_url is configured — advisory only.
async fn check_api_health() -> Value {
    let start = std::time::Instant::now();
    let mode = crate::pg::read_mode();
    if mode == "standalone" {
        return json!({
            "name": "api_health",
            "ready": true,
            "message": "not checked (standalone mode)",
            "criticality": "advisory",
            "duration_ms": 0
        });
    }

    let api_url = match crate::pg::read_api_url() {
        Some(u) => u,
        None => {
            return json!({
                "name": "api_health",
                "ready": true,
                "message": "api_url not configured",
                "criticality": "advisory",
                "duration_ms": 0
            })
        }
    };

    let health_url = format!("{}/api/health", api_url.trim_end_matches('/'));
    match reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(2))
        .build()
    {
        Ok(client) => match client.get(&health_url).send().await {
            Ok(resp) if resp.status().is_success() => json!({
                "name": "api_health",
                "ready": true,
                "message": format!("API healthy ({})", api_url),
                "criticality": "advisory",
                "duration_ms": start.elapsed().as_millis() as u64
            }),
            Ok(resp) => json!({
                "name": "api_health",
                "ready": false,
                "message": format!("API returned HTTP {}", resp.status()),
                "criticality": "advisory",
                "duration_ms": start.elapsed().as_millis() as u64
            }),
            Err(e) => json!({
                "name": "api_health",
                "ready": false,
                "message": format!("API unreachable: {e}"),
                "criticality": "advisory",
                "duration_ms": start.elapsed().as_millis() as u64
            }),
        },
        Err(e) => json!({
            "name": "api_health",
            "ready": false,
            "message": format!("HTTP client error: {e}"),
            "criticality": "advisory",
            "duration_ms": start.elapsed().as_millis() as u64
        }),
    }
}

/// Run all system readiness checks and return aggregated result.
///
/// Returns JSON:
/// ```json
/// {
///   "ready": bool,
///   "mode": "standalone" | "local" | "remote",
///   "checks": [{ "name": str, "ready": bool, "message": str, "criticality": str, "duration_ms": int }],
///   "warnings": [str],
///   "errors": [str],
///   "total_duration_ms": int
/// }
/// ```
pub async fn system_readiness_check() -> Value {
    let start = std::time::Instant::now();
    let mode = crate::pg::read_mode();

    // Run all checks — sync ones first, then async
    let sqlite = check_sqlite();
    let sqlite_integrity = check_sqlite_integrity();
    let sqlite_schema = check_sqlite_schema();
    let disk = check_disk_space();
    let pg = check_pg_pool().await;
    let api = check_api_health().await;

    let checks = vec![sqlite, sqlite_integrity, sqlite_schema, pg, disk, api];

    let mut warnings: Vec<String> = Vec::new();
    let mut errors: Vec<String> = Vec::new();
    let mut all_critical_ready = true;

    for check in &checks {
        let ready = check["ready"].as_bool().unwrap_or(false);
        let criticality = check["criticality"].as_str().unwrap_or("advisory");
        let message = check["message"].as_str().unwrap_or("unknown");

        if !ready {
            if criticality == "critical" {
                all_critical_ready = false;
                errors.push(message.to_string());
                error!("Readiness CRITICAL: {} — {}", check["name"], message);
            } else {
                warnings.push(message.to_string());
                warn!("Readiness advisory: {} — {}", check["name"], message);
            }
        } else {
            info!("Readiness OK: {} — {}", check["name"], message);
        }
    }

    json!({
        "ready": all_critical_ready,
        "mode": mode,
        "checks": checks,
        "warnings": warnings,
        "errors": errors,
        "total_duration_ms": start.elapsed().as_millis() as u64
    })
}

/// Run lightweight readiness check for lifecycle gating.
/// Only runs critical checks (SQLite), skips advisory ones.
async fn readiness_gate() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let sqlite = check_sqlite();
    if !sqlite["ready"].as_bool().unwrap_or(false) {
        return Err(err(format!("SQLite not ready: {}", sqlite["message"])));
    }

    let integrity = check_sqlite_integrity();
    if !integrity["ready"].as_bool().unwrap_or(false) {
        return Err(err(format!("SQLite integrity: {}", integrity["message"])));
    }

    Ok(())
}

// ============================================================================
// Workflow Tools (22)
// ============================================================================

pub async fn workflow_create(
    args: Value,
) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    // Gate: system readiness check (SQLite connectivity + integrity)
    readiness_gate().await?;

    let (id, name, status, pg_info) = {
        let conn = crate::sqlite::conn();
        let id = new_id();
        let name = args["name"].as_str().ok_or_else(|| err("missing name"))?;
        let description = args.get("description").and_then(|v| v.as_str());
        let status = "INIT";
        let project_path = args
            .get("project_path")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .or_else(|| {
                std::env::current_dir()
                    .ok()
                    .map(|p| p.to_string_lossy().to_string())
            });
        let t = now();
        let meta = args.get("metadata").cloned().unwrap_or(json!({}));

        conn.execute(
            "INSERT INTO workflows (id, name, description, status, project_path, metadata, created_at, updated_at) VALUES (?1,?2,?3,?4,?5,?6,?7,?8)",
            params![id, name, description, status, project_path, meta.to_string(), &t, &t],
        ).map_err(|e| err(e))?;

        (
            id.clone(),
            name.to_string(),
            status.to_string(),
            (
                id,
                name.to_string(),
                description.map(String::from),
                project_path.clone(),
                meta.to_string(),
            ),
        )
    }; // conn dropped

    // Fire-and-forget PG sync — non-blocking
    tokio::spawn(async move {
        crate::direct_pg::workflow_create(
            &pg_info.0,
            &pg_info.1,
            pg_info.2.as_deref(),
            pg_info.3.as_deref(),
            &pg_info.4,
        )
        .await;
    });

    Ok(json!({"id": id, "name": name, "status": status}))
}

pub async fn workflow_execute(
    args: Value,
) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    let (id_str, final_status) = {
        // Extract ID before touching SQLite — fail fast on bad args
        let id = args
            .get("id")
            .or_else(|| args.get("workflow_id"))
            .and_then(|v| v.as_str())
            .ok_or_else(|| err("missing id"))?;

        let conn = crate::sqlite::conn();

        // Get current workflow status
        let current_status: String = conn
            .query_row(
                "SELECT status FROM workflows WHERE id=?1",
                params![id],
                |row| row.get(0),
            )
            .map_err(|_| err(format!("workflow not found: {}", id)))?;

        // Already at or past EXECUTE — idempotent return (matches service layer)
        if matches!(
            current_status.as_str(),
            "EXECUTE" | "VERIFY" | "FIX" | "DONE"
        ) {
            return Ok(json!({"id": id, "status": current_status}));
        }

        let t = now();
        match current_status.as_str() {
            "INIT" => {
                // Only advance to ANALYZE, not all the way to EXECUTE
                conn.execute(
                    "UPDATE workflows SET status='ANALYZE', updated_at=?1 WHERE id=?2",
                    params![&t, id],
                )
                .map_err(|e| err(e))?;

                info!("Workflow {} transitioned to ANALYZE (from INIT)", id);
                (id.to_string(), "ANALYZE".to_string())
            }
            "ANALYZE" => {
                // Validate analysis artifacts exist before advancing to PLAN
                // Check both context_documents and memories
                let doc_count: i64 = conn
                    .query_row(
                        "SELECT COUNT(*) FROM context_documents WHERE workflow_id=?1",
                        params![id],
                        |r| r.get(0),
                    )
                    .unwrap_or(0);

                let mem_count: i64 = conn
                    .query_row(
                        "SELECT COUNT(*) FROM memories WHERE workflow_id=?1",
                        params![id],
                        |r| r.get(0),
                    )
                    .unwrap_or(0);

                if doc_count == 0 && mem_count == 0 {
                    return Err(err(
                        "Cannot advance to PLAN: no analysis artifacts found. Run analysis first.",
                    ));
                }

                conn.execute(
                    "UPDATE workflows SET status='PLAN', updated_at=?1 WHERE id=?2",
                    params![&t, id],
                )
                .map_err(|e| err(e))?;

                info!("Workflow {} transitioned to PLAN (from ANALYZE)", id);

                // Auto-store analysis summary as context document + memory
                {
                    let summary_json = serde_json::json!({
                        "transition": "ANALYZE -> PLAN",
                        "workflow_id": id,
                        "artifacts_found": (doc_count + mem_count),
                    })
                    .to_string();

                    auto_store_context_document_sqlite(
                        &conn,
                        id,
                        "analysis",
                        "Analysis Summary",
                        &summary_json,
                    );
                    auto_store_memory_sqlite(
                        &conn,
                        id,
                        None,
                        "experience",
                        &format!(
                            "Workflow transitioned ANALYZE -> PLAN ({} artifacts)",
                            doc_count + mem_count
                        ),
                        &summary_json,
                        0.6,
                        &["auto", "analyze-to-plan"],
                    );
                }

                (id.to_string(), "PLAN".to_string())
            }
            "PLAN" => {
                // Validate plan exists with tasks before advancing to EXECUTE
                let plan_count: i64 = conn
                    .query_row(
                        "SELECT COUNT(*) FROM plans WHERE workflow_id=?1 AND status='ACTIVE'",
                        params![id],
                        |r| r.get(0),
                    )
                    .map_err(|e| err(e))?;

                if plan_count == 0 {
                    return Err(err(
                        "Cannot advance to EXECUTE: no plan found. Create a plan first.",
                    ));
                }

                let task_count: i64 = conn
                    .query_row(
                        "SELECT COUNT(*) FROM tasks WHERE workflow_id=?1",
                        params![id],
                        |r| r.get(0),
                    )
                    .map_err(|e| err(e))?;

                if task_count == 0 {
                    return Err(err(
                        "Cannot advance to EXECUTE: plan has no tasks. Add tasks to the plan first."
                    ));
                }

                conn.execute(
                    "UPDATE workflows SET status='EXECUTE', updated_at=?1 WHERE id=?2",
                    params![&t, id],
                )
                .map_err(|e| err(e))?;

                info!("Workflow {} transitioned to EXECUTE (from PLAN)", id);
                (id.to_string(), "EXECUTE".to_string())
            }
            "PAUSED" => {
                conn.execute(
                    "UPDATE workflows SET status='EXECUTE', updated_at=?1 WHERE id=?2",
                    params![&t, id],
                )
                .map_err(|e| err(e))?;

                info!("Workflow {} transitioned to EXECUTE (from PAUSED)", id);
                (id.to_string(), "EXECUTE".to_string())
            }
            other => {
                return Err(err(format!("Cannot execute workflow in state {}", other)));
            }
        }
    }; // conn dropped

    // Fire-and-forget PG sync — non-blocking
    let pg_id = id_str.clone();
    let pg_status = final_status.clone();
    tokio::spawn(async move {
        crate::direct_pg::workflow_status(&pg_id, &pg_status).await;
    });

    Ok(json!({"id": id_str, "status": final_status}))
}

pub async fn workflow_get_status(
    args: Value,
) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    let conn = crate::sqlite::conn();
    let id = args
        .get("id")
        .or_else(|| args.get("workflow_id"))
        .and_then(|v| v.as_str())
        .ok_or_else(|| err("missing id"))?;

    let (name, status): (String, String) = conn
        .query_row(
            "SELECT name, status FROM workflows WHERE id=?1",
            params![id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .map_err(err)?;

    Ok(json!({"id": id, "status": status, "name": name}))
}

pub async fn workflow_get(args: Value) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    let conn = crate::sqlite::conn();
    let id = args["workflow_id"]
        .as_str()
        .ok_or_else(|| err("missing workflow_id"))?;

    let wf = conn.query_row(
        "SELECT id, name, description, status, project_path, metadata, created_at, updated_at FROM workflows WHERE id=?1",
        params![id],
        |row| Ok(json!({
            "id": row.get::<_, String>(0)?,
            "name": row.get::<_, String>(1)?,
            "description": row.get::<_, Option<String>>(2)?,
            "status": row.get::<_, String>(3)?,
            "projectPath": row.get::<_, Option<String>>(4)?,
            "metadata": json_col(row, 5),
            "createdAt": row.get::<_, String>(6)?,
            "updatedAt": row.get::<_, String>(7)?,
        })),
    ).map_err(|e| err(e))?;

    Ok(wf)
}

pub async fn workflow_list(args: Value) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    let conn = crate::sqlite::conn();
    let page = args["page"].as_u64().unwrap_or(1);
    let page_size = args["page_size"].as_u64().unwrap_or(50);
    let limit = page_size as i64;
    let offset = ((page - 1) * page_size) as i64;
    let project_path = args
        .get("project_path")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .or_else(|| {
            std::env::current_dir()
                .ok()
                .map(|p| p.to_string_lossy().to_string())
        });

    let workflows = if let Some(pp) = project_path {
        let mut stmt = conn.prepare(
            "SELECT id, name, status, created_at FROM workflows WHERE project_path = ?1 ORDER BY created_at DESC LIMIT ?2 OFFSET ?3"
        ).map_err(err)?;
        let rows = stmt
            .query_map(params![pp, limit, offset], |row| {
                Ok(json!({
                    "id": row.get::<_, String>(0)?,
                    "name": row.get::<_, String>(1)?,
                    "status": row.get::<_, String>(2)?,
                    "createdAt": row.get::<_, String>(3)?,
                }))
            })
            .map_err(err)?;
        collect_rows(rows)
    } else {
        let mut stmt = conn.prepare(
            "SELECT id, name, status, created_at FROM workflows ORDER BY created_at DESC LIMIT ?1 OFFSET ?2"
        ).map_err(err)?;
        let rows = stmt
            .query_map(params![limit, offset], |row| {
                Ok(json!({
                    "id": row.get::<_, String>(0)?,
                    "name": row.get::<_, String>(1)?,
                    "status": row.get::<_, String>(2)?,
                    "createdAt": row.get::<_, String>(3)?,
                }))
            })
            .map_err(err)?;
        collect_rows(rows)
    };

    Ok(json!({"workflows": workflows, "page": page, "page_size": page_size}))
}

pub async fn workflow_get_active(
    args: Value,
) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    let conn = crate::sqlite::conn();
    let project_path = args
        .get("project_path")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .or_else(|| {
            std::env::current_dir()
                .ok()
                .map(|p| p.to_string_lossy().to_string())
        });

    let workflows = if let Some(pp) = project_path {
        let mut stmt = conn.prepare(
            "SELECT id, name, status FROM workflows WHERE status NOT IN ('DONE','FAILED') AND project_path = ?1 ORDER BY created_at DESC"
        ).map_err(err)?;
        let rows = stmt.query_map(params![pp], |row| {
            Ok(json!({"id": row.get::<_, String>(0)?, "name": row.get::<_, String>(1)?, "status": row.get::<_, String>(2)?}))
        }).map_err(|e| err(e))?;
        collect_rows(rows)
    } else {
        let mut stmt = conn.prepare(
            "SELECT id, name, status FROM workflows WHERE status NOT IN ('DONE','FAILED') ORDER BY created_at DESC"
        ).map_err(err)?;
        let rows = stmt.query_map([], |row| {
            Ok(json!({"id": row.get::<_, String>(0)?, "name": row.get::<_, String>(1)?, "status": row.get::<_, String>(2)?}))
        }).map_err(|e| err(e))?;
        collect_rows(rows)
    };

    Ok(json!({"workflows": workflows}))
}

pub async fn workflow_delete(
    args: Value,
) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    let conn = crate::sqlite::conn();
    let id = args["workflow_id"]
        .as_str()
        .ok_or_else(|| err("missing workflow_id"))?;

    conn.execute("DELETE FROM workflows WHERE id=?1", params![id])
        .map_err(err)?;
    Ok(json!({"deleted": id}))
}

pub async fn workflow_add_task(
    args: Value,
) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    let conn = crate::sqlite::conn();
    let id = new_id();
    let workflow_id = args["workflow_id"]
        .as_str()
        .ok_or_else(|| err("missing workflow_id"))?;
    let plan_id = args["plan_id"].as_str().unwrap_or("default");
    let title = args["name"].as_str().ok_or_else(|| err("missing name"))?;
    let owner_agent = args.get("agent").and_then(|v| v.as_str());
    let deps = args.get("dependencies").map(|v| v.to_string());
    let t = now();

    // VALIDATION 1: Check plan_id is not empty (unless it's the default "default" string)
    if plan_id.is_empty() {
        return Err(err("plan_id cannot be empty"));
    }

    // VALIDATION 2: Check if plan exists
    let plan_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM plans WHERE id=?1",
            params![plan_id],
            |r| r.get(0),
        )
        .map_err(err)?;

    // VALIDATION 3: If plan exists, verify it belongs to the workflow
    let mut resolved_plan_id = plan_id.to_string();
    if plan_count > 0 {
        let plan_workflow_id: String = conn
            .query_row(
                "SELECT workflow_id FROM plans WHERE id=?1",
                params![plan_id],
                |r| r.get(0),
            )
            .map_err(err)?;

        if plan_workflow_id != workflow_id {
            // Default plan belongs to another workflow — look up this workflow's active plan instead
            let active_plan: Option<String> = conn
                .query_row(
                    "SELECT id FROM plans WHERE workflow_id=?1 AND status='ACTIVE' ORDER BY version DESC LIMIT 1",
                    params![workflow_id],
                    |r| r.get(0),
                )
                .ok();
            match active_plan {
                Some(pid) => resolved_plan_id = pid,
                None => {
                    return Err(err(format!(
                        "Plan {} does not belong to workflow {} and no ACTIVE plan found for this workflow",
                        plan_id, workflow_id
                    )));
                }
            }
        }
    } else {
        // VALIDATION 4: If plan doesn't exist, check workflow is in INIT state
        // Only auto-create plan for new workflows in INIT state
        let workflow_status: String = conn
            .query_row(
                "SELECT status FROM workflows WHERE id=?1",
                params![workflow_id],
                |r| r.get(0),
            )
            .map_err(|_| err(format!("Workflow {} not found", workflow_id)))?;

        if workflow_status != "INIT" {
            return Err(err(format!(
                "Cannot create plan for workflow in {} state. Only INIT state allows auto-plan creation.",
                workflow_status
            )));
        }

        // Auto-create plan for INIT workflows
        conn.execute(
            "INSERT INTO plans (id, workflow_id, version, status, summary, content, created_by_agent, created_at) VALUES (?1,?2,1,'ACTIVE','Auto-created plan','{}','system',?3)",
            params![plan_id, workflow_id, &t],
        ).map_err(|e| err(e))?;
    }

    // VALIDATION 5: Check workflow state allows task creation
    let workflow_status: String = conn
        .query_row(
            "SELECT status FROM workflows WHERE id=?1",
            params![workflow_id],
            |r| r.get(0),
        )
        .map_err(|_| err(format!("Workflow {} not found", workflow_id)))?;

    // Only allow task creation in INIT, PLAN, or EXECUTE states
    if !matches!(workflow_status.as_str(), "INIT" | "PLAN" | "EXECUTE") {
        return Err(err(format!(
            "Cannot add tasks to workflow in {} state. Allowed states: INIT, PLAN, EXECUTE.",
            workflow_status
        )));
    }

    conn.execute(
        "INSERT INTO tasks (id, workflow_id, plan_id, title, status, owner_agent, dependencies, created_at, updated_at) VALUES (?1,?2,?3,?4,'PENDING',?5,?6,?7,?8)",
        params![id, workflow_id, resolved_plan_id, title, owner_agent, deps, &t, &t],
    ).map_err(|e| err(e))?;

    Ok(json!({"id": id, "title": title, "status": "PENDING"}))
}

pub async fn workflow_start_task(
    args: Value,
) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    // Gate 1: system readiness check (SQLite connectivity + integrity)
    readiness_gate().await?;

    let conn = crate::sqlite::conn();
    let wf_id = args["workflow_id"]
        .as_str()
        .ok_or_else(|| err("missing workflow_id"))?;
    let task_id = args["task_id"]
        .as_str()
        .ok_or_else(|| err("missing task_id"))?;

    // Gate 2: validate task is in a startable state
    let task_status: String = conn
        .query_row(
            "SELECT status FROM tasks WHERE id=?1 AND workflow_id=?2",
            params![task_id, wf_id],
            |row| row.get(0),
        )
        .map_err(|e| err(format!("Task not found: {e}")))?;

    if task_status != "PENDING" {
        return Err(err(format!(
            "Task {task_id} is {task_status}, cannot start (must be PENDING)"
        )));
    }

    // Gate 3: validate workflow is in EXECUTE state
    let wf_status: String = conn
        .query_row(
            "SELECT status FROM workflows WHERE id=?1",
            params![wf_id],
            |row| row.get(0),
        )
        .map_err(|e| err(e))?;

    if wf_status != "EXECUTE" && wf_status != "ANALYZE" && wf_status != "PLAN" {
        return Err(err(format!(
            "Workflow is {wf_status}, tasks can only start in EXECUTE/ANALYZE/PLAN state"
        )));
    }

    // Gate 4: check dependency completion
    let deps_text: Option<String> = conn
        .query_row(
            "SELECT dependencies FROM tasks WHERE id=?1",
            params![task_id],
            |row| row.get(0),
        )
        .ok()
        .flatten();

    if let Some(deps) = deps_text {
        let dep_ids: Vec<&str> = serde_json::from_str::<Vec<&str>>(&deps).unwrap_or_default();
        for dep_id in dep_ids {
            let dep_status: String = conn
                .query_row(
                    "SELECT status FROM tasks WHERE id=?1",
                    params![dep_id],
                    |row| row.get(0),
                )
                .unwrap_or_default();
            if dep_status != "DONE" {
                return Err(err(format!(
                    "Dependency {dep_id} not complete (status: {dep_status})"
                )));
            }
        }
    }

    let t = now();
    conn.execute(
        "UPDATE tasks SET status='RUNNING', started_at=?1, updated_at=?2 WHERE id=?3 AND workflow_id=?4",
        params![&t, &t, task_id, wf_id],
    ).map_err(|e| err(e))?;

    Ok(json!({"status": "RUNNING", "task_id": task_id}))
}

pub async fn workflow_complete_task(
    args: Value,
) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    let conn = crate::sqlite::conn();
    let wf_id = args["workflow_id"]
        .as_str()
        .ok_or_else(|| err("missing workflow_id"))?;
    let task_id = args["task_id"]
        .as_str()
        .ok_or_else(|| err("missing task_id"))?;
    let result: Option<String> = args.get("result").map(|v| v.to_string());
    let t = now();

    conn.execute("UPDATE tasks SET status='DONE', result=?1, completed_at=?2, updated_at=?3 WHERE id=?4 AND workflow_id=?5",
        params![result, &t, &t, task_id, wf_id]).map_err(|e| err(e))?;

    // Auto-store task result as experience memory (best-effort)
    {
        let task_title: String = conn
            .query_row(
                "SELECT title FROM tasks WHERE id=?1",
                params![task_id],
                |r| r.get(0),
            )
            .unwrap_or_else(|_| "Unknown task".to_string());
        let result_str = result.as_deref().unwrap_or("null");
        auto_store_memory_sqlite(
            &conn,
            wf_id,
            Some(task_id),
            "experience",
            &format!("Task completed: {}", task_title),
            result_str,
            0.6,
            &["auto", "task-complete"],
        );
    }

    // Auto-transition workflow if all tasks done
    let pending: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM tasks WHERE workflow_id=?1 AND status NOT IN ('DONE','FAILED')",
            params![wf_id],
            |r| r.get(0),
        )
        .map_err(err)?;

    if pending == 0 {
        conn.execute("UPDATE workflows SET status='DONE', updated_at=?1 WHERE id=?2 AND status NOT IN ('DONE','FAILED')",
            params![&t, wf_id]).map_err(|e| err(e))?;

        // Auto-store workflow completion summary (best-effort)
        {
            let wf_name: String = conn
                .query_row(
                    "SELECT name FROM workflows WHERE id=?1",
                    params![wf_id],
                    |r| r.get(0),
                )
                .unwrap_or_else(|_| "Unknown".to_string());
            let task_count: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM tasks WHERE workflow_id=?1",
                    params![wf_id],
                    |r| r.get(0),
                )
                .unwrap_or(0);

            auto_store_memory_sqlite(
                &conn,
                wf_id,
                None,
                "experience",
                &format!("Workflow '{}' completed ({} tasks)", wf_name, task_count),
                &format!(
                    "{{\"workflow_id\":\"{}\",\"workflow_name\":\"{}\",\"final_status\":\"DONE\",\"task_count\":{}}}",
                    wf_id, wf_name, task_count
                ),
                0.8,
                &["auto", "workflow-complete"],
            );
        }
    }

    Ok(json!({"status": "DONE", "task_id": task_id}))
}

pub async fn workflow_save_progress(
    args: Value,
) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    let conn = crate::sqlite::conn();
    let id = new_id();
    let wf_id = args["workflow_id"]
        .as_str()
        .ok_or_else(|| err("missing workflow_id"))?;
    let task_id = args["task_id"]
        .as_str()
        .ok_or_else(|| err("missing task_id"))?;
    let agent = args["agent_name"]
        .as_str()
        .or_else(|| args["agent"].as_str())
        .ok_or_else(|| err("missing agent_name"))?;
    let note = args["progress_note"]
        .as_str()
        .or_else(|| args["note"].as_str())
        .ok_or_else(|| err("missing progress_note"))?;
    let evidence = args.get("evidence").map(|v| v.to_string());
    let t = now();

    conn.execute(
        "INSERT INTO task_progress_logs (id, workflow_id, task_id, agent_name, progress_note, evidence, created_at) VALUES (?1,?2,?3,?4,?5,?6,?7)",
        params![id, wf_id, task_id, agent, note, evidence, &t],
    ).map_err(|e| err(e))?;

    Ok(json!({"saved": true, "task_id": task_id}))
}

pub async fn workflow_create_plan(
    args: Value,
) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    let conn = crate::sqlite::conn();
    let id = new_id();
    let workflow_id = args["workflow_id"]
        .as_str()
        .ok_or_else(|| err("missing workflow_id"))?;
    let content = args
        .get("phases")
        .or_else(|| args.get("plan"))
        .ok_or_else(|| err("missing plan"))?
        .to_string();
    let t = now();

    // Get next version
    let version: i64 = conn
        .query_row(
            "SELECT COALESCE(MAX(version),0)+1 FROM plans WHERE workflow_id=?1",
            params![workflow_id],
            |r| r.get(0),
        )
        .unwrap_or(1);

    conn.execute(
        "INSERT INTO plans (id, workflow_id, version, status, summary, content, created_by_agent, created_at) VALUES (?1,?2,?3,'ACTIVE',?4,?5,'masday-mcp',?6)",
        params![id, workflow_id, version, &format!("Plan v{}", version), &content, &t],
    ).map_err(|e| err(e))?;

    Ok(json!({"id": id, "workflow_id": workflow_id, "status": "ACTIVE", "version": version}))
}

pub async fn workflow_get_plan(
    args: Value,
) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    let conn = crate::sqlite::conn();
    let workflow_id = args["workflow_id"]
        .as_str()
        .ok_or_else(|| err("missing workflow_id"))?;

    let plan = conn.query_row(
        "SELECT id, workflow_id, version, status, summary, content, created_by_agent, created_at FROM plans WHERE workflow_id=?1 ORDER BY version DESC LIMIT 1",
        params![workflow_id],
        |row| Ok(json!({
            "id": row.get::<_, String>(0)?,
            "workflowId": row.get::<_, String>(1)?,
            "version": row.get::<_, i64>(2)?,
            "status": row.get::<_, String>(3)?,
            "summary": row.get::<_, String>(4)?,
            "content": json_col(row, 5),
            "createdByAgent": row.get::<_, String>(6)?,
            "createdAt": row.get::<_, String>(7)?,
        })),
    ).map_err(|e| err(e))?;

    Ok(plan)
}

pub async fn workflow_list_tasks(
    args: Value,
) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    let conn = crate::sqlite::conn();
    let workflow_id = args["workflow_id"]
        .as_str()
        .ok_or_else(|| err("missing workflow_id"))?;

    let mut stmt = conn.prepare(
        "SELECT id, title, status, owner_agent, priority, progress_percent, created_at, updated_at FROM tasks WHERE workflow_id=?1 ORDER BY created_at"
    ).map_err(err)?;

    let rows = stmt
        .query_map(params![workflow_id], |row| {
            Ok(json!({
                "id": row.get::<_, String>(0)?,
                "title": row.get::<_, String>(1)?,
                "status": row.get::<_, String>(2)?,
                "ownerAgent": row.get::<_, Option<String>>(3)?,
                "priority": row.get::<_, Option<String>>(4)?,
                "progressPercent": row.get::<_, Option<i64>>(5)?,
                "createdAt": row.get::<_, String>(6)?,
                "updatedAt": row.get::<_, String>(7)?,
            }))
        })
        .map_err(err)?;

    let tasks: Vec<Value> = collect_rows(rows);

    // Verify the workflow exists
    let wf_exists: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM workflows WHERE id=?1",
            params![workflow_id],
            |r| r.get(0),
        )
        .unwrap_or(0);
    if wf_exists == 0 {
        return Err(format!("Workflow not found: {}", workflow_id).into());
    }

    Ok(json!({"tasks": tasks}))
}

pub async fn workflow_create_parallel_branches(
    args: Value,
) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    let conn = crate::sqlite::conn();
    let wf_id = args["workflow_id"]
        .as_str()
        .ok_or_else(|| err("missing workflow_id"))?;
    let branches_arr = match args["branches"] {
        Value::Array(ref arr) => arr.clone(),
        Value::String(ref s) => serde_json::from_str(s).unwrap_or_default(),
        _ => return Err(err("missing branches array")),
    };
    if branches_arr.is_empty() {
        return Err(err("branches array is empty — provide at least one branch object with task_id, branch_key, role"));
    }
    let t = now();
    let mut created = Vec::new();

    for b in branches_arr {
        let id = new_id();
        let task_id = match b["task_id"].as_str() {
            Some(t) if !t.is_empty() => t,
            _ => {
                warn!("workflow_create_parallel_branches: branch missing task_id, skipping");
                continue;
            }
        };
        let branch_key = b["branch_key"].as_str().unwrap_or("default");
        let role = b["role"].as_str().unwrap_or("executor");
        let input = b.get("input").cloned().unwrap_or(json!({})).to_string();

        conn.execute(
            "INSERT INTO parallel_branches (id, workflow_id, task_id, branch_key, role, status, input, created_at, updated_at) VALUES (?1,?2,?3,?4,?5,'ACTIVE',?6,?7,?8)",
            params![id, wf_id, task_id, branch_key, role, &input, &t, &t],
        ).map_err(|e| err(e))?;

        created.push(json!({"id": id, "branch_key": branch_key, "status": "ACTIVE"}));
    }

    Ok(json!({"branches": created}))
}

pub async fn workflow_complete_parallel_branch(
    args: Value,
) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    let conn = crate::sqlite::conn();
    let branch_id = args["branch_id"]
        .as_str()
        .or_else(|| args["branch_key"].as_str())
        .ok_or_else(|| err("missing branch_id"))?;
    let output = args.get("output").cloned().unwrap_or(json!({})).to_string();
    let t = now();

    conn.execute(
        "UPDATE parallel_branches SET status='DONE', output=?1, updated_at=?2 WHERE id=?3",
        params![&output, &t, branch_id],
    )
    .map_err(|e| err(e))?;

    Ok(json!({"completed": branch_id}))
}

pub async fn workflow_list_parallel_branches(
    args: Value,
) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    let conn = crate::sqlite::conn();
    let workflow_id = args["workflow_id"]
        .as_str()
        .ok_or_else(|| err("missing workflow_id"))?;

    let mut stmt = conn.prepare(
        "SELECT id, branch_key, role, status, input, output FROM parallel_branches WHERE workflow_id=?1"
    ).map_err(err)?;

    let rows = stmt
        .query_map(params![workflow_id], |row| {
            Ok(json!({
                "id": row.get::<_, String>(0)?,
                "branchKey": row.get::<_, String>(1)?,
                "role": row.get::<_, String>(2)?,
                "status": row.get::<_, String>(3)?,
                "input": json_col(row, 4),
                "output": opt_json(row, 5),
            }))
        })
        .map_err(err)?;

    let branches: Vec<Value> = collect_rows(rows);
    Ok(json!({"branches": branches}))
}

pub async fn workflow_mark_synthesis_ready(
    args: Value,
) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    let conn = crate::sqlite::conn();
    let wf_id = args
        .get("workflow_id")
        .or_else(|| args.get("session_key"))
        .and_then(|v| v.as_str())
        .ok_or_else(|| err("missing workflow_id"))?;
    let t = now();

    conn.execute(
        "UPDATE workflows SET status='VERIFY', updated_at=?1 WHERE id=?2",
        params![&t, wf_id],
    )
    .map_err(|e| err(e))?;

    Ok(json!({"status": "VERIFY", "workflow_id": wf_id}))
}

pub async fn workflow_mark_verification_ready(
    args: Value,
) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    let conn = crate::sqlite::conn();
    let wf_id = args
        .get("workflow_id")
        .or_else(|| args.get("session_key"))
        .and_then(|v| v.as_str())
        .ok_or_else(|| err("missing workflow_id"))?;
    let t = now();

    conn.execute(
        "UPDATE workflows SET status='DONE', updated_at=?1 WHERE id=?2",
        params![&t, wf_id],
    )
    .map_err(|e| err(e))?;

    Ok(json!({"status": "DONE", "workflow_id": wf_id}))
}

pub async fn workflow_set_execution_mode(
    args: Value,
) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    let conn = crate::sqlite::conn();
    let wf_id = args
        .get("workflow_id")
        .or_else(|| args.get("session_key"))
        .and_then(|v| v.as_str())
        .ok_or_else(|| err("missing workflow_id"))?;
    let mode = args["mode"].as_str().ok_or_else(|| err("missing mode"))?;
    let t = now();

    // Store execution_mode in metadata JSON
    let meta: String = conn
        .query_row(
            "SELECT metadata FROM workflows WHERE id=?1",
            params![wf_id],
            |r| r.get(0),
        )
        .map_err(err)?;
    let mut meta: Value = serde_json::from_str(&meta).unwrap_or_else(|e| {
        warn!(
            "workflow_set_execution_mode: corrupt metadata for {}: {e}",
            wf_id
        );
        json!({})
    });
    meta["execution_mode"] = json!(mode);
    conn.execute(
        "UPDATE workflows SET metadata=?1, updated_at=?2 WHERE id=?3",
        params![meta.to_string(), &t, wf_id],
    )
    .map_err(|e| err(e))?;

    Ok(json!({"updated": true, "mode": mode}))
}

pub async fn workflow_resume_suggestion(
    args: Value,
) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    let conn = crate::sqlite::conn();
    let wf_id = args["workflow_id"]
        .as_str()
        .ok_or_else(|| err("missing workflow_id"))?;

    let (name, status): (String, String) = conn
        .query_row(
            "SELECT name, status FROM workflows WHERE id=?1",
            params![wf_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .map_err(err)?;

    Ok(json!({"workflow_id": wf_id, "current_status": status,
        "suggestion": format!("Resume workflow '{}' from status '{}'", name, status)}))
}

pub async fn workflow_get_current_task(
    args: Value,
) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    let conn = crate::sqlite::conn();
    let wf_id = args["workflow_id"]
        .as_str()
        .ok_or_else(|| err("missing workflow_id"))?;

    let task = match conn.query_row(
        "SELECT id, title, status FROM tasks WHERE workflow_id=?1 AND status='RUNNING' LIMIT 1",
        params![wf_id],
        |row| Ok(json!({"id": row.get::<_, String>(0)?, "title": row.get::<_, String>(1)?, "status": row.get::<_, String>(2)?})),
    ) {
        Ok(t) => Some(t),
        Err(rusqlite::Error::QueryReturnedNoRows) => None,
        Err(e) => return Err(err(e)),
    };

    match task {
        Some(t) => Ok(json!({"task": t})),
        None => Err(format!("No running task found for workflow: {}", wf_id).into()),
    }
}

pub async fn workflow_ping(
    _args: Value,
) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    Ok(json!({"status": "pong"}))
}

// ============================================================================
// Memory Tools (13)
// ============================================================================

pub async fn memory_store(args: Value) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    let (id, pg_args) = {
        let conn = crate::sqlite::conn();
        let id = new_id();
        let memory_type = args["memory_type"]
            .as_str()
            .or_else(|| args["type"].as_str())
            .ok_or_else(|| err("missing type"))?;
        let summary = args["summary"].as_str().unwrap_or("");
        let content = args["content"]
            .as_str()
            .ok_or_else(|| err("missing content"))?;
        let created_by = args["created_by_agent"]
            .as_str()
            .or_else(|| args["created_by"].as_str())
            .unwrap_or("masday-mcp");
        let importance = args["importance_score"]
            .as_f64()
            .or_else(|| args["importance"].as_f64())
            .unwrap_or(0.5);
        let tags: String = args
            .get("tags")
            .and_then(|v| v.as_array())
            .map(|arr| {
                serde_json::Value::Array(
                    arr.iter()
                        .filter_map(|v| v.as_str().map(String::from))
                        .map(|s| json!(s))
                        .collect(),
                )
                .to_string()
            })
            .unwrap_or("[]".to_string());
        let workflow_id = args.get("workflow_id").and_then(|v| v.as_str());
        let task_id = args.get("task_id").and_then(|v| v.as_str());
        let t = now();

        // Generate embedding from summary + content
        let embedding_text = format!("{} {}", summary, content);
        let embedding_vector = crate::embedding::text_to_vector(&embedding_text);
        let embedding_blob = crate::embedding::vector_to_blob(&embedding_vector);

        conn.execute(
            "INSERT INTO memories (id, workflow_id, task_id, memory_type, summary, content, importance_score, created_by_agent, tags, embedding, created_at, updated_at) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12)",
            params![id, workflow_id, task_id, memory_type, summary, content, importance, created_by, &tags, embedding_blob, &t, &t],
        ).map_err(|e| err(e))?;

        let pg_args = (
            id.clone(),
            workflow_id.map(|s| s.to_string()),
            task_id.map(|s| s.to_string()),
            memory_type.to_string(),
            summary.to_string(),
            content.to_string(),
            importance,
            created_by.to_string(),
            tags,
        );
        (id, pg_args)
    }; // conn dropped

    // Fire-and-forget PG sync — non-blocking
    tokio::spawn(async move {
        crate::direct_pg::memory_owned(pg_args).await;
    });

    Ok(json!({"id": id, "stored": true}))
}

pub async fn memory_store_research(
    args: Value,
) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    let conn = crate::sqlite::conn();
    let id = new_id();
    let summary = args["summary"].as_str().unwrap_or("Research finding");
    let content = args["content"]
        .as_str()
        .ok_or_else(|| err("missing content"))?;
    let created_by = args["created_by_agent"].as_str().unwrap_or("masday-mcp");
    let workflow_id = args.get("workflow_id").and_then(|v| v.as_str());
    let t = now();

    conn.execute(
        "INSERT INTO memories (id, workflow_id, memory_type, summary, content, importance_score, created_by_agent, tags, created_at, updated_at) VALUES (?1,?2,'research',?3,?4,0.7,?5,'[]',?6,?7)",
        params![id, workflow_id, summary, content, created_by, &t, &t],
    ).map_err(|e| err(e))?;

    Ok(json!({"id": id, "stored": true}))
}

pub async fn memory_search(args: Value) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    let conn = crate::sqlite::conn();
    let query = args["query"].as_str().ok_or_else(|| err("missing query"))?;
    let limit = args["limit"].as_u64().unwrap_or(10) as i64;
    let pattern = format!("%{}%", query);

    // Step 1: Generate query embedding
    let query_vector = crate::embedding::text_to_vector(query);

    // Step 2: LIKE filter to get candidates (including embedding column)
    let candidate_limit = 100; // Get more candidates for re-ranking
    let mut stmt = conn.prepare(
        "SELECT id, memory_type, summary, importance_score, created_at, embedding FROM memories WHERE summary LIKE ?1 OR content LIKE ?1 LIMIT ?2"
    ).map_err(err)?;

    // Step 3: Collect candidates and compute similarity
    let mut candidates: Vec<(Value, f32)> = Vec::new();
    let rows = stmt
        .query_map(params![&pattern, candidate_limit], |row| {
            let id: String = row.get(0)?;
            let memory_type: String = row.get(1)?;
            let summary: String = row.get(2)?;
            let importance: f64 = row.get(3)?;
            let created_at: String = row.get(4)?;
            let embedding_blob: Option<Vec<u8>> = row.get(5)?;

            let similarity = if let Some(blob) = embedding_blob {
                let candidate_vector = crate::embedding::blob_to_vector(&blob);
                if !candidate_vector.is_empty() {
                    crate::embedding::cosine_similarity(&query_vector, &candidate_vector)
                } else {
                    0.3 // Default low similarity for empty embeddings
                }
            } else {
                0.3 // Default low similarity for NULL embeddings
            };

            Ok((id, memory_type, summary, importance, created_at, similarity))
        })
        .map_err(err)?;

    for row_result in rows {
        match row_result {
            Ok((id, memory_type, summary, importance, created_at, similarity)) => {
                let result = json!({
                    "id": id,
                    "memoryType": memory_type,
                    "summary": summary,
                    "importanceScore": importance,
                    "createdAt": created_at,
                    "similarity": similarity,
                });
                candidates.push((result, similarity));
            }
            Err(e) => {
                warn!("memory_search: failed to process candidate row: {}", e);
            }
        }
    }

    // Step 4: Sort by hybrid score (similarity * 0.7 + importance * 0.3)
    candidates.sort_by(|a, b| {
        let score_a = f64::from(a.1) * 0.7
            + (a.0
                .get("importanceScore")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.5)
                * 0.3);
        let score_b = f64::from(b.1) * 0.7
            + (b.0
                .get("importanceScore")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.5)
                * 0.3);
        score_b
            .partial_cmp(&score_a)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    // Step 5: Return top-K results
    let results: Vec<Value> = candidates
        .into_iter()
        .take(limit as usize)
        .map(|(result, _)| result)
        .collect();

    Ok(json!({"results": results}))
}

pub async fn memory_recall_documents(
    args: Value,
) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    let conn = crate::sqlite::conn();
    let workflow_id = args["workflow_id"]
        .as_str()
        .ok_or_else(|| err("missing workflow_id"))?;
    let limit = args["limit"].as_u64().unwrap_or(10) as i64;

    let mut stmt = conn.prepare(
        "SELECT id, memory_type, summary, content, importance_score, created_at FROM memories WHERE workflow_id=?1 ORDER BY created_at DESC LIMIT ?2"
    ).map_err(err)?;

    let rows = stmt
        .query_map(params![workflow_id, limit], |row| {
            Ok(json!({
                "id": row.get::<_, String>(0)?,
                "memoryType": row.get::<_, String>(1)?,
                "summary": row.get::<_, String>(2)?,
                "content": row.get::<_, String>(3)?,
                "importanceScore": row.get::<_, f64>(4)?,
                "createdAt": row.get::<_, String>(5)?,
            }))
        })
        .map_err(err)?;

    let mut docs = Vec::new();
    for r in rows {
        docs.push(r.map_err(err)?);
    }

    Ok(json!({"documents": docs, "count": docs.len(), "workflowId": workflow_id}))
}

pub async fn memory_recall_document_by_type(
    args: Value,
) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    let conn = crate::sqlite::conn();
    let source_type = args["source_type"]
        .as_str()
        .ok_or_else(|| err("missing source_type"))?;
    let limit = args["limit"].as_u64().unwrap_or(10) as i64;

    let mut stmt = conn.prepare(
        "SELECT id, memory_type, summary, content, created_at FROM memories WHERE memory_type=?1 ORDER BY created_at DESC LIMIT ?2"
    ).map_err(err)?;

    let rows = stmt
        .query_map(params![source_type, limit], |row| {
            Ok(json!({
                "id": row.get::<_, String>(0)?,
                "memoryType": row.get::<_, String>(1)?,
                "summary": row.get::<_, String>(2)?,
                "content": row.get::<_, String>(3)?,
                "createdAt": row.get::<_, String>(4)?,
            }))
        })
        .map_err(err)?;

    let results: Vec<Value> = collect_rows(rows);
    Ok(json!({"results": results}))
}

pub async fn memory_recall_by_task(
    args: Value,
) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    let conn = crate::sqlite::conn();
    let task_id = args["task_id"]
        .as_str()
        .ok_or_else(|| err("missing task_id"))?;
    let limit = args["limit"].as_u64().unwrap_or(10) as i64;

    let mut stmt = conn.prepare(
        "SELECT id, memory_type, summary, content, importance_score, created_at FROM memories WHERE task_id=?1 ORDER BY created_at DESC LIMIT ?2"
    ).map_err(err)?;

    let rows = stmt
        .query_map(params![task_id, limit], |row| {
            Ok(json!({
                "id": row.get::<_, String>(0)?,
                "memoryType": row.get::<_, String>(1)?,
                "summary": row.get::<_, String>(2)?,
                "content": row.get::<_, String>(3)?,
                "importanceScore": row.get::<_, f64>(4)?,
                "createdAt": row.get::<_, String>(5)?,
            }))
        })
        .map_err(err)?;

    let results: Vec<Value> = collect_rows(rows);
    Ok(json!({"memories": results}))
}

pub async fn memory_recall_recent(
    args: Value,
) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    let conn = crate::sqlite::conn();
    let limit = args["limit"].as_u64().unwrap_or(10) as i64;

    let mut stmt = conn.prepare(
        "SELECT id, memory_type, summary, importance_score, created_at FROM memories ORDER BY created_at DESC LIMIT ?1"
    ).map_err(err)?;

    let rows = stmt
        .query_map(params![limit], |row| {
            Ok(json!({
                "id": row.get::<_, String>(0)?,
                "memoryType": row.get::<_, String>(1)?,
                "summary": row.get::<_, String>(2)?,
                "importanceScore": row.get::<_, f64>(3)?,
                "createdAt": row.get::<_, String>(4)?,
            }))
        })
        .map_err(err)?;

    let results: Vec<Value> = collect_rows(rows);
    Ok(json!({"memories": results}))
}

pub async fn memory_update(args: Value) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    let conn = crate::sqlite::conn();
    let id = args["id"].as_str().ok_or_else(|| err("missing id"))?;
    let content = args.get("content").and_then(|v| v.as_str());
    let importance = args["importance"].as_f64();
    let t = now();

    match (content, importance) {
        (Some(c), Some(imp)) => {
            // Get summary to regenerate embedding
            let summary: String = conn
                .query_row(
                    "SELECT summary FROM memories WHERE id=?1",
                    params![id],
                    |r| r.get(0),
                )
                .map_err(err)?;

            // Regenerate embedding from summary + new content
            let embedding_text = format!("{} {}", summary, c);
            let embedding_vector = crate::embedding::text_to_vector(&embedding_text);
            let embedding_blob = crate::embedding::vector_to_blob(&embedding_vector);

            conn.execute("UPDATE memories SET content=?1, importance_score=?2, embedding=?3, updated_at=?4, version=version+1 WHERE id=?5",
                params![c, imp, embedding_blob, &t, id]).map_err(|e| err(e))?;
        }
        (Some(c), None) => {
            // Get summary to regenerate embedding
            let summary: String = conn
                .query_row(
                    "SELECT summary FROM memories WHERE id=?1",
                    params![id],
                    |r| r.get(0),
                )
                .map_err(err)?;

            // Regenerate embedding from summary + new content
            let embedding_text = format!("{} {}", summary, c);
            let embedding_vector = crate::embedding::text_to_vector(&embedding_text);
            let embedding_blob = crate::embedding::vector_to_blob(&embedding_vector);

            conn.execute("UPDATE memories SET content=?1, embedding=?2, updated_at=?3, version=version+1 WHERE id=?4",
                params![c, embedding_blob, &t, id]).map_err(|e| err(e))?;
        }
        (None, Some(imp)) => {
            conn.execute("UPDATE memories SET importance_score=?1, updated_at=?2, version=version+1 WHERE id=?3",
                params![imp, &t, id]).map_err(|e| err(e))?;
        }
        (None, None) => {
            conn.execute(
                "UPDATE memories SET updated_at=?1 WHERE id=?2",
                params![&t, id],
            )
            .map_err(err)?;
        }
    }

    Ok(json!({"updated": id}))
}

pub async fn memory_delete(args: Value) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    let conn = crate::sqlite::conn();
    let id = args["id"].as_str().ok_or_else(|| err("missing id"))?;

    let count = conn
        .execute("DELETE FROM memories WHERE id=?1", params![id])
        .map_err(err)?;
    if count == 0 {
        return Err(format!("Memory not found: {}", id).into());
    }
    Ok(json!({"deleted": id}))
}

pub async fn memory_delete_by_workflow(
    args: Value,
) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    let conn = crate::sqlite::conn();
    let workflow_id = args["workflow_id"]
        .as_str()
        .ok_or_else(|| err("missing workflow_id"))?;

    let count = conn
        .execute(
            "DELETE FROM memories WHERE workflow_id=?1",
            params![workflow_id],
        )
        .map_err(err)?;
    if count == 0 {
        return Err(format!("No memories found for workflow: {}", workflow_id).into());
    }
    Ok(json!({"deleted": count, "workflow_id": workflow_id}))
}

pub async fn memory_stats(_args: Value) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    let conn = crate::sqlite::conn();

    let total: i64 = conn
        .query_row("SELECT COUNT(*) FROM memories", [], |r| r.get(0))
        .map_err(err)?;
    let by_type: String = conn.query_row(
        "SELECT json_group_object(memory_type, cnt) FROM (SELECT memory_type, COUNT(*) as cnt FROM memories GROUP BY memory_type)",
        [], |r| r.get(0)
    ).map_err(|e| err(e))?;

    Ok(
        json!({"total": total, "by_type": serde_json::from_str::<Value>(&by_type).unwrap_or(json!({}))}),
    )
}

pub async fn episodic_store(
    args: Value,
) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    let conn = crate::sqlite::conn();
    let id = new_id();
    let session_id = args["session_id"]
        .as_str()
        .ok_or_else(|| err("missing session_id"))?;
    let role = args["role"].as_str().ok_or_else(|| err("missing role"))?;
    let content = args["content"]
        .as_str()
        .ok_or_else(|| err("missing content"))?;
    let t = now();

    // Get next sequence order
    let seq: i64 = conn
        .query_row(
            "SELECT COALESCE(MAX(sequence_order),0)+1 FROM episodic_memories WHERE session_id=?1",
            params![session_id],
            |r| r.get(0),
        )
        .map_err(err)?;

    conn.execute(
        "INSERT INTO episodic_memories (id, session_id, role, content, sequence_order, created_at) VALUES (?1,?2,?3,?4,?5,?6)",
        params![id, session_id, role, content, seq, &t],
    ).map_err(|e| err(e))?;

    Ok(json!({"stored": true, "session_id": session_id}))
}

pub async fn episodic_recall(
    args: Value,
) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    let conn = crate::sqlite::conn();
    let session_id = args["session_id"]
        .as_str()
        .ok_or_else(|| err("missing session_id"))?;
    let limit = args["limit"].as_u64().unwrap_or(50) as i64;

    let mut stmt = conn.prepare(
        "SELECT id, role, content, sequence_order, created_at FROM episodic_memories WHERE session_id=?1 ORDER BY sequence_order LIMIT ?2"
    ).map_err(err)?;

    let rows = stmt
        .query_map(params![session_id, limit], |row| {
            Ok(json!({
                "id": row.get::<_, String>(0)?,
                "role": row.get::<_, String>(1)?,
                "content": row.get::<_, String>(2)?,
                "sequenceOrder": row.get::<_, i64>(3)?,
                "createdAt": row.get::<_, String>(4)?,
            }))
        })
        .map_err(err)?;

    let results: Vec<Value> = collect_rows(rows);
    Ok(json!({"memories": results}))
}

// ============================================================================
// Review Tools (2)
// ============================================================================

pub async fn review_submit(args: Value) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    let conn = crate::sqlite::conn();
    let id = new_id();
    let workflow_id = args["workflow_id"]
        .as_str()
        .ok_or_else(|| err("missing workflow_id"))?;
    let task_id = args["task_id"]
        .as_str()
        .ok_or_else(|| err("missing task_id"))?;
    let reviewer = args["reviewer_agent"]
        .as_str()
        .or_else(|| args["reviewer"].as_str())
        .ok_or_else(|| err("missing reviewer_agent"))?;
    let decision = args["decision"]
        .as_str()
        .ok_or_else(|| err("missing decision"))?;
    let notes = args["notes"].as_str().unwrap_or("");
    let gaps = args.get("gaps").map(|v| v.to_string());
    let t = now();

    conn.execute(
        "INSERT INTO review_decisions (id, workflow_id, task_id, reviewer_agent, decision, notes, gaps, created_at) VALUES (?1,?2,?3,?4,?5,?6,?7,?8)",
        params![id, workflow_id, task_id, reviewer, decision, notes, gaps, &t],
    ).map_err(|e| err(e))?;

    Ok(json!({"submitted": true}))
}

pub async fn review_get_latest(
    args: Value,
) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    let conn = crate::sqlite::conn();
    let task_id = args["task_id"]
        .as_str()
        .ok_or_else(|| err("missing task_id"))?;

    let review = match conn.query_row(
        "SELECT id, reviewer_agent, decision, notes, gaps, created_at FROM review_decisions WHERE task_id=?1 ORDER BY created_at DESC LIMIT 1",
        params![task_id],
        |row| Ok(json!({
            "id": row.get::<_, String>(0)?,
            "reviewerAgent": row.get::<_, String>(1)?,
            "decision": row.get::<_, String>(2)?,
            "notes": row.get::<_, String>(3)?,
            "gaps": opt_json(row, 4),
            "createdAt": row.get::<_, String>(5)?,
        })),
    ) {
        Ok(r) => Some(r),
        Err(rusqlite::Error::QueryReturnedNoRows) => None,
        Err(e) => return Err(err(e)),
    };

    match review {
        Some(r) => Ok(json!({"review": r})),
        None => Err(err("No review found for task")),
    }
}

// ============================================================================
// Session Tools (3)
// ============================================================================

pub async fn session_init_context(
    args: Value,
) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    let conn = crate::sqlite::conn();
    let cwd = args["cwd"].as_str().ok_or_else(|| err("missing cwd"))?;
    let session_key = format!("session:{}", cwd.replace('/', ":"));
    let id = new_id();
    let t = now();
    let meta = json!({"cwd": cwd}).to_string();

    conn.execute(
        "INSERT INTO session_states (id, session_key, metadata, created_at, updated_at) VALUES (?1,?2,?3,?4,?5) ON CONFLICT(session_key) DO UPDATE SET updated_at=?5",
        params![id, &session_key, &meta, &t, &t],
    ).map_err(|e| err(e))?;

    Ok(json!({"session_key": session_key, "initialized": true}))
}

pub async fn session_get_state(
    args: Value,
) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    let conn = crate::sqlite::conn();
    let session_key = args["session_key"]
        .as_str()
        .ok_or_else(|| err("missing session_key"))?;

    let state = conn.query_row(
        "SELECT session_key, workflow_id, plan_id, task_id, execution_mode, metadata FROM session_states WHERE session_key=?1",
        params![session_key],
        |row| Ok(json!({
            "sessionKey": row.get::<_, String>(0)?,
            "workflowId": row.get::<_, Option<String>>(1)?,
            "planId": row.get::<_, Option<String>>(2)?,
            "taskId": row.get::<_, Option<String>>(3)?,
            "executionMode": row.get::<_, Option<String>>(4)?,
            "metadata": json_col(row, 5),
        })),
    );

    match state {
        Ok(s) => Ok(json!({"state": s})),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(json!({
            "state": null,
            "found": false,
            "sessionKey": session_key
        })),
        Err(e) => Err(err(e)),
    }
}

pub async fn session_patch_state(
    args: Value,
) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    let conn = crate::sqlite::conn();
    let session_key = args["session_key"]
        .as_str()
        .ok_or_else(|| err("missing session_key"))?;
    let patch = args
        .get("patch")
        .cloned()
        .ok_or_else(|| err("missing patch"))?;
    let id = new_id();
    let t = now();

    // Upsert: merge metadata if exists
    let existing_meta: String = conn
        .query_row(
            "SELECT metadata FROM session_states WHERE session_key=?1",
            params![session_key],
            |r| r.get(0),
        )
        .unwrap_or_else(|e| {
            warn!(
                "session_patch_state: no existing metadata for {}: {e}",
                session_key
            );
            "{}".to_string()
        });

    let mut meta: Value = serde_json::from_str(&existing_meta).unwrap_or(json!({}));
    if let Some(obj) = patch.as_object() {
        for (k, v) in obj {
            meta[k] = v.clone();
        }
    }

    conn.execute(
        "INSERT INTO session_states (id, session_key, workflow_id, plan_id, task_id, execution_mode, metadata, created_at, updated_at) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9) ON CONFLICT(session_key) DO UPDATE SET metadata=?7, updated_at=?9",
        params![id, session_key,
            patch.get("workflow_id").and_then(|v| v.as_str()),
            patch.get("plan_id").and_then(|v| v.as_str()),
            patch.get("task_id").and_then(|v| v.as_str()),
            patch.get("execution_mode").and_then(|v| v.as_str()),
            meta.to_string(), &t, &t],
    ).map_err(|e| err(e))?;

    Ok(json!({"session_key": session_key, "patched": true}))
}

// ============================================================================
// Context/Search Tools (5)
// ============================================================================

pub async fn search_hybrid_context_pack(
    args: Value,
) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    let conn = crate::sqlite::conn();
    let workflow_id = args["workflow_id"]
        .as_str()
        .ok_or_else(|| err("missing workflow_id"))?;

    // Gather context from memories and tasks
    let mut stmt = conn.prepare(
        "SELECT id, memory_type, summary, content FROM memories WHERE workflow_id=?1 ORDER BY importance_score DESC LIMIT 10"
    ).map_err(err)?;

    let mem_rows = stmt.query_map(params![workflow_id], |row| {
        Ok(json!({"id": row.get::<_, String>(0)?, "type": row.get::<_, String>(1)?, "summary": row.get::<_, String>(2)?}))
    }).map_err(|e| err(e))?;

    let memories: Vec<Value> = collect_rows(mem_rows);

    let mut stmt2 = conn
        .prepare("SELECT id, title, status FROM tasks WHERE workflow_id=?1 ORDER BY created_at")
        .map_err(err)?;

    let task_rows = stmt2.query_map(params![workflow_id], |row| {
        Ok(json!({"id": row.get::<_, String>(0)?, "title": row.get::<_, String>(1)?, "status": row.get::<_, String>(2)?}))
    }).map_err(|e| err(e))?;

    let tasks: Vec<Value> = collect_rows(task_rows);

    Ok(json!({"context_pack": {"memories": memories, "tasks": tasks}}))
}

pub async fn search_context_fingerprint(
    _args: Value,
) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    Ok(
        json!({"fingerprint": format!("fp-{:x}", std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs())}),
    )
}

pub async fn semantic_search_code_search(
    args: Value,
) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    // Filesystem-based code search (grep) — no DB needed
    let query = args["query"].as_str().ok_or_else(|| err("missing query"))?;
    let project_path = args
        .get("project_path")
        .and_then(|v| v.as_str())
        .unwrap_or(".");

    let output = std::process::Command::new("grep")
        .args([
            "-rn",
            "--include=*.rs",
            "--include=*.ts",
            "--include=*.js",
            "--include=*.py",
            "--exclude-dir=node_modules",
            "--exclude-dir=target",
            "--exclude-dir=.git",
            "--exclude-dir=dist",
            "--exclude-dir=build",
            "--exclude-dir=.next",
            "--exclude-dir=__pycache__",
            "--exclude-dir=.venv",
            query,
            project_path,
        ])
        .output();

    let results = match output {
        Ok(out) => String::from_utf8_lossy(&out.stdout)
            .lines()
            .take(20)
            .map(|line| json!({"match": line}))
            .collect::<Vec<_>>(),
        Err(_) => vec![],
    };

    Ok(json!({"results": results}))
}

pub async fn semantic_search_search_hybrid_context_pack(
    args: Value,
) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    search_hybrid_context_pack(args).await
}

pub async fn semantic_search_make_fingerprint(
    args: Value,
) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    let workflow_id = args["workflow_id"].as_str().unwrap_or("");
    let plan_id = args["plan_id"].as_str().unwrap_or("");
    let task_id = args["task_id"].as_str().unwrap_or("");

    Ok(json!({
        "fingerprint": format!("fp-{}-{}-{}", workflow_id, plan_id, task_id),
        "workflow_id": workflow_id,
        "plan_id": plan_id,
        "task_id": task_id,
    }))
}

// ============================================================================
// Policy Tools (6)
// ============================================================================

pub async fn policy_validate_execution(
    args: Value,
) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    let conn = crate::sqlite::conn();
    let workflow_id = args["workflow_id"]
        .as_str()
        .ok_or_else(|| err("missing workflow_id"))?;
    let task_id = args["task_id"]
        .as_str()
        .ok_or_else(|| err("missing task_id"))?;

    // Check workflow exists and is in EXECUTE state
    let wf_status: String = conn
        .query_row(
            "SELECT status FROM workflows WHERE id=?1",
            params![workflow_id],
            |r| r.get(0),
        )
        .map_err(err)?;

    let valid = wf_status == "EXECUTE" || wf_status == "PLAN";

    // Check task exists and is PENDING or RUNNING
    let task_status: String = conn
        .query_row(
            "SELECT status FROM tasks WHERE id=?1",
            params![task_id],
            |r| r.get(0),
        )
        .map_err(err)?;

    let valid = valid && (task_status == "PENDING" || task_status == "RUNNING");

    Ok(json!({"valid": valid}))
}

pub async fn policy_validate_completion(
    args: Value,
) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    let conn = crate::sqlite::conn();
    let workflow_id = args["workflow_id"]
        .as_str()
        .ok_or_else(|| err("missing workflow_id"))?;
    let task_id = args["task_id"]
        .as_str()
        .ok_or_else(|| err("missing task_id"))?;

    // Check task status
    let result = conn.query_row(
        "SELECT status, result FROM tasks WHERE id=?1",
        params![task_id],
        |r| {
            Ok((
                r.get::<_, String>(0).unwrap_or_default(),
                r.get::<_, Option<String>>(1).unwrap_or_default(),
            ))
        },
    );

    let (task_status, task_result) = match result {
        Ok(r) => r,
        Err(_) => {
            return Err(err(format!(
                "Task {} not found in workflow {}",
                task_id, workflow_id
            )))
        }
    };

    if task_status == "DONE" {
        // Check if all tasks in workflow are done
        let incomplete_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM tasks WHERE workflow_id=?1 AND status != 'DONE'",
                params![workflow_id],
                |r| r.get(0),
            )
            .unwrap_or(0);

        let all_done = incomplete_count == 0;
        let detail = if all_done {
            "Task complete, all workflow tasks done"
        } else {
            "Some workflow tasks still pending/running"
        };
        let has_result = task_result.is_some();
        Ok(json!({
            "valid": true,
            "task_status": task_status,
            "has_result": has_result,
            "all_workflow_tasks_done": all_done,
            "incomplete_count": incomplete_count,
            "detail": detail
        }))
    } else {
        Ok(json!({
            "valid": false,
            "task_status": task_status,
            "reason": format!("Task status is {} (expected DONE). Complete the task first with workflow_completeTask.", task_status),
            "suggestion": "Use workflow_saveProgress to save work, then workflow_completeTask to mark DONE."
        }))
    }
}

pub async fn policy_validate_parallel_completion(
    _args: Value,
) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    Ok(json!({"valid": true}))
}

pub async fn policy_detect_scope_drift(
    args: Value,
) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    let _workflow_id = args["workflow_id"]
        .as_str()
        .ok_or_else(|| err("missing workflow_id"))?;
    let _task_id = args.get("task_id").and_then(|v| v.as_str()).unwrap_or("");
    let _output_text = args
        .get("output_text")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    // Simple stub — no LLM-based drift detection in standalone mode
    Ok(json!({"drift_detected": false, "drift_detail": null}))
}

pub async fn policy_require_context_refresh(
    _args: Value,
) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    Ok(json!({"require_refresh": false, "reason": "No stale context detected"}))
}

pub async fn policy_check_session_readiness(
    args: Value,
) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    let system = system_readiness_check().await;
    let session_key = args
        .get("session_key")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    Ok(json!({
        "ready": system["ready"],
        "session_key": session_key,
        "system": system,
    }))
}

// ============================================================================
// Reminder Tools (3)
// ============================================================================

pub async fn reminder_check(
    _args: Value,
) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    let conn = crate::sqlite::conn();

    // Find stale tasks (RUNNING for >1 hour — simplified check)
    let mut stmt = conn.prepare(
        "SELECT id, workflow_id, title FROM tasks WHERE status='RUNNING' AND updated_at < datetime('now', '-1 hour')"
    ).map_err(err)?;

    let rows = stmt.query_map([], |row| {
        Ok(json!({"id": row.get::<_, String>(0)?, "workflow_id": row.get::<_, String>(1)?, "title": row.get::<_, String>(2)?, "type": "stale"}))
    }).map_err(|e| err(e))?;

    let reminders: Vec<Value> = collect_rows(rows);
    Ok(json!({"reminders": reminders}))
}

pub async fn reminder_list(args: Value) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    let conn = crate::sqlite::conn();
    let workflow_id = args
        .get("workflow_id")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    let mut stmt = conn.prepare(
        "SELECT id, workflow_id, reminder_type, severity, message, acknowledged FROM workflow_reminders WHERE workflow_id=?1"
    ).map_err(err)?;

    let rows = stmt
        .query_map(params![workflow_id], |row| {
            Ok(json!({
                "id": row.get::<_, String>(0)?,
                "workflowId": row.get::<_, String>(1)?,
                "type": row.get::<_, String>(2)?,
                "severity": row.get::<_, String>(3)?,
                "message": row.get::<_, String>(4)?,
                "acknowledged": row.get::<_, i64>(5)? == 1,
            }))
        })
        .map_err(err)?;

    let reminders: Vec<Value> = collect_rows(rows);
    Ok(json!({"reminders": reminders}))
}

pub async fn reminder_acknowledge(
    args: Value,
) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    let conn = crate::sqlite::conn();
    let id = args
        .get("id")
        .or_else(|| args.get("workflowId"))
        .and_then(|v| v.as_str())
        .ok_or_else(|| err("missing id"))?;

    conn.execute(
        "UPDATE workflow_reminders SET acknowledged=1 WHERE id=?1",
        params![id],
    )
    .map_err(|e| err(e))?;

    Ok(json!({"acknowledged": id}))
}

// ============================================================================
// Graph Tools (2)
// ============================================================================

pub async fn memory_create_entities(
    args: Value,
) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    let conn = crate::sqlite::conn();
    // Accept entities as either a JSON array or a JSON string containing an array
    let entities_val = if args["entities"].is_array() {
        args["entities"].clone()
    } else if let Some(s) = args["entities"].as_str() {
        serde_json::from_str::<Value>(s)
            .map_err(|e| err(format!("invalid entities JSON: {}", e)))?
    } else {
        return Err(err("missing entities: expected array or JSON string"));
    };
    let entities_arr = entities_val
        .as_array()
        .ok_or_else(|| err("entities must be a JSON array"))?;
    let t = now();
    let mut created = Vec::new();

    for entity_val in entities_arr {
        let id = new_id();
        let name = entity_val["name"]
            .as_str()
            .ok_or_else(|| err("missing name in entity"))?;
        let entity_type = entity_val["entityType"]
            .as_str()
            .ok_or_else(|| err("missing entityType"))?;
        let observations: Vec<String> = entity_val
            .get("observations")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default();
        let props = json!({"observations": observations}).to_string();

        conn.execute(
            "INSERT INTO graph_nodes (id, node_type, name, properties, created_at) VALUES (?1,?2,?3,?4,?5)",
            params![id, entity_type, name, &props, &t],
        ).map_err(|e| err(e))?;

        created.push(json!({"id": id, "name": name, "entityType": entity_type}));
    }

    Ok(json!({"entities": created}))
}

pub async fn memory_search_nodes(
    args: Value,
) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    let conn = crate::sqlite::conn();
    let query = args["query"].as_str().ok_or_else(|| err("missing query"))?;
    let pattern = format!("%{}%", query);

    let mut stmt = conn.prepare(
        "SELECT id, node_type, name, properties FROM graph_nodes WHERE name LIKE ?1 OR properties LIKE ?1"
    ).map_err(err)?;

    let rows = stmt
        .query_map(params![&pattern], |row| {
            Ok(json!({
                "id": row.get::<_, String>(0)?,
                "entityType": row.get::<_, String>(1)?,
                "name": row.get::<_, String>(2)?,
                "properties": json_col(row, 3),
            }))
        })
        .map_err(err)?;

    let nodes: Vec<Value> = collect_rows(rows);
    Ok(json!({"nodes": nodes}))
}

// ============================================================================
// Capability Tools (11) — mostly filesystem, minimal DB
// ============================================================================

pub async fn capability_list_agents(
    args: Value,
) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    let project_root = args["projectRoot"]
        .as_str()
        .or_else(|| args["project_root"].as_str())
        .ok_or_else(|| err("missing projectRoot"))?;

    let registry = load_registry(project_root);
    let agents: Vec<Value> = registry["components"]["agents"]
        .as_array()
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .map(|a| json!({"name": a["name"]}))
        .collect();

    Ok(json!({"agents": agents}))
}

pub async fn capability_list_skills(
    args: Value,
) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    let project_root = args["projectRoot"]
        .as_str()
        .or_else(|| args["project_root"].as_str())
        .ok_or_else(|| err("missing projectRoot"))?;

    let registry = load_registry(project_root);
    let skills: Vec<Value> = registry["components"]["skills"]
        .as_array()
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .map(|s| json!({"name": s["name"]}))
        .collect();

    Ok(json!({"skills": skills}))
}

pub async fn capability_match_agent(
    args: Value,
) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    let project_root = args["projectRoot"]
        .as_str()
        .or_else(|| args["project_root"].as_str())
        .unwrap_or(".");
    let task_desc = args["taskDescription"]
        .as_str()
        .or_else(|| args["task_description"].as_str())
        .ok_or_else(|| err("missing taskDescription"))?;

    let registry = load_registry(project_root);
    let agents = registry["components"]["agents"]
        .as_array()
        .cloned()
        .unwrap_or_default();

    if agents.is_empty() {
        return Ok(
            json!({"agent": "masday-orchestrator", "reason": "no agents in registry, using default"}),
        );
    }

    // Tokenize task description
    let task_lower = task_desc.to_lowercase();
    let task_tokens: Vec<&str> = task_lower
        .split(|c: char| !c.is_alphanumeric())
        .filter(|t| t.len() > 2)
        .collect();

    // Score each agent by keyword overlap against name + role + description + category
    let mut best_name = "masday-orchestrator".to_string();
    let mut best_score = 0.0f64;

    for agent in &agents {
        let text = format!(
            "{} {} {} {}",
            agent["name"].as_str().unwrap_or(""),
            agent["role"].as_str().unwrap_or(""),
            agent["description"].as_str().unwrap_or(""),
            agent["category"].as_str().unwrap_or(""),
        );
        let text_lower = text.to_lowercase();
        let agent_tokens: Vec<&str> = text_lower
            .split(|c: char| !c.is_alphanumeric())
            .filter(|t| t.len() > 2)
            .collect();

        let agent_set: std::collections::HashSet<&str> = agent_tokens.into_iter().collect();
        let matches = task_tokens
            .iter()
            .filter(|t| agent_set.contains(*t))
            .count();
        let score = if task_tokens.is_empty() {
            0.0
        } else {
            matches as f64 / task_tokens.len() as f64
        };

        if score > best_score {
            best_score = score;
            best_name = agent["name"]
                .as_str()
                .unwrap_or("masday-orchestrator")
                .to_string();
        }
    }

    // Fall back to orchestrator if no meaningful match
    if best_score < 0.05 {
        best_name = "masday-orchestrator".to_string();
    }

    Ok(json!({"agent": best_name}))
}

pub async fn capability_scaffold_feature(
    _args: Value,
) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    Ok(json!({"scaffold": "feature_scaffold_v1", "description": "Feature scaffold template"}))
}

pub async fn capability_scaffold_mcp_server(
    _args: Value,
) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    Ok(json!({"scaffold": "mcp_server_scaffold_v1", "description": "MCP server scaffold template"}))
}

pub async fn capability_system_readiness(
    args: Value,
) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    // Validate projectRoot if provided
    if let Some(root) = args.get("projectRoot").and_then(|v| v.as_str()) {
        if !root.is_empty() && !std::path::Path::new(root).exists() {
            return Err(format!("projectRoot does not exist: {}", root).into());
        }
    }
    Ok(system_readiness_check().await)
}

pub async fn capability_workflow_audit(
    args: Value,
) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    let conn = crate::sqlite::conn();
    let workflow_id = args
        .get("workflowId")
        .or_else(|| args.get("workflow_id"))
        .and_then(|v| v.as_str())
        .ok_or_else(|| err("missing workflowId"))?;

    let wf = conn.query_row(
        "SELECT id, name, status FROM workflows WHERE id=?1", params![workflow_id],
        |row| Ok(json!({"id": row.get::<_, String>(0)?, "name": row.get::<_, String>(1)?, "status": row.get::<_, String>(2)?})),
    ).map_err(|e| err(e))?;

    let task_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM tasks WHERE workflow_id=?1",
            params![workflow_id],
            |r| r.get(0),
        )
        .map_err(err)?;

    Ok(json!({"workflow": wf["id"], "status": wf["status"], "tasks_count": task_count}))
}

pub async fn capability_create_agent(
    args: Value,
) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    let project_root = args["projectRoot"].as_str().unwrap_or(".");
    let name = args["name"].as_str().ok_or_else(|| err("missing name"))?;
    let role = args["role"].as_str().unwrap_or("general");
    let description = args["description"].as_str().unwrap_or("");
    let instructions = args["instructions"].as_str().unwrap_or("");

    let dir = std::path::Path::new(project_root).join(".claude/agents");
    std::fs::create_dir_all(&dir).map_err(err)?;
    let content = format!(
        "---\nname: {}\nrole: {}\n---\n\n# {}\n\n{}",
        name, role, description, instructions
    );
    std::fs::write(dir.join(format!("{}.md", name)), content).map_err(err)?;

    Ok(json!({"created": name}))
}

pub async fn capability_create_skill(
    args: Value,
) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    let project_root = args["projectRoot"].as_str().unwrap_or(".");
    let name = args["name"].as_str().ok_or_else(|| err("missing name"))?;
    let description = args["description"].as_str().unwrap_or("");

    let dir = std::path::Path::new(project_root).join(format!(".claude/skills/{}", name));
    std::fs::create_dir_all(&dir).map_err(err)?;
    let content = format!(
        "---\nname: {}\ndescription: {}\n---\n\n# {}",
        name, description, name
    );
    std::fs::write(dir.join("SKILL.md"), content).map_err(err)?;

    Ok(json!({"created": name}))
}

pub async fn capability_list_templates(
    _args: Value,
) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    Ok(json!({"templates": []}))
}

// ============================================================================
// Local Tools (2)
// ============================================================================

pub async fn local_push(args: Value) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    let cwd = args
        .get("cwd")
        .and_then(|v| v.as_str())
        .ok_or_else(|| err("missing cwd"))?;

    let workflow_id_opt = args.get("workflow_id").and_then(|v| v.as_str());

    let state_dir = std::path::Path::new(cwd)
        .join(".masday")
        .join("state")
        .join("workflows");

    // Ensure directory exists
    if !state_dir.exists() {
        return Err(format!(
            "State directory does not exist: {}",
            state_dir.to_string_lossy()
        )
        .into());
    }

    let (pushed_workflows, errors) = {
        let mut pushed_workflows: Vec<String> = Vec::new();
        let mut errors: Vec<Value> = Vec::new();

        // Collect workflow files to push
        let workflow_files: Vec<std::path::PathBuf> = if let Some(workflow_id) = workflow_id_opt {
            // Push specific workflow
            let workflow_id = crate::client::sanitize_id(workflow_id)
                .ok_or_else(|| err("invalid workflow_id: contains disallowed characters"))?;
            let workflow_file = state_dir.join(format!("{}.json", workflow_id));
            if workflow_file.exists() {
                vec![workflow_file]
            } else {
                return Ok(json!({
                    "pushed": false,
                    "error": "Workflow file not found",
                    "workflow_id": workflow_id,
                    "expected_path": workflow_file.to_string_lossy().to_string()
                }));
            }
        } else {
            // Push all workflows
            let entries = std::fs::read_dir(&state_dir)
                .map_err(|e| err(format!("Failed to read state directory: {}", e)))?;

            entries
                .filter_map(|entry| entry.ok())
                .map(|entry| entry.path())
                .filter(|path| path.extension().and_then(|s| s.to_str()) == Some("json"))
                .collect()
        };

        let conn = crate::sqlite::conn();

        // Push each workflow state
        for workflow_file in workflow_files {
            let file_stem = workflow_file
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("unknown");

            let content = match std::fs::read_to_string(&workflow_file) {
                Ok(c) => c,
                Err(e) => {
                    errors.push(json!({
                        "workflow_id": file_stem,
                        "error": format!("Failed to read file: {}", e)
                    }));
                    continue;
                }
            };

            let state: Value = match serde_json::from_str(&content) {
                Ok(s) => s,
                Err(e) => {
                    errors.push(json!({
                        "workflow_id": file_stem,
                        "error": format!("Failed to parse JSON: {}", e)
                    }));
                    continue;
                }
            };

            // Extract workflow data
            let workflow_data = state.get("workflow").cloned().unwrap_or(state.clone());
            let workflow_id = workflow_data
                .get("id")
                .or_else(|| workflow_data.get("workflow_id"))
                .and_then(|v| v.as_str())
                .unwrap_or(file_stem);

            // Validate workflow_id is safe
            if crate::client::sanitize_id(workflow_id).is_none() {
                errors.push(json!({
                    "workflow_id": file_stem,
                    "error": "Invalid workflow_id (contains disallowed characters)"
                }));
                continue;
            }

            let t = now();

            // Update workflow in SQLite
            // Check if workflow exists first
            let exists: Result<i64, _> = conn.query_row(
                "SELECT COUNT(*) FROM workflows WHERE id=?1",
                params![workflow_id],
                |row| row.get(0),
            );

            match exists {
                Ok(0) | Err(_) => {
                    // Workflow doesn't exist, insert it
                    let name = workflow_data
                        .get("name")
                        .and_then(|v| v.as_str())
                        .unwrap_or("");
                    let description = workflow_data.get("description").and_then(|v| v.as_str());
                    let status = workflow_data
                        .get("status")
                        .and_then(|v| v.as_str())
                        .unwrap_or("INIT");
                    let project_path = workflow_data
                        .get("projectPath")
                        .and_then(|v| v.as_str())
                        .or_else(|| workflow_data.get("project_path").and_then(|v| v.as_str()));
                    let metadata = workflow_data.get("metadata").cloned().unwrap_or(json!({}));

                    match conn.execute(
                    "INSERT INTO workflows (id, name, description, status, project_path, metadata, created_at, updated_at) VALUES (?1,?2,?3,?4,?5,?6,?7,?8)",
                    params![workflow_id, name, description, status, project_path, metadata.to_string(), &t, &t],
                ) {
                    Ok(_) => pushed_workflows.push(workflow_id.to_string()),
                    Err(e) => {
                        errors.push(json!({
                            "workflow_id": workflow_id,
                            "error": format!("Failed to insert workflow: {}", e)
                        }));
                        continue;
                    }
                }
                }
                Ok(_) => {
                    // Workflow exists, update it field by field
                    let mut updated = false;

                    if let Some(name) = workflow_data.get("name").and_then(|v| v.as_str()) {
                        match conn.execute(
                            "UPDATE workflows SET name=?1, updated_at=?2 WHERE id=?3",
                            params![name, &t, workflow_id],
                        ) {
                            Ok(_) => updated = true,
                            Err(e) => {
                                errors.push(json!({
                                    "workflow_id": workflow_id,
                                    "error": format!("Failed to update name: {}", e)
                                }));
                            }
                        }
                    }

                    if let Some(description) =
                        workflow_data.get("description").and_then(|v| v.as_str())
                    {
                        match conn.execute(
                            "UPDATE workflows SET description=?1, updated_at=?2 WHERE id=?3",
                            params![description, &t, workflow_id],
                        ) {
                            Ok(_) => updated = true,
                            Err(e) => {
                                errors.push(json!({
                                    "workflow_id": workflow_id,
                                    "error": format!("Failed to update description: {}", e)
                                }));
                            }
                        }
                    }

                    if let Some(status) = workflow_data.get("status").and_then(|v| v.as_str()) {
                        match conn.execute(
                            "UPDATE workflows SET status=?1, updated_at=?2 WHERE id=?3",
                            params![status, &t, workflow_id],
                        ) {
                            Ok(_) => updated = true,
                            Err(e) => {
                                errors.push(json!({
                                    "workflow_id": workflow_id,
                                    "error": format!("Failed to update status: {}", e)
                                }));
                            }
                        }
                    }

                    if let Some(project_path) = workflow_data
                        .get("projectPath")
                        .and_then(|v| v.as_str())
                        .or_else(|| workflow_data.get("project_path").and_then(|v| v.as_str()))
                    {
                        match conn.execute(
                            "UPDATE workflows SET project_path=?1, updated_at=?2 WHERE id=?3",
                            params![project_path, &t, workflow_id],
                        ) {
                            Ok(_) => updated = true,
                            Err(e) => {
                                errors.push(json!({
                                    "workflow_id": workflow_id,
                                    "error": format!("Failed to update project_path: {}", e)
                                }));
                            }
                        }
                    }

                    if let Some(metadata) = workflow_data.get("metadata").cloned() {
                        let metadata_str = metadata.to_string();
                        match conn.execute(
                            "UPDATE workflows SET metadata=?1, updated_at=?2 WHERE id=?3",
                            params![metadata_str, &t, workflow_id],
                        ) {
                            Ok(_) => updated = true,
                            Err(e) => {
                                errors.push(json!({
                                    "workflow_id": workflow_id,
                                    "error": format!("Failed to update metadata: {}", e)
                                }));
                            }
                        }
                    }

                    if updated || errors.is_empty() {
                        pushed_workflows.push(workflow_id.to_string());
                    }
                }
            }

            // Generate embeddings for memory content in standalone mode
            // Note: SQLite doesn't support vector operations, so we just log the embedding generation
            // In HTTP mode (local.rs), embeddings are sent to the API for PostgreSQL storage
            if let Some(tasks) = state.get("tasks").and_then(|v| v.as_array()) {
                for task in tasks {
                    if let Some(task_id) = task.get("id").and_then(|v| v.as_str()) {
                        let embedding_text = {
                            let output = task.get("output").and_then(|v| v.as_str()).unwrap_or("");
                            let result = task.get("result").and_then(|v| v.as_str()).unwrap_or("");
                            format!("{} {}", output, result)
                        };

                        if !embedding_text.trim().is_empty() {
                            // Generate embedding in standalone mode (mock provider only for simplicity)
                            let embedding = generate_embedding_standalone_sync(&embedding_text);
                            if let Some(embedding) = embedding {
                                info!("Generated embedding for task {} in standalone mode: {} dimensions (not stored - SQLite lacks vector support)", task_id, embedding.len());
                            }
                        }
                    }
                }
            }

            // Update task states if present
            if let Some(tasks) = state.get("tasks").and_then(|v| v.as_array()) {
                for task in tasks {
                    let task_id = task.get("id").and_then(|v| v.as_str());
                    let task_status = task.get("status").and_then(|v| v.as_str());

                    if let (Some(tid), Some(tstatus)) = (task_id, task_status) {
                        // Check if task exists
                        let task_exists: Result<i64, _> = conn.query_row(
                            "SELECT COUNT(*) FROM tasks WHERE id=?1",
                            params![tid],
                            |row| row.get(0),
                        );

                        match task_exists {
                            Ok(0) | Err(_) => {
                                // Task doesn't exist - need to check if we should insert or skip
                                // For now, we'll skip tasks that don't exist to avoid FK violations
                                continue;
                            }
                            Ok(_) => {
                                // Update task status field by field
                                let result = task.get("result").and_then(|v| v.as_str());
                                let output = task.get("output").and_then(|v| v.as_str());

                                // Update status and updated_at
                                if let Err(e) = conn.execute(
                                    "UPDATE tasks SET status=?1, updated_at=?2 WHERE id=?3",
                                    params![tstatus, &t, tid],
                                ) {
                                    errors.push(json!({
                                        "workflow_id": workflow_id,
                                        "task_id": tid,
                                        "error": format!("Failed to update task status: {}", e)
                                    }));
                                }

                                // Update result if present
                                if let Some(r) = result {
                                    if let Err(e) = conn.execute(
                                        "UPDATE tasks SET result=?1, updated_at=?2 WHERE id=?3",
                                        params![r, &t, tid],
                                    ) {
                                        errors.push(json!({
                                            "workflow_id": workflow_id,
                                            "task_id": tid,
                                            "error": format!("Failed to update task result: {}", e)
                                        }));
                                    }
                                }

                                // Update output if present
                                if let Some(o) = output {
                                    if let Err(e) = conn.execute(
                                        "UPDATE tasks SET output=?1, updated_at=?2 WHERE id=?3",
                                        params![o, &t, tid],
                                    ) {
                                        errors.push(json!({
                                            "workflow_id": workflow_id,
                                            "task_id": tid,
                                            "error": format!("Failed to update task output: {}", e)
                                        }));
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        (pushed_workflows, errors)
    }; // conn dropped

    // Sync to PostgreSQL — fire-and-forget (do NOT await).
    // Previous code awaited the spawn, which blocked the tool response
    // when PG bulk sync was slow (1221+ memories).
    let wf_ids = pushed_workflows.clone();
    tokio::spawn(async move {
        // Workflow sync with 15s timeout
        let wf_ok = crate::direct_pg::workflows_bulk(&wf_ids).await;
        // Memory bulk push with 30s timeout
        let (mem_synced, mem_skipped, mem_errors) = tokio::time::timeout(
            std::time::Duration::from_secs(30),
            crate::direct_pg::memories_bulk_push(),
        )
        .await
        .unwrap_or((0, 0, vec!["Memory bulk push timed out after 30s".into()]));
        tracing::info!(
            "PG sync complete: workflows={} mem_synced={} mem_skipped={} mem_errors={}",
            wf_ok,
            mem_synced,
            mem_skipped,
            mem_errors.len()
        );
    });

    let pg_available = crate::pg::get_pool().await.is_some();

    Ok(json!({
        "pushed": true,
        "workflows_pushed": pushed_workflows,
        "count": pushed_workflows.len(),
        "postgresql_synced": if pg_available { "background" } else { "skipped_no_pool" },
        "pg_sync_failed": !pg_available,
        "errors": errors
    }))
}

pub async fn local_sync(args: Value) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    use crate::sqlite::conn;

    let cwd = args
        .get("cwd")
        .and_then(|v| v.as_str())
        .ok_or_else(|| err("missing cwd"))?;

    let workflow_id_opt = args.get("workflow_id").and_then(|v| v.as_str());

    // If no workflow_id provided, sync all workflows from SQLite to .masday/state/
    let workflow_id = match workflow_id_opt {
        None | Some("") => return local_sync_all(cwd).await,
        Some(id) => crate::client::sanitize_id(id)
            .ok_or_else(|| err("invalid workflow_id: contains disallowed characters"))?,
    };

    let state_dir = std::path::Path::new(cwd)
        .join(".masday")
        .join("state")
        .join("workflows");

    // Query workflow data: try SQLite first, fallback to PostgreSQL
    let sqlite_result: Option<(Value, Vec<Value>)> = {
        let conn = conn();
        let r: Result<(Value, Vec<Value>), _> = (|| {
            let wf = conn
                .query_row(
                    "SELECT id, name, description, status, project_path, metadata, created_at, updated_at FROM workflows WHERE id=?1",
                    params![workflow_id],
                    |row| Ok(json!({
                        "id": row.get::<_, String>(0)?,
                        "name": row.get::<_, String>(1)?,
                        "description": row.get::<_, Option<String>>(2)?,
                        "status": row.get::<_, String>(3)?,
                        "projectPath": row.get::<_, Option<String>>(4)?,
                        "metadata": json_col(row, 5),
                        "createdAt": row.get::<_, String>(6)?,
                        "updatedAt": row.get::<_, String>(7)?,
                    })),
                )?;

            let mut stmt = conn.prepare(
                "SELECT id, title, status, owner_agent, priority, progress_percent, created_at, updated_at FROM tasks WHERE workflow_id=?1 ORDER BY created_at"
            )?;

            let rows = stmt.query_map(params![workflow_id], |row| {
                Ok(json!({
                    "id": row.get::<_, String>(0)?,
                    "title": row.get::<_, String>(1)?,
                    "status": row.get::<_, String>(2)?,
                    "ownerAgent": row.get::<_, Option<String>>(3)?,
                    "priority": row.get::<_, Option<String>>(4)?,
                    "progressPercent": row.get::<_, Option<i64>>(5)?,
                    "createdAt": row.get::<_, String>(6)?,
                    "updatedAt": row.get::<_, String>(7)?,
                }))
            })?;

            let task_list: Vec<Value> = collect_rows(rows);
            Ok::<(Value, Vec<Value>), rusqlite::Error>((wf, task_list))
        })();
        r.ok()
    }; // conn dropped here

    let (workflow_data, tasks, _source) = match sqlite_result {
        Some((wf, task_list)) => (wf, task_list, "sqlite"),
        None => {
            // Fallback: query PostgreSQL
            tracing::info!(
                "local_sync: workflow {} not in SQLite, trying PostgreSQL",
                workflow_id
            );
            crate::direct_pg::pull_workflow(workflow_id)
                .await
                .map_err(|e| {
                    err(format!(
                        "Workflow {} not found in SQLite or PostgreSQL: {}",
                        workflow_id, e
                    ))
                })?
        }
    };

    // Ensure directory exists (async operation)
    tokio::fs::create_dir_all(&state_dir).await.map_err(err)?;

    // Build state object
    let state = json!({
        "workflow": workflow_data,
        "tasks": tasks,
        "syncedAt": now()
    });

    // Write to file
    let workflow_file = state_dir.join(format!("{}.json", workflow_id));
    let state_json = serde_json::to_string_pretty(&state)
        .map_err(|e| format!("Failed to serialize state: {}", e))?;

    tokio::fs::write(&workflow_file, state_json)
        .await
        .map_err(err)?;

    // Pull memories from PostgreSQL → SQLite (complement to local_push)
    let (mem_pulled, mem_skipped, mem_errors) = crate::direct_pg::memories_bulk_pull().await;

    // Add pull stats to the state response
    let mut state = state;
    if let Some(s) = state.as_object_mut() {
        s.insert("memories_pulled".into(), json!(mem_pulled));
        s.insert("memories_skipped".into(), json!(mem_skipped));
        s.insert("memory_pull_errors".into(), json!(mem_errors));
        // Flag PG failure clearly so callers know sync was incomplete
        let pg_failed = crate::pg::get_pool().await.is_none();
        if pg_failed {
            s.insert("pg_sync_failed".into(), json!(true));
            s.insert("pg_unavailable".into(), json!(true));
        }
    }

    Ok(state)
}

/// Sync all workflows from SQLite to .masday/state/ directory.
/// Called when no specific workflow_id is provided.
async fn local_sync_all(cwd: &str) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    use crate::sqlite::conn;

    let state_dir = std::path::Path::new(cwd)
        .join(".masday")
        .join("state")
        .join("workflows");

    tokio::fs::create_dir_all(&state_dir).await.map_err(err)?;

    // Fetch all workflow IDs from SQLite (sync, then drop conn before any await)
    let wf_ids: Vec<String> = {
        let conn = conn();
        let mut stmt = conn.prepare("SELECT id FROM workflows").map_err(err)?;
        let ids: Vec<String> = stmt
            .query_map([], |row| row.get(0))
            .map_err(err)?
            .filter_map(|r| r.ok())
            .collect();
        ids
    }; // conn dropped here

    let mut synced = Vec::new();
    let mut errors = Vec::new();

    for wf_id in &wf_ids {
        let sanitized = match crate::client::sanitize_id(wf_id) {
            Some(s) => s,
            None => {
                errors.push(format!("{}: invalid workflow_id", wf_id));
                continue;
            }
        };

        // Read workflow + tasks from SQLite (sync block, no await inside)
        let sqlite_data: Option<(Value, Vec<Value>)> = {
            let conn = conn();
            (|| -> Result<(Value, Vec<Value>), rusqlite::Error> {
                let wf = conn.query_row(
                    "SELECT id, name, description, status, project_path, metadata, created_at, updated_at FROM workflows WHERE id=?1",
                    params![sanitized],
                    |row| Ok(json!({
                        "id": row.get::<_, String>(0)?,
                        "name": row.get::<_, String>(1)?,
                        "description": row.get::<_, Option<String>>(2)?,
                        "status": row.get::<_, String>(3)?,
                        "projectPath": row.get::<_, Option<String>>(4)?,
                        "metadata": json_col(row, 5),
                        "createdAt": row.get::<_, String>(6)?,
                        "updatedAt": row.get::<_, String>(7)?,
                    })),
                )?;
                let mut stmt = conn.prepare(
                    "SELECT id, title, status, owner_agent, priority, progress_percent, created_at, updated_at FROM tasks WHERE workflow_id=?1 ORDER BY created_at"
                )?;
                let rows = stmt.query_map(params![sanitized], |row| {
                    Ok(json!({
                        "id": row.get::<_, String>(0)?,
                        "title": row.get::<_, String>(1)?,
                        "status": row.get::<_, String>(2)?,
                        "ownerAgent": row.get::<_, Option<String>>(3)?,
                        "priority": row.get::<_, Option<String>>(4)?,
                        "progressPercent": row.get::<_, Option<i64>>(5)?,
                        "createdAt": row.get::<_, String>(6)?,
                        "updatedAt": row.get::<_, String>(7)?,
                    }))
                })?;
                let task_list: Vec<Value> = collect_rows(rows);
                Ok((wf, task_list))
            })().ok()
        }; // conn dropped here

        let Some((wf_data, tasks)) = sqlite_data else {
            errors.push(format!("{}: not found in SQLite", wf_id));
            continue;
        };

        // Write state file (async, but conn is already dropped)
        let state = json!({"workflow": wf_data, "tasks": tasks, "syncedAt": now()});
        let workflow_file = state_dir.join(format!("{}.json", sanitized));
        match serde_json::to_string_pretty(&state) {
            Ok(json_str) => {
                if let Err(e) = tokio::fs::write(&workflow_file, json_str).await {
                    errors.push(format!("{}: write failed: {}", wf_id, e));
                    continue;
                }
            }
            Err(e) => {
                errors.push(format!("{}: serialize failed: {}", wf_id, e));
                continue;
            }
        }
        synced.push(wf_id.clone());
    }

    // Also pull memories from PostgreSQL → SQLite
    let (mem_pulled, mem_skipped, mem_errors) = crate::direct_pg::memories_bulk_pull().await;
    let pg_failed = crate::pg::get_pool().await.is_none();

    Ok(json!({
        "synced": synced,
        "syncedCount": synced.len(),
        "errors": errors,
        "memories_pulled": mem_pulled,
        "memories_skipped": mem_skipped,
        "memory_pull_errors": mem_errors,
        "pg_sync_failed": pg_failed,
        "pg_unavailable": pg_failed,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_local_sync_invalid_workflow_id() {
        let temp_dir = TempDir::new().unwrap();
        let cwd = temp_dir.path().to_str().unwrap();

        let args = json!({
            "cwd": cwd,
            "workflow_id": "invalid/workflow"
        });

        let result = local_sync(args).await;

        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("invalid workflow_id"));
    }

    #[test]
    fn test_generate_embedding_standalone_sync() {
        let text = "standalone test";
        let embedding = generate_embedding_standalone_sync(text);

        assert!(embedding.is_some());
        let vector = embedding.unwrap();
        assert_eq!(vector.len(), 768); // Feature hashing produces 768-dim vectors
    }

    #[test]
    fn test_generate_embedding_standalone_empty() {
        let text = "";
        let embedding = generate_embedding_standalone_sync(text);

        assert!(embedding.is_some());
        let vector = embedding.unwrap();
        assert_eq!(vector.len(), 768);
        // Empty text produces zero vector
        let norm_sq: f64 = vector.iter().map(|&x| x * x).sum();
        assert_eq!(norm_sq, 0.0);
    }

    #[test]
    fn test_generate_embedding_standalone_deterministic() {
        let text = "deterministic standalone test";
        let embedding1 = generate_embedding_standalone_sync(text);
        let embedding2 = generate_embedding_standalone_sync(text);

        assert!(embedding1.is_some());
        assert!(embedding2.is_some());
        let vec1 = embedding1.unwrap();
        let vec2 = embedding2.unwrap();
        assert_eq!(vec1.len(), vec2.len());
        // Check vectors are identical
        for (i, (v1, v2)) in vec1.iter().zip(vec2.iter()).enumerate() {
            assert_eq!(v1, v2, "Vectors differ at index {}", i);
        }
    }

    #[tokio::test]
    async fn test_workflow_execute_missing_id() {
        let result = workflow_execute(json!({})).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("missing id"));
    }

    #[tokio::test]
    async fn test_workflow_execute_not_found() {
        // Ensure SQLite is initialized for the handler
        let _guard = TestDbGuard::new();

        let result = workflow_execute(json!({"id": "nonexistent-id"})).await;
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("workflow not found"));
    }

    #[tokio::test]
    async fn test_workflow_execute_from_init() {
        let guard = TestDbGuard::new();
        let (id, _) = setup_test_db_via_guard(&guard, "INIT");

        let result = workflow_execute(json!({"id": id})).await;
        assert!(result.is_ok(), "Expected ok, got {:?}", result);
        let val = result.unwrap();
        // New behavior: only advances one state at a time
        assert_eq!(val["status"], "ANALYZE");
    }

    #[tokio::test]
    async fn test_workflow_execute_from_plan() {
        let _guard = TestDbGuard::new();
        let (id, _) = setup_test_db_via_guard(&_guard, "PLAN");

        // Create a plan and tasks for the workflow (prerequisites for EXECUTE)
        {
            let conn = crate::sqlite::conn();
            let plan_id = uuid::Uuid::new_v4().to_string();
            let task_id = uuid::Uuid::new_v4().to_string();
            let t = now();

            conn.execute(
                "INSERT INTO plans (id, workflow_id, version, status, summary, content, created_by_agent, created_at) VALUES (?1,?2,1,'ACTIVE','Test plan','{}','test',?3)",
                params![&plan_id, &id, &t],
            ).unwrap();

            conn.execute(
                "INSERT INTO tasks (id, workflow_id, plan_id, title, status, created_at, updated_at) VALUES (?1,?2,?3,'Test task','PENDING',?4,?4)",
                params![&task_id, &id, &plan_id, &t],
            ).unwrap();
        } // conn dropped here, before await

        let result = workflow_execute(json!({"workflow_id": id})).await;
        assert!(result.is_ok(), "Expected ok, got {:?}", result);
        let val = result.unwrap();
        assert_eq!(val["status"], "EXECUTE");
    }

    #[tokio::test]
    async fn test_workflow_execute_from_plan_without_prerequisites() {
        let guard = TestDbGuard::new();
        let (id, _) = setup_test_db_via_guard(&guard, "PLAN");

        // Try to execute without plan or tasks - should fail
        let result = workflow_execute(json!({"workflow_id": id})).await;
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Cannot advance to EXECUTE: no plan found"));
    }

    #[tokio::test]
    async fn test_workflow_execute_from_analyze_without_prerequisites() {
        let guard = TestDbGuard::new();
        let (id, _) = setup_test_db_via_guard(&guard, "ANALYZE");

        // Try to execute without analysis artifacts - should fail
        let result = workflow_execute(json!({"workflow_id": id})).await;
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Cannot advance to PLAN: no analysis artifacts found"));
    }

    #[tokio::test]
    async fn test_workflow_execute_from_analyze_with_prerequisites() {
        let _guard = TestDbGuard::new();
        let (id, _) = setup_test_db_via_guard(&_guard, "ANALYZE");

        // Create analysis artifacts (use memory table instead of context_documents)
        {
            let conn = crate::sqlite::conn();
            let mem_id = uuid::Uuid::new_v4().to_string();
            let t = now();

            conn.execute(
                "INSERT INTO memories (id, workflow_id, memory_type, summary, content, importance_score, created_by_agent, tags, created_at, updated_at) VALUES (?1,?2,'research','Test analysis','Analysis content',0.7,'test','[]',?3,?3)",
                params![&mem_id, &id, &t],
            ).unwrap();
        } // conn dropped here, before await

        let result = workflow_execute(json!({"workflow_id": id})).await;
        assert!(result.is_ok(), "Expected ok, got {:?}", result);
        let val = result.unwrap();
        assert_eq!(val["status"], "PLAN");
    }

    #[tokio::test]
    async fn test_workflow_execute_from_done_rejected() {
        let guard = TestDbGuard::new();
        let (id, _) = setup_test_db_via_guard(&guard, "DONE");

        let result = workflow_execute(json!({"id": id})).await;
        // DONE is idempotent — returns current state, not error
        assert!(result.is_ok());
        assert_eq!(result.unwrap()["status"], "DONE");
    }

    #[tokio::test]
    async fn test_workflow_execute_from_failed_rejected() {
        let guard = TestDbGuard::new();
        let (id, _) = setup_test_db_via_guard(&guard, "FAILED");

        let result = workflow_execute(json!({"id": id})).await;
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Cannot execute workflow in state FAILED"));
    }

    #[tokio::test]
    async fn test_workflow_execute_idempotent_execute() {
        let guard = TestDbGuard::new();
        let (id, _) = setup_test_db_via_guard(&guard, "EXECUTE");

        let result = workflow_execute(json!({"id": id})).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap()["status"], "EXECUTE");
    }

    /// RAII guard that ensures SQLite is initialized exactly once.
    ///
    /// Uses a `Once` to guarantee single initialization even under parallel
    /// test execution. All tests share the same connection (UUIDs prevent data
    /// collisions). The `TempDir` keeps the database file alive for the
    /// lifetime of the process.
    struct TestDbGuard {
        _dir: tempfile::TempDir,
    }

    static TEST_ONCE: std::sync::Once = std::sync::Once::new();

    impl TestDbGuard {
        fn new() -> Self {
            let dir = tempfile::TempDir::new().unwrap();
            let db_path = dir.path().join("data.db");

            TEST_ONCE.call_once(|| {
                std::env::set_var("MASDAY_SQLITE_PATH", db_path.to_str().unwrap());
                crate::sqlite::init_sqlite().expect("SQLite init must succeed on first call");
            });

            Self { _dir: dir }
        }
    }

    fn setup_test_db_via_guard(_guard: &TestDbGuard, status: &str) -> (String, ()) {
        let conn = crate::sqlite::conn();
        let id = uuid::Uuid::new_v4().to_string();
        conn.execute(
            "INSERT INTO workflows (id, name, status) VALUES (?1, ?2, ?3)",
            params![&id, &"test-workflow", &status],
        )
        .unwrap();
        (id, ())
    }
}
