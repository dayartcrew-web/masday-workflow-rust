//! Workflow reminder repository
//!
//! Table names are PascalCase (created by Drizzle/TypeScript): "WorkflowReminder"
//! Column names are camelCase: "workflowId", "taskId", etc.
//! NOTE: The "type" column is lowercase (not camelCase) in the actual DB schema.

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
            SELECT * FROM "WorkflowReminder"
            WHERE COALESCE(acknowledged, false) = false
            ORDER BY severity DESC, "createdAt" ASC
        "#;

        let rows = client
            .query(query, &[])
            .await
            .map_err(|e| AppError::Database(format!("Failed to check reminders: {}", e)))?;

        Ok(rows.iter().map(|r| WorkflowReminder::from_row(r)).collect())
    }

    /// Acknowledge a reminder
    pub async fn acknowledge(&self, id: &str) -> Result<bool> {
        let client = self
            .pool
            .get()
            .await
            .map_err(|e| AppError::Database(format!("Failed to get connection: {}", e)))?;

        let query = r#"UPDATE "WorkflowReminder" SET acknowledged = true WHERE id = $1"#;
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
            r#"SELECT * FROM "WorkflowReminder" WHERE "workflowId" = $1 ORDER BY "createdAt" DESC"#;
        let rows = client
            .query(query, &[&workflow_id])
            .await
            .map_err(|e| AppError::Database(format!("Failed to list reminders: {}", e)))?;

        Ok(rows.iter().map(|r| WorkflowReminder::from_row(r)).collect())
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
            INSERT INTO "WorkflowReminder" (
                id, "workflowId", "taskId", type, severity, message, acknowledged, "createdAt"
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
            SELECT * FROM "WorkflowReminder"
            WHERE severity = $1 AND COALESCE(acknowledged, false) = false
            ORDER BY "createdAt" ASC
        "#;

        let rows = client.query(query, &[&severity]).await.map_err(|e| {
            AppError::Database(format!("Failed to list reminders by severity: {}", e))
        })?;

        Ok(rows.iter().map(|r| WorkflowReminder::from_row(r)).collect())
    }

    /// Delete a reminder
    pub async fn delete(&self, id: &str) -> Result<bool> {
        let client = self
            .pool
            .get()
            .await
            .map_err(|e| AppError::Database(format!("Failed to get connection: {}", e)))?;

        let query = r#"DELETE FROM "WorkflowReminder" WHERE id = $1"#;
        let rows_affected = client
            .execute(query, &[&id])
            .await
            .map_err(|e| AppError::Database(format!("Failed to delete reminder: {}", e)))?;

        Ok(rows_affected > 0)
    }
}
