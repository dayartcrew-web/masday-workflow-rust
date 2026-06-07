//! Retrieval log repository
//!
//! Table names are snake_case: "retrieval_logs"
//! Column names are snake_case: "workflow_id", "task_id", "agent_name", etc.

use crate::pool::DbPool;
use crate::schema::{NewRetrievalLog, RetrievalLog};
use masday_core::{AppError, Result};

pub struct RetrievalLogRepo {
    pool: DbPool,
}

impl RetrievalLogRepo {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    /// Create a new retrieval log entry
    pub async fn create(&self, log: &NewRetrievalLog) -> Result<RetrievalLog> {
        let client = self
            .pool
            .get()
            .await
            .map_err(|e| AppError::Database(format!("Failed to get connection: {}", e)))?;

        let id = uuid::Uuid::new_v4().to_string();
        let now: chrono::NaiveDateTime = chrono::Utc::now().naive_utc();

        let query = r#"
            INSERT INTO retrieval_logs (
                id, workflow_id, task_id, agent_name, query, source, results, created_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            RETURNING *
        "#;

        let row = client
            .query_one(
                query,
                &[
                    &id,
                    &log.workflow_id,
                    &log.task_id,
                    &log.agent_name,
                    &log.query,
                    &log.source,
                    &log.results,
                    &now,
                ],
            )
            .await
            .map_err(|e| AppError::Database(format!("Failed to create retrieval log: {}", e)))?;

        Ok(RetrievalLog::from_row(&row))
    }

    /// Get retrieval log by ID
    pub async fn get_by_id(&self, id: &str) -> Result<RetrievalLog> {
        let client = self
            .pool
            .get()
            .await
            .map_err(|e| AppError::Database(format!("Failed to get connection: {}", e)))?;

        let query = r#"SELECT * FROM retrieval_logs WHERE id = $1"#;
        let row = client
            .query_one(query, &[&id])
            .await
            .map_err(|_e| AppError::not_found("RetrievalLog", id))?;

        Ok(RetrievalLog::from_row(&row))
    }

    /// List all retrieval logs for a workflow
    pub async fn list_by_workflow(&self, workflow_id: &str) -> Result<Vec<RetrievalLog>> {
        let client = self
            .pool
            .get()
            .await
            .map_err(|e| AppError::Database(format!("Failed to get connection: {}", e)))?;

        let query = r#"
            SELECT * FROM retrieval_logs
            WHERE workflow_id = $1
            ORDER BY created_at DESC
        "#;
        let rows = client
            .query(query, &[&workflow_id])
            .await
            .map_err(|e| AppError::Database(format!("Failed to list retrieval logs: {}", e)))?;

        Ok(rows.iter().map(RetrievalLog::from_row).collect())
    }

    /// List all retrieval logs for a task
    pub async fn list_by_task(&self, task_id: &str) -> Result<Vec<RetrievalLog>> {
        let client = self
            .pool
            .get()
            .await
            .map_err(|e| AppError::Database(format!("Failed to get connection: {}", e)))?;

        let query = r#"
            SELECT * FROM retrieval_logs
            WHERE task_id = $1
            ORDER BY created_at DESC
        "#;
        let rows = client.query(query, &[&task_id]).await.map_err(|e| {
            AppError::Database(format!("Failed to list task retrieval logs: {}", e))
        })?;

        Ok(rows.iter().map(RetrievalLog::from_row).collect())
    }

    /// List all retrieval logs (with optional limit)
    pub async fn list_all(&self, limit: Option<i64>) -> Result<Vec<RetrievalLog>> {
        let client = self
            .pool
            .get()
            .await
            .map_err(|e| AppError::Database(format!("Failed to get connection: {}", e)))?;

        let capped = limit.unwrap_or(100).min(1000);
        let query = r#"SELECT * FROM retrieval_logs ORDER BY created_at DESC LIMIT $1"#;
        let rows = client
            .query(query, &[&capped])
            .await
            .map_err(|e| AppError::Database(format!("Failed to list all retrieval logs: {}", e)))?;

        Ok(rows.iter().map(RetrievalLog::from_row).collect())
    }

    /// Delete a retrieval log entry
    pub async fn delete(&self, id: &str) -> Result<bool> {
        let client = self
            .pool
            .get()
            .await
            .map_err(|e| AppError::Database(format!("Failed to get connection: {}", e)))?;

        let result = client
            .execute(r#"DELETE FROM retrieval_logs WHERE id = $1"#, &[&id])
            .await
            .map_err(|e| AppError::Database(format!("Failed to delete retrieval log: {}", e)))?;

        Ok(result > 0)
    }
}
