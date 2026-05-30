//! Session state repository

use crate::schema::SessionState;
use deadpool_postgres::Pool;
use masday_core::{AppError, Result};
use tracing::debug;

pub struct SessionRepo {
    pool: Pool,
}

impl SessionRepo {
    pub fn new(pool: Pool) -> Self {
        Self { pool }
    }

    /// Get session state by session key
    pub async fn get_state(&self, session_key: &str) -> Result<Option<SessionState>> {
        let client = self.pool.get().await
            .map_err(|e| AppError::Database(format!("Failed to get connection: {}", e)))?;

        let query = "SELECT * FROM session_states WHERE session_key = $1";
        let rows = client.query(query, &[&session_key]).await
            .map_err(|e| AppError::Database(format!("Failed to get session state: {}", e)))?;

        if rows.is_empty() {
            return Ok(None);
        }

        let row = &rows[0];
        Ok(Some(SessionState {
            id: row.get("id"),
            session_key: row.get("session_key"),
            workflow_id: row.get("workflow_id"),
            plan_id: row.get("plan_id"),
            task_id: row.get("task_id"),
            workflow_loaded: row.get("workflow_loaded"),
            plan_loaded: row.get("plan_loaded"),
            task_loaded: row.get("task_loaded"),
            context_loaded: row.get("context_loaded"),
            review_approved: row.get("review_approved"),
            context_fingerprint: row.get("context_fingerprint"),
            execution_mode: row.get("execution_mode"),
            active_branch_ids: row.try_get("active_branch_ids").unwrap_or(None),
            synthesis_ready: row.get("synthesis_ready"),
            verification_ready: row.get("verification_ready"),
            last_command: row.get("last_command"),
            metadata: row.try_get("metadata").unwrap_or(None),
            updated_at: row.get("updated_at"),
            created_at: row.get("created_at"),
        }))
    }

    /// Update session state with patch (upsert)
    pub async fn patch_state(&self, session_key: &str, patch: serde_json::Value) -> Result<SessionState> {
        let client = self.pool.get().await
            .map_err(|e| AppError::Database(format!("Failed to get connection: {}", e)))?;

        let now: chrono::DateTime<chrono::Utc> = chrono::Utc::now();

        // First check if session exists
        let existing_query = "SELECT * FROM session_states WHERE session_key = $1";
        let existing_rows = client.query(existing_query, &[&session_key]).await
            .map_err(|e| AppError::Database(format!("Failed to check existing session: {}", e)))?;

        if existing_rows.is_empty() {
            // Create new session state
            let id = uuid::Uuid::new_v4().to_string();

            let query = "
                INSERT INTO session_states (
                    id, session_key, workflow_id, plan_id, task_id,
                    workflow_loaded, plan_loaded, task_loaded, context_loaded,
                    review_approved, context_fingerprint, execution_mode,
                    active_branch_ids, synthesis_ready, verification_ready,
                    last_command, metadata, created_at, updated_at
                )
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19)
                RETURNING *
            ";

            let row = client.query_one(
                query,
                &[
                    &id,
                    &session_key,
                    &patch.get("workflow_id").and_then(|v| v.as_str()),
                    &patch.get("plan_id").and_then(|v| v.as_str()),
                    &patch.get("task_id").and_then(|v| v.as_str()),
                    &patch.get("workflow_loaded").and_then(|v| v.as_bool()),
                    &patch.get("plan_loaded").and_then(|v| v.as_bool()),
                    &patch.get("task_loaded").and_then(|v| v.as_bool()),
                    &patch.get("context_loaded").and_then(|v| v.as_bool()),
                    &patch.get("review_approved").and_then(|v| v.as_bool()),
                    &patch.get("context_fingerprint").and_then(|v| v.as_str()),
                    &patch.get("execution_mode").and_then(|v| v.as_str()),
                    &patch.get("active_branch_ids"),
                    &patch.get("synthesis_ready").and_then(|v| v.as_bool()),
                    &patch.get("verification_ready").and_then(|v| v.as_bool()),
                    &patch.get("last_command").and_then(|v| v.as_str()),
                    &patch.get("metadata"),
                    &now,
                    &now,
                ],
            ).await.map_err(|e| AppError::Database(format!("Failed to create session state: {}", e)))?;

            return Ok(SessionState {
                id: row.get("id"),
                session_key: row.get("session_key"),
                workflow_id: row.get("workflow_id"),
                plan_id: row.get("plan_id"),
                task_id: row.get("task_id"),
                workflow_loaded: row.get("workflow_loaded"),
                plan_loaded: row.get("plan_loaded"),
                task_loaded: row.get("task_loaded"),
                context_loaded: row.get("context_loaded"),
                review_approved: row.get("review_approved"),
                context_fingerprint: row.get("context_fingerprint"),
                execution_mode: row.get("execution_mode"),
                active_branch_ids: row.try_get("active_branch_ids").unwrap_or(None),
                synthesis_ready: row.get("synthesis_ready"),
                verification_ready: row.get("verification_ready"),
                last_command: row.get("last_command"),
                metadata: row.try_get("metadata").unwrap_or(None),
                updated_at: row.get("updated_at"),
                created_at: row.get("created_at"),
            });
        }

        // Update existing session - build dynamic UPDATE
        let mut set_clauses = vec!["updated_at = $2".to_string()];
        let mut param_count = 2;
        let mut params: Vec<Box<dyn tokio_postgres::types::ToSql + Sync>> = vec![
            Box::new(session_key.to_string()),
            Box::new(now)
        ];

        for (key, value) in patch.as_object().unwrap_or(&serde_json::Map::new()) {
            param_count += 1;
            set_clauses.push(format!("{} = ${}", key, param_count));
            params.push(Box::new(value.clone()));
        }

        let sql = format!(
            "UPDATE session_states SET {} WHERE session_key = $1 RETURNING *",
            set_clauses.join(", ")
        );

        debug!("Executing session patch query: {}", sql);

        let params_refs: Vec<&(dyn tokio_postgres::types::ToSql + Sync)> = params.iter().map(|p| p.as_ref()).collect();

        let row = client.query_one(&sql, params_refs.as_slice()).await
            .map_err(|e| AppError::Database(format!("Failed to patch session state: {}", e)))?;

        Ok(SessionState {
            id: row.get("id"),
            session_key: row.get("session_key"),
            workflow_id: row.get("workflow_id"),
            plan_id: row.get("plan_id"),
            task_id: row.get("task_id"),
            workflow_loaded: row.get("workflow_loaded"),
            plan_loaded: row.get("plan_loaded"),
            task_loaded: row.get("task_loaded"),
            context_loaded: row.get("context_loaded"),
            review_approved: row.get("review_approved"),
            context_fingerprint: row.get("context_fingerprint"),
            execution_mode: row.get("execution_mode"),
            active_branch_ids: row.try_get("active_branch_ids").unwrap_or(None),
            synthesis_ready: row.get("synthesis_ready"),
            verification_ready: row.get("verification_ready"),
            last_command: row.get("last_command"),
            metadata: row.try_get("metadata").unwrap_or(None),
            updated_at: row.get("updated_at"),
            created_at: row.get("created_at"),
        })
    }
}
