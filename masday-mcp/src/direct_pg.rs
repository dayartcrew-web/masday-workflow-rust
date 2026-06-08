//! PostgreSQL sync functions for MCP local mode.
//!
//! These functions sync data from SQLite to PostgreSQL on-demand.
//! Called after SQLite writes succeed. If PostgreSQL is not configured,
//! all functions return silently (no-op).
//!
//! All functions are wrapped in timeouts to prevent blocking the MCP tool response.

// ─── Workflow Sync ──────────────────────────────────────────────────

/// Sync a new workflow to PostgreSQL (5s timeout).
pub async fn workflow_create(
    id: &str,
    name: &str,
    description: Option<&str>,
    project_path: Option<&str>,
    metadata: &str,
) {
    let id = id.to_string();
    let name = name.to_string();
    let description = description.map(|s| s.to_string());
    let project_path = project_path.map(|s| s.to_string());
    let metadata = metadata.to_string();

    let _ = tokio::time::timeout(std::time::Duration::from_secs(5), async move {
        let pool = match crate::pg::get_pool().await {
            Some(p) => p,
            None => return,
        };

        // Parse metadata as JSON for proper jsonb casting
        let meta_json: serde_json::Value =
            serde_json::from_str(&metadata).unwrap_or(serde_json::json!({}));

        if let Ok(client) = pool.get().await {
            let q = r#"
                INSERT INTO workflows (id, name, description, status, project_path, metadata, created_at, updated_at)
                VALUES ($1, $2, $3, 'INIT', $4, $5, NOW(), NOW())
                ON CONFLICT (id) DO UPDATE SET
                    name = EXCLUDED.name, description = EXCLUDED.description,
                    metadata = EXCLUDED.metadata, updated_at = NOW()
            "#;
            if let Err(e) = client
                .execute(
                    q,
                    &[&id, &name, &description, &project_path, &meta_json],
                )
                .await
            {
                tracing::warn!("PG workflow create sync failed {}: {}", id, e);
            } else {
                tracing::info!("Synced workflow {} to PostgreSQL", id);
            }
        }
    })
    .await;
}

/// Sync workflow status change to PostgreSQL (5s timeout).
pub async fn workflow_status(id: &str, status: &str) {
    let id = id.to_string();
    let status = status.to_string();

    let _ = tokio::time::timeout(std::time::Duration::from_secs(5), async move {
        let pool = match crate::pg::get_pool().await {
            Some(p) => p,
            None => return,
        };

        if let Ok(client) = pool.get().await {
            let q = "UPDATE workflows SET status=$1, updated_at=NOW() WHERE id=$2";
            if let Err(e) = client.execute(q, &[&status, &id]).await {
                tracing::warn!("PG workflow status sync failed {}: {}", id, e);
            }
        }
    })
    .await;
}

/// Bulk sync workflows from SQLite to PostgreSQL (15s timeout for bulk).
/// Reads from SQLite synchronously, then writes to PostgreSQL.
pub async fn workflows_bulk(workflow_ids: &[String]) -> bool {
    let workflow_ids = workflow_ids.to_vec();

    tokio::time::timeout(std::time::Duration::from_secs(15), async move {
        let pool = match crate::pg::get_pool().await {
            Some(p) => p,
            None => return false,
        };

        // Read from SQLite (synchronous — no MutexGuard across await)
        let workflows = read_sqlite_workflows(&workflow_ids);

        let mut synced = 0;
        for (id, name, desc, status, path, metadata) in workflows {
            let meta_json: serde_json::Value =
                serde_json::from_str(&metadata).unwrap_or(serde_json::json!({}));
            if let Ok(client) = pool.get().await {
                let q = r#"
                    INSERT INTO workflows (id, name, description, status, project_path, metadata, created_at, updated_at)
                    VALUES ($1, $2, $3, $4, $5, $6, NOW(), NOW())
                    ON CONFLICT (id) DO UPDATE SET
                        name = EXCLUDED.name, description = EXCLUDED.description,
                        status = EXCLUDED.status, project_path = EXCLUDED.project_path,
                        metadata = EXCLUDED.metadata, updated_at = NOW()
                "#;
                match client
                    .execute(q, &[&id, &name, &desc, &status, &path, &meta_json])
                    .await
                {
                    Ok(_) => synced += 1,
                    Err(e) => tracing::warn!("PG workflow sync failed {}: {}", id, e),
                }
            }
        }
        tracing::info!(
            "Synced {}/{} workflows to PostgreSQL",
            synced,
            workflow_ids.len()
        );
        true
    })
    .await
    .unwrap_or(false)
}

// ─── Memory Sync ────────────────────────────────────────────────────

/// Sync a memory record to PostgreSQL (owned strings, 5s timeout).
pub async fn memory_owned(
    data: (
        String,
        Option<String>,
        Option<String>,
        String,
        String,
        String,
        f64,
        String,
        String,
    ),
) {
    let _ = tokio::time::timeout(std::time::Duration::from_secs(5), async move {
        let pool = match crate::pg::get_pool().await {
            Some(p) => p,
            None => return,
        };

        let (id, workflow_id, task_id, memory_type, summary, content, importance, created_by, tags) =
            data;

        if let Ok(client) = pool.get().await {
            let tags_list: Vec<String> =
                serde_json::from_str::<Vec<String>>(&tags).unwrap_or_default();
            let q = r#"
                INSERT INTO memories (id, workflow_id, task_id, memory_type, summary, content,
                                      importance_score, created_by_agent, tags, created_at, updated_at)
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, NOW(), NOW())
                ON CONFLICT (id) DO UPDATE SET
                    summary = EXCLUDED.summary, content = EXCLUDED.content,
                    importance_score = EXCLUDED.importance_score, updated_at = NOW()
            "#;
            if let Err(e) = client
                .execute(
                    q,
                    &[
                        &id,
                        &workflow_id,
                        &task_id,
                        &memory_type,
                        &summary,
                        &content,
                        &importance,
                        &created_by,
                        &tags_list,
                    ],
                )
                .await
            {
                tracing::warn!("PG memory sync failed {}: {}", id, e);
            } else {
                tracing::info!("Synced memory {} to PostgreSQL", id);
            }
        }
    })
    .await;
}

/// Memory data for PG sync (borrowed, for direct calls).
pub struct MemoryData<'a> {
    pub id: &'a str,
    pub workflow_id: Option<&'a str>,
    pub task_id: Option<&'a str>,
    pub memory_type: &'a str,
    pub summary: &'a str,
    pub content: &'a str,
    pub importance: f64,
    pub created_by: &'a str,
    pub tags: &'a str,
}

/// Sync a memory record to PostgreSQL (borrowed, 5s timeout).
pub async fn memory(data: &MemoryData<'_>) {
    let _ = tokio::time::timeout(std::time::Duration::from_secs(5), async {
        let pool = match crate::pg::get_pool().await {
            Some(p) => p,
            None => return,
        };

        if let Ok(client) = pool.get().await {
            let tags_list: Vec<String> =
                serde_json::from_str::<Vec<String>>(data.tags).unwrap_or_default();
            let q = r#"
                INSERT INTO memories (id, workflow_id, task_id, memory_type, summary, content,
                                      importance_score, created_by_agent, tags, created_at, updated_at)
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, NOW(), NOW())
                ON CONFLICT (id) DO UPDATE SET
                    summary = EXCLUDED.summary, content = EXCLUDED.content,
                    importance_score = EXCLUDED.importance_score, updated_at = NOW()
            "#;
            if let Err(e) = client
                .execute(
                    q,
                    &[
                        &data.id,
                        &data.workflow_id,
                        &data.task_id,
                        &data.memory_type,
                        &data.summary,
                        &data.content,
                        &data.importance,
                        &data.created_by,
                        &tags_list,
                    ],
                )
                .await
            {
                tracing::warn!("PG memory sync failed {}: {}", data.id, e);
            } else {
                tracing::debug!("Synced memory {} to PostgreSQL", data.id);
            }
        }
    })
    .await;
}

// ─── Bulk Memory Sync ────────────────────────────────────────────────

/// Bulk push memories from SQLite to PostgreSQL.
/// Only pushes memories that don't exist in PostgreSQL (by id).
/// Limited to MAX_BATCH_PER_PUSH per call to avoid timeouts.
/// Returns count of synced memories.
const MAX_BATCH_PER_PUSH: usize = 200;

pub async fn memories_bulk_push() -> (usize, usize, Vec<String>) {
    // Pool should already be initialized at startup in run_local()
    let pool = match crate::pg::get_pool().await {
        Some(p) => p,
        None => return (0, 0, vec!["No PostgreSQL pool".into()]),
    };

    // Read all memories from SQLite, limit to batch size
    let all_memories = read_sqlite_memories();
    let remaining = if all_memories.len() > MAX_BATCH_PER_PUSH {
        tracing::info!(
            "Pushing {}/{} memories this batch (remaining will sync next call)",
            MAX_BATCH_PER_PUSH,
            all_memories.len()
        );
        all_memories.len() - MAX_BATCH_PER_PUSH
    } else {
        0
    };
    let memories: Vec<_> = all_memories.into_iter().take(MAX_BATCH_PER_PUSH).collect();

    if memories.is_empty() {
        return (0, 0, vec![]);
    }

    // Get existing memory IDs from PostgreSQL to skip
    let existing_ids = {
        let client = match pool.get().await {
            Ok(c) => c,
            Err(e) => return (0, memories.len(), vec![format!("PG client error: {}", e)]),
        };
        let rows = client.query("SELECT id FROM memories", &[]).await.unwrap_or_default();
        rows.iter()
            .filter_map(|r| {
                let id: Result<String, _> = r.try_get(0);
                id.ok()
            })
            .collect::<std::collections::HashSet<_>>()
    };

    let mut synced = 0usize;
    let mut skipped = 0usize;
    let mut errors = Vec::new();

    if let Ok(client) = pool.get().await {
        for (
            id,
            workflow_id,
            task_id,
            memory_type,
            summary,
            content,
            importance,
            created_by,
            tags,
        ) in &memories
        {
            // Skip if already exists in PostgreSQL
            if existing_ids.contains(id) {
                skipped += 1;
                continue;
            }

            let tags_list: Vec<String> =
                serde_json::from_str::<Vec<String>>(tags).unwrap_or_default();
            let q = r#"
                INSERT INTO memories (id, workflow_id, task_id, memory_type, summary, content,
                                      importance_score, created_by_agent, tags, created_at, updated_at)
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, NOW(), NOW())
                ON CONFLICT (id) DO UPDATE SET
                    summary = EXCLUDED.summary, content = EXCLUDED.content,
                    importance_score = EXCLUDED.importance_score, updated_at = NOW()
            "#;
            match client
                .execute(
                    q,
                    &[
                        id,
                        workflow_id,
                        task_id,
                        memory_type,
                        summary,
                        content,
                        importance,
                        created_by,
                        &tags_list,
                    ],
                )
                .await
            {
                Ok(_) => synced += 1,
                Err(e) => {
                    errors.push(format!("{}: {}", id, e));
                }
            }
        }
    }

    tracing::info!(
        "Bulk pushed {}/{} memories to PostgreSQL ({} skipped, {} remaining for next sync)",
        synced,
        memories.len(),
        skipped,
        remaining
    );
    (synced, skipped, errors)
}

/// Bulk pull memories from PostgreSQL into SQLite.
/// Only inserts memories that don't exist in SQLite (by id).
/// Returns count of pulled memories.
pub async fn memories_bulk_pull() -> (usize, usize, Vec<String>) {
    // Pool should already be initialized at startup in run_local()
    let pool = match crate::pg::get_pool().await {
        Some(p) => p,
        None => return (0, 0, vec!["No PostgreSQL pool".into()]),
    };

    let client = match pool.get().await {
        Ok(c) => c,
        Err(e) => return (0, 0, vec![format!("PG client error: {}", e)]),
    };

    // Read all memories from PostgreSQL
    let rows = match client
        .query(
            "SELECT id, workflow_id, task_id, memory_type, summary, content,
                    importance_score, created_by_agent, tags
             FROM memories ORDER BY created_at",
            &[],
        )
        .await
    {
        Ok(r) => r,
        Err(e) => return (0, 0, vec![format!("PG query error: {}", e)]),
    };

    if rows.is_empty() {
        return (0, 0, vec![]);
    }

    // Get existing memory IDs from SQLite
    let sqlite_ids: std::collections::HashSet<String> = {
        let conn = crate::sqlite::conn();
        let mut stmt = conn.prepare("SELECT id FROM memories").unwrap();
        let ids: Vec<String> = stmt
            .query_map([], |row| row.get(0))
            .unwrap()
            .filter_map(|r| r.ok())
            .collect();
        ids.into_iter().collect()
    };

    let conn = crate::sqlite::conn();
    let mut pulled = 0usize;
    let mut skipped = 0usize;
    let mut errors = Vec::new();

    for row in &rows {
        let id: String = match row.try_get(0) {
            Ok(s) => s,
            Err(_) => continue,
        };
        if id.is_empty() {
            continue;
        }

        // Skip if already exists in SQLite
        if sqlite_ids.contains(&id) {
            skipped += 1;
            continue;
        }

        let workflow_id: Option<String> = row.try_get(1).ok().flatten();
        let task_id: Option<String> = row.try_get(2).ok().flatten();
        let memory_type: String = row.try_get(3).unwrap_or_else(|_| "fact".into());
        let summary: String = row.try_get(4).unwrap_or_default();
        let content: String = row.try_get(5).unwrap_or_default();
        let importance: f64 = row.try_get(6).unwrap_or(0.5);
        let created_by: String = row.try_get(7).unwrap_or_else(|_| "system".into());
        let tags_json: String = match row.try_get::<_, Option<Vec<String>>>(8) {
            Ok(Some(v)) => serde_json::to_string(&v).unwrap_or_else(|_| "[]".into()),
            _ => "[]".into(),
        };

        let now_str = pg_now();

        let result = if workflow_id.is_none() && task_id.is_none() {
            conn.execute(
                "INSERT OR IGNORE INTO memories (id, workflow_id, task_id, memory_type, summary, content, importance_score, created_by_agent, tags, created_at, updated_at) VALUES (?1, NULL, NULL, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?8)",
                rusqlite::params![id, memory_type, summary, content, importance, created_by, tags_json, now_str],
            )
        } else {
            conn.execute(
                "INSERT OR IGNORE INTO memories (id, workflow_id, task_id, memory_type, summary, content, importance_score, created_by_agent, tags, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?10)",
                rusqlite::params![id, workflow_id, task_id, memory_type, summary, content, importance, created_by, tags_json, now_str],
            )
        };

        match result {
            Ok(_) => pulled += 1,
            Err(e) => errors.push(format!("{}: {}", id, e)),
        }
    }

    tracing::info!(
        "Bulk pulled {}/{} memories from PostgreSQL ({} skipped, already exist)",
        pulled,
        rows.len(),
        skipped
    );
    (pulled, skipped, errors)
}

// ─── Helpers ────────────────────────────────────────────────────────

#[allow(clippy::type_complexity)]
type WorkflowRow = (
    String,
    String,
    Option<String>,
    String,
    Option<String>,
    String,
);

/// Read workflows from SQLite synchronously.
fn read_sqlite_workflows(workflow_ids: &[String]) -> Vec<WorkflowRow> {
    let conn = crate::sqlite::conn();
    let mut results = Vec::new();
    for wid in workflow_ids {
        let r: Result<WorkflowRow, _> = conn.query_row(
            "SELECT id, name, COALESCE(description,''), status, project_path, COALESCE(metadata,'{}') FROM workflows WHERE id=?1",
            rusqlite::params![wid],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?, row.get(5)?)),
        );
        if let Ok(data) = r {
            results.push(data);
        }
    }
    results
}

/// Read all memories from SQLite synchronously.
#[allow(clippy::type_complexity)]
type MemoryRow = (
    String,
    Option<String>,
    Option<String>,
    String,
    String,
    String,
    f64,
    String,
    String,
);

fn read_sqlite_memories() -> Vec<MemoryRow> {
    let conn = crate::sqlite::conn();
    let mut results = Vec::new();
    let mut stmt = match conn.prepare(
        "SELECT id, workflow_id, task_id, memory_type, summary, content,
                importance_score, created_by_agent, COALESCE(tags,'[]')
         FROM memories ORDER BY created_at",
    ) {
        Ok(s) => s,
        Err(_) => return results,
    };

    let rows = match stmt.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, Option<String>>(1)?,
            row.get::<_, Option<String>>(2)?,
            row.get::<_, String>(3)?,
            row.get::<_, String>(4)?,
            row.get::<_, String>(5)?,
            row.get::<_, f64>(6)?,
            row.get::<_, String>(7)?,
            row.get::<_, String>(8)?,
        ))
    }) {
        Ok(r) => r,
        Err(_) => return results,
    };

    for data in rows.flatten() {
        results.push(data);
    }
    results
}

/// Get current timestamp string.
fn pg_now() -> String {
    chrono::Utc::now().to_rfc3339()
}
