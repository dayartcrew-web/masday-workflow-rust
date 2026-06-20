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
    prd: Option<(&str, &str, &str, &str)>,
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
            match client
                .execute(
                    q,
                    &[&id, &name, &description, &project_path, &meta_json],
                )
                .await
            {
                Ok(_) => {
                    tracing::info!("Synced workflow {} to PostgreSQL", id);
                    // Ingest the project PRD as a context document (H1 gap 3).
                    // Only on workflow-insert success so we never create an
                    // orphan doc referencing a missing workflow (FK).
                    if let Some((doc_id, source_ref, title, content)) = prd {
                        let q2 = r#"
                            INSERT INTO context_documents (id, workflow_id, source_type, source_ref, title, content, metadata, fingerprint, embedding, created_at, updated_at)
                            VALUES ($1, $2, 'prd', $3, $4, $5, $6, NULL, NULL, NOW(), NOW())
                            ON CONFLICT (id) DO NOTHING
                        "#;
                        let prd_meta = serde_json::json!({ "ingested_at_create": true });
                        if let Err(e) = client
                            .execute(q2, &[&doc_id, &id, &source_ref, &title, &content, &prd_meta])
                            .await
                        {
                            tracing::warn!("PG PRD ingest sync failed {}: {}", id, e);
                        } else {
                            tracing::info!("Synced PRD context document for workflow {}", id);
                        }
                    }
                }
                Err(e) => tracing::warn!("PG workflow create sync failed {}: {}", id, e),
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

/// Mirror the FIX→EXECUTE FAILED→PENDING task reset to PostgreSQL (5s timeout).
/// In stdio/local mode `direct::workflow_update_status` resets FAILED tasks to
/// PENDING locally on a FIX→EXECUTE resume, but PG is never the source of truth
/// there (`transition_status` is not invoked), so without this the dashboard
/// keeps those tasks as FAILED after a resume. Mirrors the SQLite UPDATE in
/// `direct::workflow_update_status`.
pub async fn reset_failed_tasks(workflow_id: &str) {
    let workflow_id = workflow_id.to_string();

    let _ = tokio::time::timeout(std::time::Duration::from_secs(5), async move {
        let pool = match crate::pg::get_pool().await {
            Some(p) => p,
            None => return,
        };

        if let Ok(client) = pool.get().await {
            let q = "UPDATE tasks SET status='PENDING', updated_at=NOW() \
                     WHERE workflow_id=$1 AND status='FAILED'";
            if let Err(e) = client.execute(q, &[&workflow_id]).await {
                tracing::warn!("PG reset_failed_tasks sync failed {}: {}", workflow_id, e);
            }
        }
    })
    .await;
}

/// Mirror a stdio `review_submit` (SQLite INSERT) to PostgreSQL (5s timeout).
/// In stdio/local mode reviews land in SQLite only; the completion gate
/// (`complete_task`) reads from that same SQLite DB, so this does NOT affect
/// gating — the effect is dashboard-only (hybrid stdio+PG users see
/// stdio-submitted reviews on the PG-backed dashboard). Mirrors the SQLite
/// INSERT in `direct::review_submit` and the PG `ReviewRepo::submit` column set
/// (tests_verified/test_summary left NULL — the stdio path sets neither).
/// No-op without a pool; warn-on-err.
#[allow(clippy::too_many_arguments)]
pub async fn review_submit(
    id: &str,
    workflow_id: &str,
    task_id: &str,
    reviewer_agent: &str,
    decision: &str,
    notes: &str,
    gaps: Option<serde_json::Value>,
    created_at: &str,
) {
    let id = id.to_string();
    let workflow_id = workflow_id.to_string();
    let task_id = task_id.to_string();
    let reviewer_agent = reviewer_agent.to_string();
    let decision = decision.to_string();
    let notes = notes.to_string();
    let created_at = created_at.to_string();

    let _ = tokio::time::timeout(std::time::Duration::from_secs(5), async move {
        let pool = match crate::pg::get_pool().await {
            Some(p) => p,
            None => return,
        };

        if let Ok(client) = pool.get().await {
            // `now()` in direct.rs is RFC3339; parse to the native TIMESTAMPTZ
            // tokio-postgres expects (with-chrono-0_4 enabled). `gaps` is bound
            // natively to JSONB (with-serde_json-1 enabled), matching
            // ReviewRepo::submit — no text+cast needed.
            let created_ts: chrono::DateTime<chrono::Utc> = match created_at.parse() {
                Ok(t) => t,
                Err(_) => {
                    tracing::warn!(
                        "PG review_submit sync: unparseable created_at '{}' for {}",
                        created_at,
                        id
                    );
                    return;
                }
            };
            let q = "INSERT INTO review_decisions \
                     (id, workflow_id, task_id, reviewer_agent, decision, notes, gaps, created_at) \
                     VALUES ($1, $2, $3, $4, $5, $6, $7, $8)";
            if let Err(e) = client
                .execute(
                    q,
                    &[
                        &id,
                        &workflow_id,
                        &task_id,
                        &reviewer_agent,
                        &decision,
                        &notes,
                        &gaps,
                        &created_ts,
                    ],
                )
                .await
            {
                tracing::warn!("PG review_submit sync failed {}: {}", id, e);
            }
        }
    })
    .await;
}

/// Sync a plan row to PostgreSQL (5s timeout). Upsert by id. Plans are
/// otherwise absent from PostgreSQL in stdio/local mode, so this must run
/// before `task_create` to satisfy the `tasks.plan_id` foreign key when a
/// task is synced from local mode.
pub async fn plan_create(
    id: &str,
    workflow_id: &str,
    version: i32,
    status: &str,
    summary: &str,
    content: &str,
    created_by_agent: &str,
) {
    let id = id.to_string();
    let workflow_id = workflow_id.to_string();
    let status = status.to_string();
    let summary = summary.to_string();
    let content_json: serde_json::Value =
        serde_json::from_str(content).unwrap_or(serde_json::json!({}));
    let created_by_agent = created_by_agent.to_string();

    let _ = tokio::time::timeout(std::time::Duration::from_secs(5), async move {
        let pool = match crate::pg::get_pool().await {
            Some(p) => p,
            None => return,
        };

        if let Ok(client) = pool.get().await {
            let q = r#"
                INSERT INTO plans (id, workflow_id, version, status, summary, content, created_by_agent, created_at)
                VALUES ($1, $2, $3, $4, $5, $6, $7, NOW())
                ON CONFLICT (id) DO UPDATE SET
                    version = EXCLUDED.version, status = EXCLUDED.status,
                    summary = EXCLUDED.summary, content = EXCLUDED.content
            "#;
            if let Err(e) = client
                .execute(
                    q,
                    &[
                        &id,
                        &workflow_id,
                        &version,
                        &status,
                        &summary,
                        &content_json,
                        &created_by_agent,
                    ],
                )
                .await
            {
                tracing::warn!("PG plan create sync failed {}: {}", id, e);
            } else {
                tracing::debug!("Synced plan {} to PostgreSQL", id);
            }
        }
    })
    .await;
}

/// Sync a new task row to PostgreSQL (5s timeout). Upsert by id. Mirrors the
/// SQLite INSERT in `direct.rs::workflow_add_task`. The referenced plan must
/// already exist in PostgreSQL — callers sync it via `plan_create` first.
/// Parse a JSON-text field (arriving as a serialized JSON string from SQLite)
/// into a JSON value, dropping JSON `null` to SQL NULL. Shared by the JSONB
/// task columns (dependencies / input / acceptance_criteria / required_context).
/// Invalid JSON also collapses to SQL NULL (sync is best-effort).
fn parse_json_text(raw: Option<&str>) -> Option<serde_json::Value> {
    raw.and_then(|s| serde_json::from_str(s).ok())
        .filter(|v: &serde_json::Value| !v.is_null())
}

#[allow(clippy::too_many_arguments)]
pub async fn task_create(
    id: &str,
    workflow_id: &str,
    plan_id: &str,
    title: &str,
    status: &str,
    priority: &str,
    owner_agent: Option<&str>,
    dependencies: Option<&str>,
    progress_percent: i32,
    requires_tdd: bool,
    skill: Option<&str>,
    input: Option<&str>,
    acceptance_criteria: Option<&str>,
    required_context: Option<&str>,
    context_fingerprint: Option<&str>,
) {
    let id = id.to_string();
    let workflow_id = workflow_id.to_string();
    let plan_id = plan_id.to_string();
    let title = title.to_string();
    let status = status.to_string();
    let priority = priority.to_string();
    let owner_agent = owner_agent.map(|s| s.to_string());
    let skill = skill.map(|s| s.to_string());
    // JSON-text fields arrive serialized from SQLite (or null); normalize each
    // to a JSON value, dropping JSON nulls to SQL NULL.
    let deps_json = parse_json_text(dependencies);
    let input_json = parse_json_text(input);
    let acceptance_criteria_json = parse_json_text(acceptance_criteria);
    let required_context_json = parse_json_text(required_context);
    let context_fingerprint = context_fingerprint.map(|s| s.to_string());

    let _ = tokio::time::timeout(std::time::Duration::from_secs(5), async move {
        let pool = match crate::pg::get_pool().await {
            Some(p) => p,
            None => return,
        };

        if let Ok(client) = pool.get().await {
            let q = r#"
                INSERT INTO tasks (id, workflow_id, plan_id, title, status, priority,
                                   owner_agent, dependencies, progress_percent,
                                   requires_tdd, skill, input, acceptance_criteria,
                                   required_context, context_fingerprint, created_at, updated_at)
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, NOW(), NOW())
                ON CONFLICT (id) DO UPDATE SET
                    title = EXCLUDED.title, status = EXCLUDED.status,
                    priority = EXCLUDED.priority, owner_agent = EXCLUDED.owner_agent,
                    dependencies = EXCLUDED.dependencies,
                    progress_percent = EXCLUDED.progress_percent,
                    requires_tdd = EXCLUDED.requires_tdd, skill = EXCLUDED.skill,
                    input = EXCLUDED.input,
                    acceptance_criteria = EXCLUDED.acceptance_criteria,
                    required_context = EXCLUDED.required_context,
                    context_fingerprint = EXCLUDED.context_fingerprint, updated_at = NOW()
            "#;
            if let Err(e) = client
                .execute(
                    q,
                    &[
                        &id,
                        &workflow_id,
                        &plan_id,
                        &title,
                        &status,
                        &priority,
                        &owner_agent,
                        &deps_json,
                        &progress_percent,
                        &requires_tdd,
                        &skill,
                        &input_json,
                        &acceptance_criteria_json,
                        &required_context_json,
                        &context_fingerprint,
                    ],
                )
                .await
            {
                tracing::warn!("PG task create sync failed {}: {}", id, e);
            } else {
                tracing::info!("Synced task {} to PostgreSQL", id);
            }
        }
    })
    .await;
}

/// Sync a parallel-branch row to PostgreSQL (5s timeout). Upsert by id.
/// Mirrors the SQLite INSERT in `direct.rs::workflow_create_parallel_branches`
/// so the dashboard's branch/synthesis view reflects local branch creation —
/// branches are otherwise absent from PG in stdio/local mode. `input` is
/// JSONB NOT NULL in PG, so an absent/invalid input defaults to `{}`.
#[allow(clippy::too_many_arguments)]
pub async fn parallel_branch_create(
    id: &str,
    workflow_id: &str,
    task_id: &str,
    branch_key: &str,
    role: &str,
    status: &str,
    input: Option<&str>,
) {
    let id = id.to_string();
    let workflow_id = workflow_id.to_string();
    let task_id = task_id.to_string();
    let branch_key = branch_key.to_string();
    let role = role.to_string();
    let status = status.to_string();
    let input_json: serde_json::Value = input
        .and_then(|s| serde_json::from_str(s).ok())
        .unwrap_or_else(|| serde_json::json!({}));

    let _ = tokio::time::timeout(std::time::Duration::from_secs(5), async move {
        let pool = match crate::pg::get_pool().await {
            Some(p) => p,
            None => return,
        };
        if let Ok(client) = pool.get().await {
            let q = r#"
                INSERT INTO parallel_branches
                    (id, workflow_id, task_id, branch_key, role, status, input, created_at, updated_at)
                VALUES ($1, $2, $3, $4, $5, $6, $7, NOW(), NOW())
                ON CONFLICT (id) DO UPDATE SET
                    status = EXCLUDED.status, input = EXCLUDED.input, updated_at = NOW()
            "#;
            if let Err(e) = client
                .execute(
                    q,
                    &[
                        &id,
                        &workflow_id,
                        &task_id,
                        &branch_key,
                        &role,
                        &status,
                        &input_json,
                    ],
                )
                .await
            {
                tracing::warn!("PG parallel-branch create sync failed {}: {}", id, e);
            } else {
                tracing::debug!("Synced parallel branch {} to PostgreSQL", id);
            }
        }
    })
    .await;
}

/// Mark a parallel branch DONE in PostgreSQL (5s timeout). Mirrors the SQLite
/// UPDATE in `direct.rs::workflow_complete_parallel_branch` so PG-side
/// synthesis/VERIFY gating keyed on branch completion sees stdio branches reach
/// DONE. `output` is JSONB (nullable); an absent/invalid output becomes SQL
/// NULL.
pub async fn parallel_branch_complete(id: &str, output: Option<&str>) {
    let id = id.to_string();
    let output_json: Option<serde_json::Value> = output.and_then(|s| serde_json::from_str(s).ok());

    let _ = tokio::time::timeout(std::time::Duration::from_secs(5), async move {
        let pool = match crate::pg::get_pool().await {
            Some(p) => p,
            None => return,
        };
        if let Ok(client) = pool.get().await {
            let q = r#"
                UPDATE parallel_branches
                SET status = 'DONE', output = $1, updated_at = NOW()
                WHERE id = $2
            "#;
            if let Err(e) = client.execute(q, &[&output_json, &id]).await {
                tracing::warn!("PG parallel-branch complete sync failed {}: {}", id, e);
            } else {
                tracing::debug!("Synced parallel branch {} DONE to PostgreSQL", id);
            }
        }
    })
    .await;
}

/// Sync a task status change to PostgreSQL (5s timeout). UPDATEs an existing
/// task row (created via `task_create`); the row must already exist in PG.
/// Mirrors `workflow_status`, additionally propagating `progress_percent`,
/// `result`, and `completed_at` so a locally-completed task reads DONE
/// consistently in the API/dashboard. `completed=true` stamps `completed_at`.
pub async fn task_status(
    id: &str,
    status: &str,
    progress_percent: i32,
    result: Option<&str>,
    completed: bool,
) {
    let id = id.to_string();
    let status = status.to_string();
    // result arrives as a JSON string (or null) from SQLite; normalize JSON
    // nulls to SQL NULL.
    let result_json: Option<serde_json::Value> = result
        .and_then(|s| serde_json::from_str(s).ok())
        .filter(|v: &serde_json::Value| !v.is_null());

    let _ = tokio::time::timeout(std::time::Duration::from_secs(5), async move {
        let pool = match crate::pg::get_pool().await {
            Some(p) => p,
            None => return,
        };

        if let Ok(client) = pool.get().await {
            let q = if completed {
                r#"
                    UPDATE tasks SET status=$1, progress_percent=$2, result=$3,
                                      completed_at=NOW(), updated_at=NOW()
                    WHERE id=$4
                "#
            } else {
                r#"
                    UPDATE tasks SET status=$1, progress_percent=$2, result=$3,
                                      updated_at=NOW()
                    WHERE id=$4
                "#
            };
            if let Err(e) = client
                .execute(q, &[&status, &progress_percent, &result_json, &id])
                .await
            {
                tracing::warn!("PG task status sync failed {}: {}", id, e);
            } else {
                tracing::debug!("Synced task {} status {} to PostgreSQL", id, status);
            }
        }
    })
    .await;
}

/// Delete a workflow (and its children via cascade FKs) from PostgreSQL
/// (5s timeout). Mirrors the SQLite `DELETE FROM workflows WHERE id=?1` in
/// `direct.rs::workflow_delete`. The PG schema cascades `ON DELETE CASCADE`
/// to plans/tasks/memories/reviews/etc., so a single-row delete cleans the
/// whole workflow tree. No-op if no PG pool. Scoped to ONE workflow id —
/// never batch or project-scoped.
pub async fn workflow_delete(id: &str) {
    let id = id.to_string();

    let _ = tokio::time::timeout(std::time::Duration::from_secs(5), async move {
        let pool = match crate::pg::get_pool().await {
            Some(p) => p,
            None => return,
        };

        if let Ok(client) = pool.get().await {
            let q = "DELETE FROM workflows WHERE id=$1";
            if let Err(e) = client.execute(q, &[&id]).await {
                tracing::warn!("PG workflow delete sync failed {}: {}", id, e);
            } else {
                tracing::info!("Deleted workflow {} from PostgreSQL", id);
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

/// Delete a memory from PostgreSQL by id (5s timeout). Mirrors the SQLite
/// `DELETE FROM memories WHERE id=?1`. No-op if no PG pool. Propagating the
/// delete is what stops `memories_bulk_pull` from resurrecting a locally-
/// deleted memory (it re-inserts any PG id absent from SQLite).
pub async fn memory_delete(id: &str) {
    let id = id.to_string();
    let _ = tokio::time::timeout(std::time::Duration::from_secs(5), async move {
        let pool = match crate::pg::get_pool().await {
            Some(p) => p,
            None => return,
        };
        if let Ok(client) = pool.get().await {
            let q = "DELETE FROM memories WHERE id=$1";
            if let Err(e) = client.execute(q, &[&id]).await {
                tracing::warn!("PG memory delete sync failed {}: {}", id, e);
            } else {
                tracing::debug!("Deleted memory {} from PostgreSQL", id);
            }
        }
    })
    .await;
}

/// Delete all memories for a workflow from PostgreSQL (5s timeout). Mirrors
/// the SQLite `DELETE FROM memories WHERE workflow_id=?1`. Scoped to ONE
/// workflow id (not project-scoped). No-op if no PG pool.
pub async fn memory_delete_by_workflow(workflow_id: &str) {
    let workflow_id = workflow_id.to_string();
    let _ = tokio::time::timeout(std::time::Duration::from_secs(5), async move {
        let pool = match crate::pg::get_pool().await {
            Some(p) => p,
            None => return,
        };
        if let Ok(client) = pool.get().await {
            let q = "DELETE FROM memories WHERE workflow_id=$1";
            if let Err(e) = client.execute(q, &[&workflow_id]).await {
                tracing::warn!(
                    "PG memory delete-by-workflow sync failed {}: {}",
                    workflow_id,
                    e
                );
            } else {
                tracing::debug!(
                    "Deleted memories for workflow {} from PostgreSQL",
                    workflow_id
                );
            }
        }
    })
    .await;
}

/// Update a memory in PostgreSQL (5s timeout). Mirrors the SQLite
/// `memory_update`: updates `content` and/or `importance_score` for whichever
/// were supplied, always bumps `version` + `updated_at`. COALESCE keeps the
/// existing value when a field wasn't supplied (mirrors the SQLite handler's
/// per-field branching). No-op if no PG pool or the row is absent (e.g. a
/// memory never synced to PG). Embeddings are NOT synced here — consistent
/// with `memory_owned`/`memory` (PG embeddings are managed by the API-side
/// pipeline). Propagating the update keeps PG from drifting behind SQLite.
pub async fn memory_update(id: &str, content: Option<&str>, importance: Option<f64>) {
    let id = id.to_string();
    let content = content.map(|s| s.to_string());
    let _ = tokio::time::timeout(std::time::Duration::from_secs(5), async move {
        let pool = match crate::pg::get_pool().await {
            Some(p) => p,
            None => return,
        };
        if let Ok(client) = pool.get().await {
            let q = r#"
                UPDATE memories
                SET content = COALESCE($2, content),
                    importance_score = COALESCE($3, importance_score),
                    version = COALESCE(version, 0) + 1,
                    updated_at = NOW()
                WHERE id = $1
            "#;
            if let Err(e) = client.execute(q, &[&id, &content, &importance]).await {
                tracing::warn!("PG memory update sync failed {}: {}", id, e);
            } else {
                tracing::debug!("Updated memory {} in PostgreSQL", id);
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
        let rows = client
            .query("SELECT id FROM memories", &[])
            .await
            .unwrap_or_default();
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

/// Pull a single workflow (with tasks) from PostgreSQL.
/// Returns (workflow_data, tasks, source) for local_sync fallback.
pub async fn pull_workflow(
    workflow_id: &str,
) -> Result<(serde_json::Value, Vec<serde_json::Value>, &'static str), String> {
    let pool = match crate::pg::get_pool().await {
        Some(p) => p,
        None => return Err("No PostgreSQL pool available".into()),
    };

    let client = pool
        .get()
        .await
        .map_err(|e| format!("PG pool error: {}", e))?;

    // Query workflow
    let wf_rows = client.query(
        "SELECT id, name, description, status, project_path, metadata::text, created_at::text, updated_at::text FROM workflows WHERE id=$1",
        &[&workflow_id],
    ).await.map_err(|e| format!("PG query error: {}", e))?;

    let wf_row = wf_rows
        .into_iter()
        .next()
        .ok_or_else(|| format!("Workflow {} not found in PostgreSQL", workflow_id))?;

    let wf = serde_json::json!({
        "id": wf_row.get::<_, String>(0),
        "name": wf_row.get::<_, String>(1),
        "description": wf_row.get::<_, Option<String>>(2),
        "status": wf_row.get::<_, String>(3),
        "projectPath": wf_row.get::<_, Option<String>>(4),
        "metadata": wf_row.get::<_, Option<String>>(5).and_then(|m| serde_json::from_str(&m).ok()).unwrap_or(serde_json::json!({})),
        "createdAt": wf_row.get::<_, String>(6),
        "updatedAt": wf_row.get::<_, String>(7),
    });

    // Query tasks
    let task_rows = client.query(
        "SELECT id, title, status, owner_agent, priority, progress_percent, created_at::text, updated_at::text FROM tasks WHERE workflow_id=$1 ORDER BY created_at",
        &[&workflow_id],
    ).await.map_err(|e| format!("PG tasks query error: {}", e))?;

    let tasks: Vec<serde_json::Value> = task_rows
        .iter()
        .map(|row| {
            serde_json::json!({
                "id": row.get::<_, String>(0),
                "title": row.get::<_, String>(1),
                "status": row.get::<_, String>(2),
                "ownerAgent": row.get::<_, Option<String>>(3),
                "priority": row.get::<_, Option<String>>(4),
                "progressPercent": row.get::<_, Option<i64>>(5),
                "createdAt": row.get::<_, String>(6),
                "updatedAt": row.get::<_, String>(7),
            })
        })
        .collect();

    // Also insert into SQLite for future lookups
    {
        let conn = crate::sqlite::conn();
        let now = chrono::Utc::now().to_rfc3339();
        let _ = conn.execute(
            "INSERT OR IGNORE INTO workflows (id, name, description, status, project_path, metadata, created_at, updated_at) VALUES (?1,?2,?3,?4,?5,?6,?7,?8)",
            rusqlite::params![
                wf["id"].as_str().unwrap_or(""),
                wf["name"].as_str().unwrap_or(""),
                wf["description"].as_str().unwrap_or(""),
                wf["status"].as_str().unwrap_or("INIT"),
                wf["projectPath"].as_str().unwrap_or(""),
                wf["metadata"].to_string(),
                wf["createdAt"].as_str().unwrap_or(&now),
                wf["updatedAt"].as_str().unwrap_or(&now),
            ],
        );
        for t in &tasks {
            let _ = conn.execute(
                "INSERT OR IGNORE INTO tasks (id, workflow_id, title, status, owner_agent, priority, progress_percent, created_at, updated_at) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9)",
                rusqlite::params![
                    t["id"].as_str().unwrap_or(""),
                    workflow_id,
                    t["title"].as_str().unwrap_or(""),
                    t["status"].as_str().unwrap_or("PENDING"),
                    t["ownerAgent"].as_str().unwrap_or(""),
                    t["priority"].as_str().unwrap_or(""),
                    t["progressPercent"].as_i64(),
                    t["createdAt"].as_str().unwrap_or(&now),
                    t["updatedAt"].as_str().unwrap_or(&now),
                ],
            );
        }
    }

    Ok((wf, tasks, "postgresql"))
}

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

#[cfg(test)]
mod tests {
    use super::parse_json_text;
    use serde_json::json;

    #[test]
    fn parse_json_text_none_for_absent() {
        assert_eq!(parse_json_text(None), None);
    }

    #[test]
    fn parse_json_text_drops_json_null() {
        // A literal JSON null must collapse to SQL NULL (None), not a stored null.
        assert_eq!(parse_json_text(Some("null")), None);
    }

    #[test]
    fn parse_json_text_parses_object() {
        let v = parse_json_text(Some(r#"{"dependencies":["t1","t2"]}"#));
        assert_eq!(v, Some(json!({"dependencies": ["t1", "t2"]})));
    }

    #[test]
    fn parse_json_text_parses_array() {
        let v = parse_json_text(Some(r#"["a","b"]"#));
        assert_eq!(v, Some(json!(["a", "b"])));
    }

    #[test]
    fn parse_json_text_invalid_json_is_none() {
        // Malformed JSON collapses to None (sync is best-effort).
        assert_eq!(parse_json_text(Some("not-json")), None);
    }

    #[test]
    fn parse_json_text_preserves_string_value() {
        // A JSON string value is a valid value, not null — keep it.
        assert_eq!(parse_json_text(Some(r#""hello""#)), Some(json!("hello")));
    }
}
