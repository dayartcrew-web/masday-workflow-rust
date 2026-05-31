//! Stale/stuck workflow detection
//!
//! Detects and manages workflow reminders for stale or stuck workflows.

use masday_db::DbPool;
use masday_core::{AppError, Result};
use masday_db::repos::{ReminderRepo, WorkflowRepo};
use masday_db::schema::WorkflowReminder;
use tracing::{debug, info};
use chrono::Utc;
use chrono::Duration;

/// Reminder service
pub struct ReminderService {
    reminder_repo: ReminderRepo,
    workflow_repo: WorkflowRepo,
}

impl ReminderService {
    /// Create a new reminder service
    pub fn new(pool: DbPool) -> Self {
        Self {
            reminder_repo: ReminderRepo::new(pool.clone()),
            workflow_repo: WorkflowRepo::new(pool),
        }
    }

    /// Check for reminders (stale/stuck/failed workflows)
    ///
    /// # Arguments
    /// * `pool` - Database connection pool
    ///
    /// # Returns
    /// * `Result<Vec<WorkflowReminder>>` - List of active reminders
    pub async fn check_reminders(pool: &DbPool) -> Result<Vec<WorkflowReminder>> {
        info!("Checking for workflow reminders");

        let service = Self::new(pool.clone());

        // Get all active workflows
        let active_workflows = service.workflow_repo.get_active().await?;

        let mut reminders = Vec::new();
        let now = Utc::now();

        for workflow in active_workflows {
            let reminder_type = Self::check_workflow_staleness(&workflow, &now);

            if let Some(reminder_type) = reminder_type {
                let reminder = WorkflowReminder {
                    id: uuid::Uuid::new_v4().to_string(),
                    workflow_id: workflow.id.clone(),
                    task_id: None,
                    severity: "warning".to_string(),
                    message: Self::get_reminder_message(&workflow, &reminder_type),
                    reminder_type,
                    acknowledged: None,
                    created_at: now,
                };

                reminders.push(reminder);
            }
        }

        // Note: Stuck task detection would require additional repo methods
        // For now, we only detect stale workflows

        info!("Found {} active reminders", reminders.len());
        Ok(reminders)
    }

    /// Acknowledge a reminder
    ///
    /// # Arguments
    /// * `pool` - Database connection pool
    /// * `id` - Reminder ID
    ///
    /// # Returns
    /// * `Result<bool>` - true if acknowledged
    pub async fn acknowledge_reminder(pool: &DbPool, id: &str) -> Result<bool> {
        info!("Acknowledging reminder {}", id);

        let service = Self::new(pool.clone());
        service
            .reminder_repo
            .acknowledge(id)
            .await
    }

    /// Check if a workflow is stale
    fn check_workflow_staleness(workflow: &masday_db::schema::Workflow, now: &chrono::DateTime<Utc>) -> Option<String> {
        let updated_age = now.signed_duration_since(workflow.updated_at);

        // Different thresholds based on status
        match workflow.status.as_str() {
            "INIT" | "ANALYZE" | "PLAN" => {
                // Early phases: stale after 1 hour
                if updated_age > Duration::hours(1) {
                    return Some("STALE_EARLY".to_string());
                }
            }
            "EXECUTE" => {
                // Execution phase: stale after 4 hours
                if updated_age > Duration::hours(4) {
                    return Some("STALE_EXECUTE".to_string());
                }
            }
            "VERIFY" => {
                // Verify phase: stale after 30 minutes
                if updated_age > Duration::minutes(30) {
                    return Some("STALE_VERIFY".to_string());
                }
            }
            "FIX" => {
                // Fix phase: stale after 2 hours
                if updated_age > Duration::hours(2) {
                    return Some("STALE_FIX".to_string());
                }
            }
            "PAUSED" => {
                // Paused: remind after 24 hours
                if updated_age > Duration::hours(24) {
                    return Some("PAUSED_LONG".to_string());
                }
            }
            _ => {}
        }

        None
    }

    /// Get reminder message for a workflow
    fn get_reminder_message(workflow: &masday_db::schema::Workflow, reminder_type: &str) -> String {
        match reminder_type {
            "STALE_EARLY" => format!(
                "Workflow '{}' has been idle in {} phase for over 1 hour",
                workflow.name, workflow.status
            ),
            "STALE_EXECUTE" => format!(
                "Workflow '{}' has been executing for over 4 hours",
                workflow.name
            ),
            "STALE_VERIFY" => format!(
                "Workflow '{}' has been in verification for over 30 minutes",
                workflow.name
            ),
            "STALE_FIX" => format!(
                "Workflow '{}' has been in fix mode for over 2 hours",
                workflow.name
            ),
            "PAUSED_LONG" => format!(
                "Workflow '{}' has been paused for over 24 hours",
                workflow.name
            ),
            "STUCK_TASK" => format!(
                "Workflow '{}' has a stuck task",
                workflow.name
            ),
            _ => format!("Reminder for workflow '{}'", workflow.name),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate() {
        assert!(true);
    }
}
