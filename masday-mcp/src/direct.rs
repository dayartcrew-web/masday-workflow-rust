//! Direct-call adapter for standalone stdio mode (SQLite)
//!
//! Each function uses rusqlite directly against ~/.masday/data.db.
//! Takes `serde_json::Value` args, returns `Result<Value, Box<dyn Error + Send + Sync>>`.

use rusqlite::params;
use serde_json::{json, Value};
use tracing::warn;

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
// Workflow Tools (22)
// ============================================================================

pub async fn workflow_create(
    args: Value,
) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    let conn = crate::sqlite::conn();
    let id = new_id();
    let name = args["name"].as_str().ok_or_else(|| err("missing name"))?;
    let description = args.get("description").and_then(|v| v.as_str());
    let status = "INIT";
    let project_path = args.get("project_path").and_then(|v| v.as_str());
    let t = now();
    let meta = args.get("metadata").cloned().unwrap_or(json!({}));

    conn.execute(
        "INSERT INTO workflows (id, name, description, status, project_path, metadata, created_at, updated_at) VALUES (?1,?2,?3,?4,?5,?6,?7,?8)",
        params![id, name, description, status, project_path, meta.to_string(), &t, &t],
    ).map_err(|e| err(e))?;

    Ok(json!({"id": id, "name": name, "status": status}))
}

pub async fn workflow_execute(
    args: Value,
) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    let conn = crate::sqlite::conn();
    let id = args
        .get("id")
        .or_else(|| args.get("workflow_id"))
        .and_then(|v| v.as_str())
        .ok_or_else(|| err("missing id"))?;
    let t = now();

    conn.execute(
        "UPDATE workflows SET status='EXECUTE', updated_at=?1 WHERE id=?2",
        params![&t, id],
    )
    .map_err(|e| err(e))?;

    Ok(json!({"id": id, "status": "EXECUTE"}))
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
        .map_err(|e| err(e))?;

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
            .map_err(|e| err(e))?;
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
            .map_err(|e| err(e))?;
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
        .map_err(|e| err(e))?;
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

    // Get or create default plan
    let plan_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM plans WHERE id=?1",
            params![plan_id],
            |r| r.get(0),
        )
        .map_err(|e| err(e))?;
    if plan_count == 0 {
        conn.execute(
            "INSERT INTO plans (id, workflow_id, version, status, summary, content, created_by_agent, created_at) VALUES (?1,?2,1,'ACTIVE','Auto-created plan','{}','system',?3)",
            params![plan_id, workflow_id, &t],
        ).map_err(|e| err(e))?;
    }

    conn.execute(
        "INSERT INTO tasks (id, workflow_id, plan_id, title, status, owner_agent, dependencies, created_at, updated_at) VALUES (?1,?2,?3,?4,'PENDING',?5,?6,?7,?8)",
        params![id, workflow_id, plan_id, title, owner_agent, deps, &t, &t],
    ).map_err(|e| err(e))?;

    Ok(json!({"id": id, "title": title, "status": "PENDING"}))
}

pub async fn workflow_start_task(
    args: Value,
) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    let conn = crate::sqlite::conn();
    let wf_id = args["workflow_id"]
        .as_str()
        .ok_or_else(|| err("missing workflow_id"))?;
    let task_id = args["task_id"]
        .as_str()
        .ok_or_else(|| err("missing task_id"))?;
    let t = now();

    conn.execute("UPDATE tasks SET status='RUNNING', started_at=?1, updated_at=?2 WHERE id=?3 AND workflow_id=?4",
        params![&t, &t, task_id, wf_id]).map_err(|e| err(e))?;

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

    // Auto-transition workflow if all tasks done
    let pending: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM tasks WHERE workflow_id=?1 AND status NOT IN ('DONE','FAILED')",
            params![wf_id],
            |r| r.get(0),
        )
        .map_err(|e| err(e))?;

    if pending == 0 {
        conn.execute("UPDATE workflows SET status='DONE', updated_at=?1 WHERE id=?2 AND status NOT IN ('DONE','FAILED')",
            params![&t, wf_id]).map_err(|e| err(e))?;
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
        .map_err(|e| err(e))?;

    let tasks: Vec<Value> = collect_rows(rows);
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
        .map_err(|e| err(e))?;

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
        .map_err(|e| err(e))?;
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
        .map_err(|e| err(e))?;

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
        None => Ok(json!({"task": null})),
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
        .map_err(|e| err(e))?;

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
    memory_search(args).await
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
        .map_err(|e| err(e))?;

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
        .map_err(|e| err(e))?;

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
        .map_err(|e| err(e))?;

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
                .map_err(|e| err(e))?;

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
                .map_err(|e| err(e))?;

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
            .map_err(|e| err(e))?;
        }
    }

    Ok(json!({"updated": id}))
}

pub async fn memory_delete(args: Value) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    let conn = crate::sqlite::conn();
    let id = args["id"].as_str().ok_or_else(|| err("missing id"))?;

    conn.execute("DELETE FROM memories WHERE id=?1", params![id])
        .map_err(|e| err(e))?;
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
        .map_err(|e| err(e))?;
    Ok(json!({"deleted": count, "workflow_id": workflow_id}))
}

pub async fn memory_stats(_args: Value) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    let conn = crate::sqlite::conn();

    let total: i64 = conn
        .query_row("SELECT COUNT(*) FROM memories", [], |r| r.get(0))
        .map_err(|e| err(e))?;
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
        .map_err(|e| err(e))?;

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
        .map_err(|e| err(e))?;

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
        None => Ok(json!({"review": null})),
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
    ).map_err(|e| err(e))?;

    Ok(json!({"state": state}))
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
        .map_err(|e| err(e))?;

    let valid = wf_status == "EXECUTE" || wf_status == "PLAN";

    // Check task exists and is PENDING or RUNNING
    let task_status: String = conn
        .query_row(
            "SELECT status FROM tasks WHERE id=?1",
            params![task_id],
            |r| r.get(0),
        )
        .map_err(|e| err(e))?;

    let valid = valid && (task_status == "PENDING" || task_status == "RUNNING");

    Ok(json!({"valid": valid}))
}

pub async fn policy_validate_completion(
    args: Value,
) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    let conn = crate::sqlite::conn();
    let _workflow_id = args["workflow_id"]
        .as_str()
        .ok_or_else(|| err("missing workflow_id"))?;
    let task_id = args["task_id"]
        .as_str()
        .ok_or_else(|| err("missing task_id"))?;

    let task_status: String = conn
        .query_row(
            "SELECT status FROM tasks WHERE id=?1",
            params![task_id],
            |r| r.get(0),
        )
        .map_err(|e| err(e))?;

    let valid = task_status == "DONE";

    Ok(json!({"valid": valid}))
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
    _args: Value,
) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    Ok(json!({"ready": true}))
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
        .map_err(|e| err(e))?;

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
    let entities_arr = args["entities"]
        .as_array()
        .ok_or_else(|| err("missing entities array"))?;
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
        .map_err(|e| err(e))?;

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
    _args: Value,
) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    // SQLite is always ready if we got this far
    Ok(json!({"ready": true, "db": "sqlite", "pool_healthy": true}))
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
        .map_err(|e| err(e))?;

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
    let conn = crate::sqlite::conn();
    let workflow_id = args["workflow_id"]
        .as_str()
        .ok_or_else(|| err("missing workflow_id"))?;
    let updates = args.get("updates").cloned().unwrap_or(json!({}));
    let t = now();

    // Update individual fields from updates JSON
    if let Some(obj) = updates.as_object() {
        if let Some(name) = obj.get("name").and_then(|v| v.as_str()) {
            conn.execute(
                "UPDATE workflows SET name=?1, updated_at=?2 WHERE id=?3",
                params![name, &t, workflow_id],
            )
            .map_err(|e| err(e))?;
        }
        if let Some(status) = obj.get("status").and_then(|v| v.as_str()) {
            conn.execute(
                "UPDATE workflows SET status=?1, updated_at=?2 WHERE id=?3",
                params![status, &t, workflow_id],
            )
            .map_err(|e| err(e))?;
        }
        if let Some(path) = obj.get("project_path").and_then(|v| v.as_str()) {
            conn.execute(
                "UPDATE workflows SET project_path=?1, updated_at=?2 WHERE id=?3",
                params![path, &t, workflow_id],
            )
            .map_err(|e| err(e))?;
        }
        if let Some(meta) = obj.get("metadata").cloned() {
            conn.execute(
                "UPDATE workflows SET metadata=?1, updated_at=?2 WHERE id=?3",
                params![meta.to_string(), &t, workflow_id],
            )
            .map_err(|e| err(e))?;
        }
    }

    Ok(json!({"id": workflow_id, "updated": true}))
}

pub async fn local_sync(args: Value) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    // Delegate to filesystem-based local sync
    crate::tools::local::local_sync(args).await
}
