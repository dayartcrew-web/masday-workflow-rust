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
