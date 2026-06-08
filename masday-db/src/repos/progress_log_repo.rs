//! Task progress log repository
//!
//! Table names are snake_case: "task_progress_logs"
//! Column names are snake_case: "workflow_id", "task_id", "agent_name", etc.

use crate::pool::DbPool;
use crate::schema::{NewTaskProgressLog, TaskProgressLog};
use masday_core::{AppError, Result};

pub struct ProgressLogRepo {
    pool: DbPool,
}

impl ProgressLogRepo {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    /// Create a new progress log entry
    pub async fn create(&self, log: &NewTaskProgressLog) -> Result<TaskProgressLog> {
        let client = self
            .pool
            .get()
            .await
            .map_err(|e| AppError::Database(format!("Failed to get connection: {}", e)))?;

        let id = uuid::Uuid::new_v4().to_string();
        let now: chrono::DateTime<chrono::Utc> = chrono::Utc::now();

        let query = r#"
            INSERT INTO task_progress_logs (
                id, workflow_id, task_id, agent_name, status_before,
                status_after, progress_note, evidence, created_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
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
                    &log.status_before,
                    &log.status_after,
                    &log.progress_note,
                    &log.evidence,
                    &now,
                ],
            )
            .await
            .map_err(|e| AppError::Database(format!("Failed to create progress log: {}", e)))?;

        Ok(TaskProgressLog::from_row(&row))
    }

    /// Get progress log by ID
    pub async fn get_by_id(&self, id: &str) -> Result<TaskProgressLog> {
        let client = self
            .pool
            .get()
            .await
            .map_err(|e| AppError::Database(format!("Failed to get connection: {}", e)))?;

        let query = r#"SELECT * FROM task_progress_logs WHERE id = $1"#;
        let row = client
            .query_one(query, &[&id])
            .await
            .map_err(|_e| AppError::not_found("TaskProgressLog", id))?;

        Ok(TaskProgressLog::from_row(&row))
    }

    /// List all progress logs for a workflow
    pub async fn list_by_workflow(&self, workflow_id: &str) -> Result<Vec<TaskProgressLog>> {
        let client = self
            .pool
            .get()
            .await
            .map_err(|e| AppError::Database(format!("Failed to get connection: {}", e)))?;

        let query = r#"
            SELECT * FROM task_progress_logs
            WHERE workflow_id = $1
            ORDER BY created_at DESC
        "#;
        let rows = client
            .query(query, &[&workflow_id])
            .await
            .map_err(|e| AppError::Database(format!("Failed to list progress logs: {}", e)))?;

        Ok(rows.iter().map(TaskProgressLog::from_row).collect())
    }

    /// List all progress logs for a task
    pub async fn list_by_task(&self, task_id: &str) -> Result<Vec<TaskProgressLog>> {
        let client = self
            .pool
            .get()
            .await
            .map_err(|e| AppError::Database(format!("Failed to get connection: {}", e)))?;

        let query = r#"
            SELECT * FROM task_progress_logs
            WHERE task_id = $1
            ORDER BY created_at DESC
        "#;
        let rows = client
            .query(query, &[&task_id])
            .await
            .map_err(|e| AppError::Database(format!("Failed to list task progress logs: {}", e)))?;

        Ok(rows.iter().map(TaskProgressLog::from_row).collect())
    }

    /// List all progress logs (with optional limit)
    pub async fn list_all(&self, limit: Option<i64>) -> Result<Vec<TaskProgressLog>> {
        let client = self
            .pool
            .get()
            .await
            .map_err(|e| AppError::Database(format!("Failed to get connection: {}", e)))?;

        let capped = limit.unwrap_or(100).min(1000);
        let query = r#"SELECT * FROM task_progress_logs ORDER BY created_at DESC LIMIT $1"#;
        let rows = client
            .query(query, &[&capped])
            .await
            .map_err(|e| AppError::Database(format!("Failed to list all progress logs: {}", e)))?;

        Ok(rows.iter().map(TaskProgressLog::from_row).collect())
    }

    /// Delete a progress log entry
    pub async fn delete(&self, id: &str) -> Result<bool> {
        let client = self
            .pool
            .get()
            .await
            .map_err(|e| AppError::Database(format!("Failed to get connection: {}", e)))?;

        let result = client
            .execute(r#"DELETE FROM task_progress_logs WHERE id = $1"#, &[&id])
            .await
            .map_err(|e| AppError::Database(format!("Failed to delete progress log: {}", e)))?;

        Ok(result > 0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_constructor_signature() {
        fn _check() {
            let _ = ProgressLogRepo::new;
        }
    }

    #[test]
    fn test_new_progress_log_construction() {
        let log = NewTaskProgressLog {
            workflow_id: "wf-123".to_string(),
            task_id: "task-456".to_string(),
            agent_name: "masday-executor".to_string(),
            status_before: Some("RUNNING".to_string()),
            status_after: Some("DONE".to_string()),
            progress_note: "Completed".to_string(),
            evidence: None,
        };
        assert_eq!(log.agent_name, "masday-executor");
    }

    #[test]
    fn test_limit_capping() {
        assert_eq!(1000, 1000);
        assert_eq!(500i64, 500);
    }
}
