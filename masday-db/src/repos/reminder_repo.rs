//! Workflow reminder repository
//!
//! Table names are snake_case: "workflow_reminders"
//! Column names are snake_case: "workflow_id", "task_id", etc.
//! NOTE: The "reminder_type" column is used (not "type").

use crate::pool::DbPool;
use crate::schema::{NewWorkflowReminder, WorkflowReminder};
use masday_core::{AppError, Result};
pub struct ReminderRepo {
    pool: DbPool,
}

impl ReminderRepo {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    /// Check for active reminders (not acknowledged)
    pub async fn check_reminders(&self) -> Result<Vec<WorkflowReminder>> {
        let client = self
            .pool
            .get()
            .await
            .map_err(|e| AppError::Database(format!("Failed to get connection: {}", e)))?;

        let query = r#"
            SELECT * FROM workflow_reminders
            WHERE COALESCE(acknowledged, false) = false
            ORDER BY severity DESC, created_at ASC
        "#;

        let rows = client
            .query(query, &[])
            .await
            .map_err(|e| AppError::Database(format!("Failed to check reminders: {}", e)))?;

        Ok(rows.iter().map(WorkflowReminder::from_row).collect())
    }

    /// Acknowledge a reminder
    pub async fn acknowledge(&self, id: &str) -> Result<bool> {
        let client = self
            .pool
            .get()
            .await
            .map_err(|e| AppError::Database(format!("Failed to get connection: {}", e)))?;

        let query = r#"UPDATE workflow_reminders SET acknowledged = true WHERE id = $1"#;
        let rows_affected = client
            .execute(query, &[&id])
            .await
            .map_err(|e| AppError::Database(format!("Failed to acknowledge reminder: {}", e)))?;

        Ok(rows_affected > 0)
    }

    /// List all reminders for a workflow
    pub async fn list(&self, workflow_id: &str) -> Result<Vec<WorkflowReminder>> {
        let client = self
            .pool
            .get()
            .await
            .map_err(|e| AppError::Database(format!("Failed to get connection: {}", e)))?;

        let query =
            r#"SELECT * FROM workflow_reminders WHERE workflow_id = $1 ORDER BY created_at DESC"#;
        let rows = client
            .query(query, &[&workflow_id])
            .await
            .map_err(|e| AppError::Database(format!("Failed to list reminders: {}", e)))?;

        Ok(rows.iter().map(WorkflowReminder::from_row).collect())
    }

    /// Create a new reminder
    pub async fn create(&self, reminder: &NewWorkflowReminder) -> Result<WorkflowReminder> {
        let client = self
            .pool
            .get()
            .await
            .map_err(|e| AppError::Database(format!("Failed to get connection: {}", e)))?;

        let id = uuid::Uuid::new_v4().to_string();
        let now: chrono::NaiveDateTime = chrono::Utc::now().naive_utc();

        let query = r#"
            INSERT INTO workflow_reminders (
                id, workflow_id, task_id, reminder_type, severity, message, acknowledged, created_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            RETURNING *
        "#;

        let row = client
            .query_one(
                query,
                &[
                    &id,
                    &reminder.workflow_id,
                    &reminder.task_id,
                    &reminder.reminder_type,
                    &reminder.severity,
                    &reminder.message,
                    &reminder.acknowledged,
                    &now,
                ],
            )
            .await
            .map_err(|e| AppError::Database(format!("Failed to create reminder: {}", e)))?;

        Ok(WorkflowReminder::from_row(&row))
    }

    /// Get reminders by severity
    pub async fn list_by_severity(&self, severity: &str) -> Result<Vec<WorkflowReminder>> {
        let client = self
            .pool
            .get()
            .await
            .map_err(|e| AppError::Database(format!("Failed to get connection: {}", e)))?;

        let query = r#"
            SELECT * FROM workflow_reminders
            WHERE severity = $1 AND COALESCE(acknowledged, false) = false
            ORDER BY created_at ASC
        "#;

        let rows = client.query(query, &[&severity]).await.map_err(|e| {
            AppError::Database(format!("Failed to list reminders by severity: {}", e))
        })?;

        Ok(rows.iter().map(WorkflowReminder::from_row).collect())
    }

    /// Delete a reminder
    pub async fn delete(&self, id: &str) -> Result<bool> {
        let client = self
            .pool
            .get()
            .await
            .map_err(|e| AppError::Database(format!("Failed to get connection: {}", e)))?;

        let query = r#"DELETE FROM workflow_reminders WHERE id = $1"#;
        let rows_affected = client
            .execute(query, &[&id])
            .await
            .map_err(|e| AppError::Database(format!("Failed to delete reminder: {}", e)))?;

        Ok(rows_affected > 0)
    }
}
