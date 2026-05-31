//! Session state repository
//!
//! Table names are PascalCase (created by Drizzle/TypeScript): "SessionState"
//! Column names are camelCase: "sessionKey", "workflowId", "contextFingerprint", etc.

use crate::pool::DbPool;
use crate::schema::SessionState;
use masday_core::{AppError, Result};
use tracing::debug;

pub struct SessionRepo {
    pool: DbPool,
}

impl SessionRepo {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    /// Get session state by session key
    pub async fn get_state(&self, session_key: &str) -> Result<Option<SessionState>> {
        let client = self
            .pool
            .get()
            .await
            .map_err(|e| AppError::Database(format!("Failed to get connection: {}", e)))?;

        let query = r#"SELECT * FROM "SessionState" WHERE "sessionKey" = $1"#;
        let rows = client
            .query(query, &[&session_key])
            .await
            .map_err(|e| AppError::Database(format!("Failed to get session state: {}", e)))?;

        if rows.is_empty() {
            return Ok(None);
        }

        Ok(Some(SessionState::from_row(&rows[0])))
    }

    /// Update session state with patch (upsert)
    pub async fn patch_state(
        &self,
        session_key: &str,
        patch: serde_json::Value,
    ) -> Result<SessionState> {
        let client = self
            .pool
            .get()
            .await
            .map_err(|e| AppError::Database(format!("Failed to get connection: {}", e)))?;

        let now: chrono::NaiveDateTime = chrono::Utc::now().naive_utc();

        // First check if session exists
        let existing_query = r#"SELECT * FROM "SessionState" WHERE "sessionKey" = $1"#;
        let existing_rows = client
            .query(existing_query, &[&session_key])
            .await
            .map_err(|e| AppError::Database(format!("Failed to check existing session: {}", e)))?;

        if existing_rows.is_empty() {
            // Create new session state
            let id = uuid::Uuid::new_v4().to_string();

            let query = r#"
                INSERT INTO "SessionState" (
                    id, "sessionKey", "workflowId", "planId", "taskId",
                    "workflowLoaded", "planLoaded", "taskLoaded", "contextLoaded",
                    "reviewApproved", "contextFingerprint", "executionMode",
                    "activeBranchIds", "synthesisReady", "verificationReady",
                    "lastCommand", metadata, "createdAt", "updatedAt"
                )
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19)
                RETURNING *
            "#;

            let row = client
                .query_one(
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
                )
                .await
                .map_err(|e| {
                    AppError::Database(format!("Failed to create session state: {}", e))
                })?;

            return Ok(SessionState::from_row(&row));
        }

        // Update existing session - build dynamic UPDATE
        // Map JSON snake_case keys to DB camelCase column names
        let mut set_clauses = vec![r#""updatedAt" = $2"#.to_string()];
        let mut param_count = 2;
        let mut params: Vec<Box<dyn tokio_postgres::types::ToSql + Sync>> =
            vec![Box::new(session_key.to_string()), Box::new(now)];

        let column_map: std::collections::HashMap<&str, &str> = [
            ("workflow_id", "\"workflowId\""),
            ("plan_id", "\"planId\""),
            ("task_id", "\"taskId\""),
            ("workflow_loaded", "\"workflowLoaded\""),
            ("plan_loaded", "\"planLoaded\""),
            ("task_loaded", "\"taskLoaded\""),
            ("context_loaded", "\"contextLoaded\""),
            ("review_approved", "\"reviewApproved\""),
            ("context_fingerprint", "\"contextFingerprint\""),
            ("execution_mode", "\"executionMode\""),
            ("active_branch_ids", "\"activeBranchIds\""),
            ("synthesis_ready", "\"synthesisReady\""),
            ("verification_ready", "\"verificationReady\""),
            ("last_command", "\"lastCommand\""),
            ("metadata", "metadata"),
        ]
        .iter()
        .cloned()
        .collect();

        for (key, value) in patch.as_object().unwrap_or(&serde_json::Map::new()) {
            if let Some(col_name) = column_map.get(key.as_str()) {
                param_count += 1;
                set_clauses.push(format!("{} = ${}", col_name, param_count));
                params.push(Box::new(value.clone()));
            }
        }

        let sql = format!(
            r#"UPDATE "SessionState" SET {} WHERE "sessionKey" = $1 RETURNING *"#,
            set_clauses.join(", ")
        );

        debug!("Executing session patch query: {}", sql);

        let params_refs: Vec<&(dyn tokio_postgres::types::ToSql + Sync)> =
            params.iter().map(|p| p.as_ref()).collect();

        let row = client
            .query_one(&sql, params_refs.as_slice())
            .await
            .map_err(|e| AppError::Database(format!("Failed to patch session state: {}", e)))?;

        Ok(SessionState::from_row(&row))
    }
}
