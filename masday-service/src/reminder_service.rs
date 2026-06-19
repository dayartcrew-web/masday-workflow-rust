//! Stale/stuck workflow detection
//!
//! Detects and manages workflow reminders for stale or stuck workflows.

use chrono::Duration;
use chrono::Utc;
use masday_core::Result;
use masday_db::repos::{ReminderRepo, WorkflowRepo};
use masday_db::schema::{NewWorkflowReminder, WorkflowReminder};
use masday_db::DbPool;
use tracing::{info, warn};

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
        let active_workflows = service.workflow_repo.get_active(None).await?;
        let now = Utc::now();

        // Persist any newly-detected reminders (deduped against the
        // unacknowledged set already in the DB). Previously these were built
        // in-memory with a fresh UUID every call and never INSERTed, so list()
        // was always empty and acknowledge() could never match an id — the whole
        // reminder subsystem was theater. See audit round-2 leverage #7.
        let existing = service.reminder_repo.check_reminders().await?;
        let fresh = Self::compute_new_reminders(&active_workflows, &existing, &now);
        for new in &fresh {
            if let Err(e) = service.reminder_repo.create(new).await {
                warn!("Failed to persist reminder for {}: {}", new.workflow_id, e);
            }
        }

        // Return the full outstanding set so callers see stable, acknowledge-able
        // IDs. (Stuck-task detection is still unimplemented — deferred.)
        info!("Found {} active reminders", existing.len() + fresh.len());
        if fresh.is_empty() {
            Ok(existing)
        } else {
            service.reminder_repo.check_reminders().await
        }
    }

    /// Decide which NEW reminders to create, given the active workflows and the
    /// reminders already outstanding (unacknowledged) in the DB. Pure (no I/O) so
    /// the dedup + classification logic is unit-testable without PostgreSQL.
    ///
    /// A reminder is produced only when the workflow is stale AND no
    /// unacknowledged reminder of the same `(workflow_id, reminder_type)` already
    /// exists — this is the dedup gate that keeps `check_reminders` idempotent.
    fn compute_new_reminders(
        active: &[masday_db::schema::Workflow],
        existing: &[WorkflowReminder],
        now: &chrono::DateTime<Utc>,
    ) -> Vec<NewWorkflowReminder> {
        use std::collections::HashSet;

        let mut have: HashSet<(String, String)> = existing
            .iter()
            .map(|r| (r.workflow_id.clone(), r.reminder_type.clone()))
            .collect();

        let mut out = Vec::new();
        for workflow in active {
            if let Some(reminder_type) = Self::check_workflow_staleness(workflow, now) {
                // HashSet::insert returns true only for a brand-new key.
                if have.insert((workflow.id.clone(), reminder_type.clone())) {
                    let message = Self::get_reminder_message(workflow, &reminder_type);
                    out.push(NewWorkflowReminder {
                        workflow_id: workflow.id.clone(),
                        task_id: None,
                        reminder_type,
                        severity: "warning".to_string(),
                        message,
                        acknowledged: Some(false),
                    });
                }
            }
        }
        out
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
        service.reminder_repo.acknowledge(id).await
    }

    /// Check if a workflow is stale
    fn check_workflow_staleness(
        workflow: &masday_db::schema::Workflow,
        now: &chrono::DateTime<Utc>,
    ) -> Option<String> {
        let updated_age = now.signed_duration_since(workflow.updated_at);

        // Different thresholds based on status
        match workflow.status.as_str() {
            status
                if matches!(status, "INIT" | "ANALYZE" | "PLAN")
                    && updated_age > Duration::hours(1) =>
            {
                Some("STALE_EARLY".to_string())
            }
            "EXECUTE" if updated_age > Duration::hours(4) => Some("STALE_EXECUTE".to_string()),
            "VERIFY" if updated_age > Duration::minutes(30) => Some("STALE_VERIFY".to_string()),
            "FIX" if updated_age > Duration::hours(2) => Some("STALE_FIX".to_string()),
            "PAUSED" if updated_age > Duration::hours(24) => Some("PAUSED_LONG".to_string()),
            _ => None,
        }
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
            "STUCK_TASK" => format!("Workflow '{}' has a stuck task", workflow.name),
            _ => format!("Reminder for workflow '{}'", workflow.name),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::DateTime;

    fn workflow(id: &str, status: &str, updated_at: DateTime<Utc>) -> masday_db::schema::Workflow {
        masday_db::schema::Workflow {
            id: id.to_string(),
            name: format!("wf-{}", id),
            description: None,
            status: status.to_string(),
            project_path: None,
            trace_id: None,
            current_plan_id: None,
            current_task_id: None,
            metadata: None,
            created_at: updated_at,
            updated_at,
        }
    }

    fn existing_reminder(workflow_id: &str, reminder_type: &str) -> WorkflowReminder {
        WorkflowReminder {
            id: format!("r-{}-{}", workflow_id, reminder_type),
            workflow_id: workflow_id.to_string(),
            task_id: None,
            severity: "warning".to_string(),
            message: "outstanding".to_string(),
            reminder_type: reminder_type.to_string(),
            acknowledged: None,
            created_at: Utc::now(),
        }
    }

    #[test]
    fn compute_new_reminders_detects_stale_execute() {
        let now = Utc::now();
        let stale = workflow("wf-1", "EXECUTE", now - Duration::hours(5)); // > 4h threshold
        let fresh = workflow("wf-2", "EXECUTE", now - Duration::minutes(10)); // under threshold
        let out = ReminderService::compute_new_reminders(&[stale, fresh], &[], &now);
        assert_eq!(out.len(), 1, "only the stale workflow yields a reminder");
        assert_eq!(out[0].workflow_id, "wf-1");
        assert_eq!(out[0].reminder_type, "STALE_EXECUTE");
    }

    #[test]
    fn compute_new_reminders_dedups_already_outstanding() {
        let now = Utc::now();
        let stale = workflow("wf-1", "EXECUTE", now - Duration::hours(5));
        let existing = vec![existing_reminder("wf-1", "STALE_EXECUTE")];
        let out = ReminderService::compute_new_reminders(&[stale], &existing, &now);
        assert!(
            out.is_empty(),
            "must not duplicate a reminder type already outstanding for the workflow"
        );
    }

    #[test]
    fn compute_new_reminders_emits_distinct_workflows() {
        let now = Utc::now();
        let a = workflow("wf-1", "EXECUTE", now - Duration::hours(5)); // STALE_EXECUTE
        let b = workflow("wf-2", "VERIFY", now - Duration::minutes(45)); // STALE_VERIFY (>30m)
        let out = ReminderService::compute_new_reminders(&[a, b], &[], &now);
        assert_eq!(
            out.len(),
            2,
            "two stale workflows each emit their own reminder"
        );
    }

    #[test]
    fn compute_new_reminders_skips_non_stale() {
        let now = Utc::now();
        let ok = workflow("wf-1", "EXECUTE", now - Duration::minutes(10));
        let out = ReminderService::compute_new_reminders(&[ok], &[], &now);
        assert!(out.is_empty(), "a fresh workflow must not yield a reminder");
    }
}
