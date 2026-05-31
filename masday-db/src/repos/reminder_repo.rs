//! Workflow reminder repository

use crate::schema::{WorkflowReminder, NewWorkflowReminder};
use crate::pool::DbPool;
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
        let client = self.pool.get().await
            .map_err(|e| AppError::Database(format!("Failed to get connection: {}", e)))?;

        let query = "
            SELECT * FROM workflow_reminders
            WHERE COALESCE(acknowledged, false) = false
            ORDER BY severity DESC, created_at ASC
        ";

        let rows = client.query(query, &[]).await
            .map_err(|e| AppError::Database(format!("Failed to check reminders: {}", e)))?;

        let reminders = rows.iter().map(|row| WorkflowReminder {
            id: row.get("id"),
            workflow_id: row.get("workflow_id"),
            task_id: row.get("task_id"),
            reminder_type: row.get("reminder_type"),
            severity: row.get("severity"),
            message: row.get("message"),
            acknowledged: row.get("acknowledged"),
            created_at: row.get("created_at"),
        }).collect();

        Ok(reminders)
    }

    /// Acknowledge a reminder
    pub async fn acknowledge(&self, id: &str) -> Result<bool> {
        let client = self.pool.get().await
            .map_err(|e| AppError::Database(format!("Failed to get connection: {}", e)))?;

        let query = "UPDATE workflow_reminders SET acknowledged = true WHERE id = $1";
        let rows_affected = client.execute(query, &[&id]).await
            .map_err(|e| AppError::Database(format!("Failed to acknowledge reminder: {}", e)))?;

        Ok(rows_affected > 0)
    }

    /// List all reminders for a workflow
    pub async fn list(&self, workflow_id: &str) -> Result<Vec<WorkflowReminder>> {
        let client = self.pool.get().await
            .map_err(|e| AppError::Database(format!("Failed to get connection: {}", e)))?;

        let query = "SELECT * FROM workflow_reminders WHERE workflow_id = $1 ORDER BY created_at DESC";
        let rows = client.query(query, &[&workflow_id]).await
            .map_err(|e| AppError::Database(format!("Failed to list reminders: {}", e)))?;

        let reminders = rows.iter().map(|row| WorkflowReminder {
            id: row.get("id"),
            workflow_id: row.get("workflow_id"),
            task_id: row.get("task_id"),
            reminder_type: row.get("reminder_type"),
            severity: row.get("severity"),
            message: row.get("message"),
            acknowledged: row.get("acknowledged"),
            created_at: row.get("created_at"),
        }).collect();

        Ok(reminders)
    }

    /// Create a new reminder
    pub async fn create(&self, reminder: &NewWorkflowReminder) -> Result<WorkflowReminder> {
        let client = self.pool.get().await
            .map_err(|e| AppError::Database(format!("Failed to get connection: {}", e)))?;

        let id = uuid::Uuid::new_v4().to_string();
        let now: chrono::DateTime<chrono::Utc> = chrono::Utc::now();

        let query = "
            INSERT INTO workflow_reminders (
                id, workflow_id, task_id, reminder_type, severity, message, acknowledged, created_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            RETURNING *
        ";

        let row = client.query_one(
            query,
            &[
                &id, &reminder.workflow_id, &reminder.task_id, &reminder.reminder_type,
                &reminder.severity, &reminder.message, &reminder.acknowledged, &now,
            ],
        ).await.map_err(|e| AppError::Database(format!("Failed to create reminder: {}", e)))?;

        Ok(WorkflowReminder {
            id: row.get("id"),
            workflow_id: row.get("workflow_id"),
            task_id: row.get("task_id"),
            reminder_type: row.get("reminder_type"),
            severity: row.get("severity"),
            message: row.get("message"),
            acknowledged: row.get("acknowledged"),
            created_at: row.get("created_at"),
        })
    }

    /// Get reminders by severity
    pub async fn list_by_severity(&self, severity: &str) -> Result<Vec<WorkflowReminder>> {
        let client = self.pool.get().await
            .map_err(|e| AppError::Database(format!("Failed to get connection: {}", e)))?;

        let query = "
            SELECT * FROM workflow_reminders
            WHERE severity = $1 AND COALESCE(acknowledged, false) = false
            ORDER BY created_at ASC
        ";

        let rows = client.query(query, &[&severity]).await
            .map_err(|e| AppError::Database(format!("Failed to list reminders by severity: {}", e)))?;

        let reminders = rows.iter().map(|row| WorkflowReminder {
            id: row.get("id"),
            workflow_id: row.get("workflow_id"),
            task_id: row.get("task_id"),
            reminder_type: row.get("reminder_type"),
            severity: row.get("severity"),
            message: row.get("message"),
            acknowledged: row.get("acknowledged"),
            created_at: row.get("created_at"),
        }).collect();

        Ok(reminders)
    }

    /// Delete a reminder
    pub async fn delete(&self, id: &str) -> Result<bool> {
        let client = self.pool.get().await
            .map_err(|e| AppError::Database(format!("Failed to get connection: {}", e)))?;

        let query = "DELETE FROM workflow_reminders WHERE id = $1";
        let rows_affected = client.execute(query, &[&id]).await
            .map_err(|e| AppError::Database(format!("Failed to delete reminder: {}", e)))?;

        Ok(rows_affected > 0)
    }
}
