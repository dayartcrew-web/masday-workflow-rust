//! Stale/stuck workflow detection
//!
//! Detects and manages workflow reminders for stale or stuck workflows.

use chrono::Duration;
use chrono::Utc;
use masday_core::Result;
use masday_db::repos::{ReminderRepo, TaskRepo, WorkflowRepo};
use masday_db::schema::{NewWorkflowReminder, Task, WorkflowReminder};
use masday_db::DbPool;
use tracing::{info, warn};

/// A task RUNNING longer than this with no `updated_at` refresh is considered
/// stuck. Used as the fallback when a caller omits the `stuckTaskMinutes` MCP
/// param (or passes a value too small to be meaningful — see
/// [`resolve_stuck_task_threshold`]).
///
/// Public so the standalone stdio/SQLite path (`masday-mcp` `direct.rs`) reuses
/// the exact same threshold — single source of truth for what "stuck" means
/// (mirrors how `compute_new_reminders` / `compute_stuck_task_reminders` are
/// already shared across both paths).
pub const DEFAULT_STUCK_TASK_THRESHOLD: Duration = Duration::minutes(60);

/// A workflow in the EXECUTE phase longer than this with no `updated_at`
/// refresh is considered stale (`STALE_EXECUTE`). Used as the fallback when a
/// caller omits the `staleExecutionMinutes` MCP param (or passes a value too
/// small to be meaningful — see [`resolve_stale_execute_threshold`]).
///
/// Public for the same reason as [`DEFAULT_STUCK_TASK_THRESHOLD`]: the
/// standalone stdio/SQLite path (`masday-mcp` `direct.rs`) reuses the exact
/// same threshold — single source of truth for what "stale execution" means.
pub const DEFAULT_STALE_EXECUTE_THRESHOLD: Duration = Duration::hours(4);

/// Resolve a caller-supplied `stuckTaskMinutes` value to a `Duration`.
///
/// Returns [`DEFAULT_STUCK_TASK_THRESHOLD`] when the caller omitted the param
/// (`None`) — preserving legacy behavior — or passed a value `< 1` minute (a
/// 0/negative threshold would flag every RUNNING task as stuck immediately,
/// which is never the intent). Otherwise honors the explicit value.
///
/// Pure (no I/O) and shared by both the HTTP/API path and the stdio/SQLite
/// path, so the clamping rule is identical everywhere — single source of truth
/// for the param→threshold translation (mirrors
/// [`DEFAULT_STUCK_TASK_THRESHOLD`] being shared).
pub fn resolve_stuck_task_threshold(stuck_task_minutes: Option<i64>) -> Duration {
    match stuck_task_minutes {
        Some(m) if m >= 1 => Duration::minutes(m),
        _ => DEFAULT_STUCK_TASK_THRESHOLD,
    }
}

/// Resolve a caller-supplied `staleExecutionMinutes` value to a `Duration`.
///
/// Returns [`DEFAULT_STALE_EXECUTE_THRESHOLD`] when the caller omitted the param
/// (`None`) — preserving legacy behavior — or passed a value `< 1` minute (a
/// 0/negative threshold would flag every EXECUTE workflow as stale immediately,
/// which is never the intent). Otherwise honors the explicit value.
///
/// Pure (no I/O) and shared by both the HTTP/API path and the stdio/SQLite
/// path, so the clamping rule is identical everywhere — mirrors
/// [`resolve_stuck_task_threshold`].
pub fn resolve_stale_execute_threshold(stale_execution_minutes: Option<i64>) -> Duration {
    match stale_execution_minutes {
        Some(m) if m >= 1 => Duration::minutes(m),
        _ => DEFAULT_STALE_EXECUTE_THRESHOLD,
    }
}

/// Render a staleness threshold as a human span for reminder messages: whole
/// hours (`"4 hours"`) when the value divides evenly, otherwise minutes
/// (`"90 minutes"`). Pure so it is unit-testable and reusable by both the PG
/// and stdio message builders.
fn fmt_threshold_span(threshold: Duration) -> String {
    let mins = threshold.num_minutes();
    if mins >= 60 && mins % 60 == 0 {
        format!("{} hours", mins / 60)
    } else {
        format!("{} minutes", mins)
    }
}

/// Reminder service
pub struct ReminderService {
    reminder_repo: ReminderRepo,
    workflow_repo: WorkflowRepo,
    task_repo: TaskRepo,
}

impl ReminderService {
    /// Create a new reminder service
    pub fn new(pool: DbPool) -> Self {
        Self {
            reminder_repo: ReminderRepo::new(pool.clone()),
            workflow_repo: WorkflowRepo::new(pool.clone()),
            task_repo: TaskRepo::new(pool),
        }
    }

    /// Check for reminders (stale/stuck/failed workflows) using the default
    /// stuck-task threshold ([`DEFAULT_STUCK_TASK_THRESHOLD`]).
    ///
    /// Backwards-compatible entry point for callers that have no caller-supplied
    /// threshold — the background sweeper and the reminder routes when no
    /// `stuckTaskMinutes` param was supplied. Behavior is identical to pre-wiring.
    ///
    /// # Arguments
    /// * `pool` - Database connection pool
    ///
    /// # Returns
    /// * `Result<Vec<WorkflowReminder>>` - List of active reminders
    pub async fn check_reminders(pool: &DbPool) -> Result<Vec<WorkflowReminder>> {
        Self::check_reminders_with_options(
            pool,
            DEFAULT_STUCK_TASK_THRESHOLD,
            DEFAULT_STALE_EXECUTE_THRESHOLD,
            false,
        )
        .await
    }

    /// Check for reminders with caller-supplied thresholds and an
    /// `include_failed` flag.
    ///
    /// `stuck_task_threshold` tunes the stuck-task window (the `stuckTaskMinutes`
    /// MCP param). `stale_execute_threshold` tunes the EXECUTE-phase staleness
    /// window (the `staleExecutionMinutes` MCP param) — a workflow in EXECUTE
    /// longer than this surfaces `STALE_EXECUTE`. `include_failed` is the
    /// advertised `includeFailed` MCP param: when true, FAILED workflows are
    /// also fetched ([`WorkflowRepo::get_failed`]) and checked against the
    /// FAILED-staleness threshold — a workflow that failed and has sat
    /// unresolved surfaces a `FAILED_STALE` reminder. When false (default),
    /// FAILED workflows are excluded exactly as before
    /// ([`WorkflowRepo::get_active`] drops them), so legacy behavior is unchanged.
    pub async fn check_reminders_with_options(
        pool: &DbPool,
        stuck_task_threshold: Duration,
        stale_execute_threshold: Duration,
        include_failed: bool,
    ) -> Result<Vec<WorkflowReminder>> {
        info!("Checking for workflow reminders");

        let service = Self::new(pool.clone());

        // Get all active workflows (excludes DONE/FAILED).
        let mut active_workflows = service.workflow_repo.get_active(None).await?;

        // Opt-in FAILED pass. A FAILED workflow is normally terminal and not
        // nagged; include_failed surfaces long-stale FAILED workflows too so an
        // abandoned failure isn't silently forgotten.
        if include_failed {
            active_workflows.extend(service.workflow_repo.get_failed(None).await?);
        }

        let now = Utc::now();

        // Persist any newly-detected reminders (deduped against the
        // unacknowledged set already in the DB). Previously these were built
        // in-memory with a fresh UUID every call and never INSERTed, so list()
        // was always empty and acknowledge() could never match an id — the whole
        // reminder subsystem was theater. See audit round-2 leverage #7.
        let existing = service.reminder_repo.check_reminders().await?;
        let mut fresh = Self::compute_new_reminders(
            &active_workflows,
            &existing,
            &now,
            stale_execute_threshold,
        );

        // Stuck-task pass: a task RUNNING past the threshold with no updated_at
        // refresh yields a STUCK_TASK reminder (one per workflow, deduped against
        // the outstanding set). Previously STUCK_TASK existed only as a message
        // string — nothing produced it and TaskRepo had no stuck query, so the
        // /reminders/stuck route was pure theater.
        let stuck = service.task_repo.find_stuck(stuck_task_threshold).await?;
        fresh.extend(Self::compute_stuck_task_reminders(&stuck, &existing));

        for new in &fresh {
            if let Err(e) = service.reminder_repo.create(new).await {
                warn!("Failed to persist reminder for {}: {}", new.workflow_id, e);
            }
        }

        // Return the full outstanding set so callers see stable, acknowledge-able
        // IDs.
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
    ///
    /// Public so the standalone stdio/SQLite path (`masday-mcp` `direct.rs`) can
    /// reuse the exact same staleness thresholds + dedup instead of a divergent
    /// re-implementation — single source of truth for what "stale" means.
    pub fn compute_new_reminders(
        active: &[masday_db::schema::Workflow],
        existing: &[WorkflowReminder],
        now: &chrono::DateTime<Utc>,
        stale_execute_threshold: Duration,
    ) -> Vec<NewWorkflowReminder> {
        use std::collections::HashSet;

        let mut have: HashSet<(String, String)> = existing
            .iter()
            .map(|r| (r.workflow_id.clone(), r.reminder_type.clone()))
            .collect();

        let mut out = Vec::new();
        for workflow in active {
            if let Some(reminder_type) =
                Self::check_workflow_staleness(workflow, now, stale_execute_threshold)
            {
                // HashSet::insert returns true only for a brand-new key.
                if have.insert((workflow.id.clone(), reminder_type.clone())) {
                    let message = Self::get_reminder_message(
                        workflow,
                        &reminder_type,
                        stale_execute_threshold,
                    );
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

    /// Decide which NEW `STUCK_TASK` reminders to create from a set of stuck
    /// tasks, deduped against reminders already outstanding (unacknowledged) in
    /// the DB. Pure (no I/O) so the per-workflow dedup is unit-testable without
    /// PostgreSQL.
    ///
    /// One reminder per workflow that has at least one stuck task — keyed by
    /// `(workflow_id, "STUCK_TASK")` to match [`compute_new_reminders`]'s dedup
    /// granularity. `task_id` captures the first stuck task found in that
    /// workflow (any subsequent stuck task in the same workflow is a duplicate).
    pub fn compute_stuck_task_reminders(
        stuck: &[Task],
        existing: &[WorkflowReminder],
    ) -> Vec<NewWorkflowReminder> {
        use std::collections::HashSet;

        let have: HashSet<String> = existing
            .iter()
            .filter(|r| r.reminder_type == "STUCK_TASK")
            .map(|r| r.workflow_id.clone())
            .collect();

        let mut seen: HashSet<String> = HashSet::new();
        let mut out = Vec::new();
        for task in stuck {
            // First stuck task per workflow wins; skip workflows that already
            // have an outstanding STUCK_TASK reminder.
            if !seen.insert(task.workflow_id.clone()) || have.contains(&task.workflow_id) {
                continue;
            }
            out.push(NewWorkflowReminder {
                workflow_id: task.workflow_id.clone(),
                task_id: Some(task.id.clone()),
                reminder_type: "STUCK_TASK".to_string(),
                severity: "warning".to_string(),
                message: format!(
                    "Task '{}' ({}) is stuck — RUNNING with no progress",
                    task.title, task.id
                ),
                acknowledged: Some(false),
            });
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
        stale_execute_threshold: Duration,
    ) -> Option<String> {
        let updated_age = now.signed_duration_since(workflow.updated_at);

        // Different thresholds based on status. Only the EXECUTE window is
        // caller-tunable (the `staleExecutionMinutes` MCP param); the others are
        // fixed policy.
        match workflow.status.as_str() {
            status
                if matches!(status, "INIT" | "ANALYZE" | "PLAN")
                    && updated_age > Duration::hours(1) =>
            {
                Some("STALE_EARLY".to_string())
            }
            "EXECUTE" if updated_age > stale_execute_threshold => Some("STALE_EXECUTE".to_string()),
            "VERIFY" if updated_age > Duration::minutes(30) => Some("STALE_VERIFY".to_string()),
            "FIX" if updated_age > Duration::hours(2) => Some("STALE_FIX".to_string()),
            "PAUSED" if updated_age > Duration::hours(24) => Some("PAUSED_LONG".to_string()),
            // Only reachable when the caller opts in via include_failed — FAILED
            // workflows are excluded by get_active/get_failed gating. A FAILED
            // workflow unresolved for over 24h is abandoned and worth surfacing.
            "FAILED" if updated_age > Duration::hours(24) => Some("FAILED_STALE".to_string()),
            _ => None,
        }
    }

    /// Get reminder message for a workflow
    fn get_reminder_message(
        workflow: &masday_db::schema::Workflow,
        reminder_type: &str,
        stale_execute_threshold: Duration,
    ) -> String {
        match reminder_type {
            "STALE_EARLY" => format!(
                "Workflow '{}' has been idle in {} phase for over 1 hour",
                workflow.name, workflow.status
            ),
            "STALE_EXECUTE" => format!(
                "Workflow '{}' has been executing for over {}",
                workflow.name,
                fmt_threshold_span(stale_execute_threshold)
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
            "FAILED_STALE" => format!(
                "Workflow '{}' failed over 24 hours ago and is still unresolved",
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
        let out = ReminderService::compute_new_reminders(
            &[stale, fresh],
            &[],
            &now,
            DEFAULT_STALE_EXECUTE_THRESHOLD,
        );
        assert_eq!(out.len(), 1, "only the stale workflow yields a reminder");
        assert_eq!(out[0].workflow_id, "wf-1");
        assert_eq!(out[0].reminder_type, "STALE_EXECUTE");
    }

    #[test]
    fn compute_new_reminders_dedups_already_outstanding() {
        let now = Utc::now();
        let stale = workflow("wf-1", "EXECUTE", now - Duration::hours(5));
        let existing = vec![existing_reminder("wf-1", "STALE_EXECUTE")];
        let out = ReminderService::compute_new_reminders(
            &[stale],
            &existing,
            &now,
            DEFAULT_STALE_EXECUTE_THRESHOLD,
        );
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
        let out = ReminderService::compute_new_reminders(
            &[a, b],
            &[],
            &now,
            DEFAULT_STALE_EXECUTE_THRESHOLD,
        );
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
        let out = ReminderService::compute_new_reminders(
            &[ok],
            &[],
            &now,
            DEFAULT_STALE_EXECUTE_THRESHOLD,
        );
        assert!(out.is_empty(), "a fresh workflow must not yield a reminder");
    }

    #[test]
    fn compute_new_reminders_failed_stale_when_old() {
        // A FAILED workflow older than 24h (only reached when the caller opts in
        // via include_failed — the repo layer gates the fetch) yields
        // FAILED_STALE; a recently-failed one does not.
        let now = Utc::now();
        let old_failed = workflow("wf-1", "FAILED", now - Duration::hours(25));
        let fresh_failed = workflow("wf-2", "FAILED", now - Duration::minutes(10));
        let out = ReminderService::compute_new_reminders(
            &[old_failed, fresh_failed],
            &[],
            &now,
            DEFAULT_STALE_EXECUTE_THRESHOLD,
        );
        assert_eq!(
            out.len(),
            1,
            "only the long-stale FAILED workflow yields a reminder"
        );
        assert_eq!(out[0].workflow_id, "wf-1");
        assert_eq!(out[0].reminder_type, "FAILED_STALE");
    }

    fn task(id: &str, workflow_id: &str, title: &str) -> Task {
        let now = Utc::now();
        Task {
            id: id.to_string(),
            workflow_id: workflow_id.to_string(),
            plan_id: "plan".to_string(),
            title: title.to_string(),
            status: "RUNNING".to_string(),
            priority: None,
            owner_agent: None,
            skill: None,
            description: None,
            dependencies: None,
            acceptance_criteria: None,
            required_context: None,
            verification_steps: None,
            context_fingerprint: None,
            progress_percent: None,
            requires_tdd: None,
            input: None,
            result: None,
            test_evidence: None,
            metadata: None,
            created_at: now,
            started_at: None,
            completed_at: None,
            updated_at: now,
        }
    }

    #[test]
    fn compute_stuck_task_reminders_empty_when_no_stuck() {
        let out = ReminderService::compute_stuck_task_reminders(&[], &[]);
        assert!(out.is_empty());
    }

    #[test]
    fn compute_stuck_task_reminders_one_per_workflow() {
        let a = task("t-1", "wf-1", "build");
        let out = ReminderService::compute_stuck_task_reminders(&[a], &[]);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].workflow_id, "wf-1");
        assert_eq!(out[0].task_id.as_deref(), Some("t-1"));
        assert_eq!(out[0].reminder_type, "STUCK_TASK");
        assert!(out[0].message.contains("build"));
    }

    #[test]
    fn compute_stuck_task_reminders_dedups_within_workflow() {
        // Two stuck tasks in the same workflow -> a single STUCK_TASK reminder.
        let a = task("t-1", "wf-1", "build");
        let b = task("t-2", "wf-1", "test");
        let out = ReminderService::compute_stuck_task_reminders(&[a, b], &[]);
        assert_eq!(out.len(), 1);
        assert_eq!(
            out[0].task_id.as_deref(),
            Some("t-1"),
            "first stuck task wins"
        );
    }

    #[test]
    fn compute_stuck_task_reminders_emits_distinct_workflows() {
        let a = task("t-1", "wf-1", "build");
        let b = task("t-2", "wf-2", "test");
        let out = ReminderService::compute_stuck_task_reminders(&[a, b], &[]);
        assert_eq!(out.len(), 2);
    }

    #[test]
    fn compute_stuck_task_reminders_skips_already_outstanding() {
        // wf-1 already has an unacknowledged STUCK_TASK -> not re-created.
        let a = task("t-1", "wf-1", "build");
        let existing = vec![existing_reminder("wf-1", "STUCK_TASK")];
        let out = ReminderService::compute_stuck_task_reminders(&[a], &existing);
        assert!(
            out.is_empty(),
            "must not duplicate an outstanding STUCK_TASK"
        );
    }

    #[test]
    fn resolve_stuck_task_threshold_defaults_when_absent() {
        // Omitted param (the legacy/no-arg path) -> default 60-minute window.
        assert_eq!(
            resolve_stuck_task_threshold(None),
            DEFAULT_STUCK_TASK_THRESHOLD
        );
    }

    #[test]
    fn resolve_stuck_task_threshold_honors_explicit_value() {
        // A caller-supplied stuckTaskMinutes=10 wins (the whole point of wiring
        // the param).
        assert_eq!(
            resolve_stuck_task_threshold(Some(10)),
            Duration::minutes(10)
        );
    }

    #[test]
    fn resolve_stuck_task_threshold_clamps_non_positive() {
        // 0 / negative would flag every RUNNING task immediately; fall back to
        // the default rather than producing a degenerate window.
        assert_eq!(
            resolve_stuck_task_threshold(Some(0)),
            DEFAULT_STUCK_TASK_THRESHOLD
        );
        assert_eq!(
            resolve_stuck_task_threshold(Some(-5)),
            DEFAULT_STUCK_TASK_THRESHOLD
        );
    }

    #[test]
    fn resolve_stale_execute_threshold_defaults_when_absent() {
        // Omitted param (the legacy/no-arg path) -> default 4-hour window.
        assert_eq!(
            resolve_stale_execute_threshold(None),
            DEFAULT_STALE_EXECUTE_THRESHOLD
        );
    }

    #[test]
    fn resolve_stale_execute_threshold_honors_explicit_value() {
        // A caller-supplied staleExecutionMinutes=120 wins (the whole point of
        // wiring the param).
        assert_eq!(
            resolve_stale_execute_threshold(Some(120)),
            Duration::minutes(120)
        );
    }

    #[test]
    fn resolve_stale_execute_threshold_clamps_non_positive() {
        // 0 / negative would flag every EXECUTE workflow immediately; fall back
        // to the default rather than producing a degenerate window.
        assert_eq!(
            resolve_stale_execute_threshold(Some(0)),
            DEFAULT_STALE_EXECUTE_THRESHOLD
        );
        assert_eq!(
            resolve_stale_execute_threshold(Some(-5)),
            DEFAULT_STALE_EXECUTE_THRESHOLD
        );
    }

    #[test]
    fn compute_new_reminders_stale_execute_honors_custom_threshold() {
        // Proves the threshold param is actually wired into the EXECUTE arm: the
        // SAME workflow (EXECUTE, updated 3.5h ago) fires STALE_EXECUTE when the
        // threshold is 3h but not when it is 5h. Pre-wiring this was impossible —
        // the window was a hardcoded 4h.
        let now = Utc::now();
        let wf = workflow("wf-1", "EXECUTE", now - Duration::minutes(210)); // 3.5h ago

        let fires = ReminderService::compute_new_reminders(
            std::slice::from_ref(&wf),
            &[],
            &now,
            Duration::minutes(180),
        );
        assert_eq!(fires.len(), 1, "3.5h > 3h threshold -> STALE_EXECUTE");
        assert_eq!(fires[0].reminder_type, "STALE_EXECUTE");

        let quiet = ReminderService::compute_new_reminders(
            std::slice::from_ref(&wf),
            &[],
            &now,
            Duration::minutes(300),
        );
        assert!(quiet.is_empty(), "3.5h < 5h threshold -> no reminder");
    }

    #[test]
    fn fmt_threshold_span_renders_hours_and_minutes() {
        // Whole hours collapse to "N hours"; sub-hour spans stay in minutes.
        assert_eq!(fmt_threshold_span(Duration::hours(4)), "4 hours");
        assert_eq!(fmt_threshold_span(Duration::minutes(240)), "4 hours");
        assert_eq!(fmt_threshold_span(Duration::minutes(90)), "90 minutes");
        assert_eq!(fmt_threshold_span(Duration::minutes(30)), "30 minutes");
    }
}
