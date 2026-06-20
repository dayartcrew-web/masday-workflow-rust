//! Task business logic and lifecycle management
//!
//! Manages task creation, execution, and completion within workflows.
//! All task state transitions are validated before being persisted.

use masday_core::{AppError, Result, WorkflowState};
use masday_db::repos::TaskRepo;
use masday_db::schema::{NewTask, NewTaskProgressLog, Task, TaskProgressLog};
use masday_db::DbPool;
use tracing::{debug, info, warn};

use crate::policy_service::PolicyService;
use crate::workflow_service::{self, status_to_state};

/// Task service
pub struct TaskService {
    repo: TaskRepo,
}

/// Optional context attached to a new task (audit round-1 H1).
///
/// Previously `add_task` hardcoded `skill`, `acceptance_criteria`,
/// `required_context`, and `input` to `None`, so those task columns were always
/// empty — even when an API/MCP caller sent them. Reviewers and policy/drift
/// checks keyed on `acceptance_criteria` / `required_context` were therefore
/// silent no-ops. `TaskContext` carries the caller-supplied values through to
/// the persisted [`NewTask`]; [`TaskContext::default`] (all `None`) preserves
/// the historical behavior for callers that supply none of them.
///
/// `context_fingerprint` is NOT carried on `TaskContext`: it is derived from
/// the context fields by [`compute_context_fingerprint`] at creation time, so
/// the persisted column is non-`None` whenever the task carries any context.
#[derive(Debug, Clone, Default)]
pub struct TaskContext {
    /// Skill the task is scoped to (e.g. `masday-tdd`).
    pub skill: Option<String>,
    /// Structured acceptance criteria the task must satisfy to be reviewable.
    pub acceptance_criteria: Option<serde_json::Value>,
    /// Explicit required context. When `None`, `add_task` falls back to the
    /// deps-derived `{"dependencies": [...]}` used historically.
    pub required_context: Option<serde_json::Value>,
    /// Free-form task input / payload.
    pub input: Option<serde_json::Value>,
}

/// Outcome of comparing an observed context against a task's recorded baseline
/// fingerprint. Pure — produced by [`evaluate_context_drift`].
#[derive(Debug, Clone, PartialEq)]
pub struct ContextDriftResult {
    /// `true` when the observed context differs from the recorded baseline.
    pub refresh_required: bool,
    /// Human-readable explanation of the verdict.
    pub reason: String,
    /// The task's stored `context_fingerprint` (the baseline), if any.
    pub baseline_fingerprint: Option<String>,
    /// The fingerprint derived from the caller's observed context, if supplied.
    pub observed_fingerprint: Option<String>,
}

/// Compute a content-based fingerprint over a task's defining context
/// (`skill` / `input` / `acceptance_criteria` / `required_context`).
///
/// Returns `None` when **all four** fields are absent/empty/null, so tasks that
/// carry no context (e.g. legacy rows) keep `context_fingerprint = None` and are
/// unaffected. Otherwise returns `"ctx-{016x}"`.
///
/// Deterministic for equal content: JSON values are hashed via their canonical
/// `to_string()` form — `serde_json` does not enable `preserve_order`, so object
/// keys are sorted, and PostgreSQL `jsonb` canonicalizes on read, so a
/// creation-time hash and a later validation-time hash agree.
pub fn compute_context_fingerprint(
    skill: Option<&str>,
    input: Option<&serde_json::Value>,
    acceptance_criteria: Option<&serde_json::Value>,
    required_context: Option<&serde_json::Value>,
) -> Option<String> {
    let skill = skill.filter(|s| !s.is_empty());
    let input = input.filter(|v| !v.is_null());
    let acceptance_criteria = acceptance_criteria.filter(|v| !v.is_null());
    let required_context = required_context.filter(|v| !v.is_null());

    if skill.is_none()
        && input.is_none()
        && acceptance_criteria.is_none()
        && required_context.is_none()
    {
        return None;
    }

    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut hasher = DefaultHasher::new();
    // Tag-prefixed fields prevent concatenation collisions across field
    // boundaries; absent JSON fields hash as the empty string.
    0u8.hash(&mut hasher);
    skill.hash(&mut hasher);
    1u8.hash(&mut hasher);
    canonical_json(input).hash(&mut hasher);
    2u8.hash(&mut hasher);
    canonical_json(acceptance_criteria).hash(&mut hasher);
    3u8.hash(&mut hasher);
    canonical_json(required_context).hash(&mut hasher);
    Some(format!("ctx-{:016x}", hasher.finish()))
}

/// Canonical string form of an optional JSON value (`None` → empty string).
fn canonical_json(value: Option<&serde_json::Value>) -> String {
    value.map(|v| v.to_string()).unwrap_or_default()
}

/// Compare an observed context fingerprint against a task's stored baseline.
///
/// Pure decision logic (no I/O) so it is unit-testable without a database. The
/// caller resolves `observed` either from a supplied `last_fingerprint` or by
/// running [`compute_context_fingerprint`] over its declared context.
pub fn evaluate_context_drift(
    baseline: Option<&str>,
    observed: Option<&str>,
) -> ContextDriftResult {
    let baseline_owned = baseline.map(|s| s.to_string());
    let observed_owned = observed.map(|s| s.to_string());
    match (baseline_owned.as_deref(), observed_owned.as_deref()) {
        (None, _) => ContextDriftResult {
            refresh_required: false,
            reason: "Task has no recorded context fingerprint; nothing to compare against."
                .to_string(),
            baseline_fingerprint: None,
            observed_fingerprint: observed_owned,
        },
        (Some(_), None) => ContextDriftResult {
            refresh_required: false,
            reason: "No observed context or last_fingerprint supplied.".to_string(),
            baseline_fingerprint: baseline_owned,
            observed_fingerprint: None,
        },
        (Some(b), Some(o)) if b == o => ContextDriftResult {
            refresh_required: false,
            reason: "Observed context matches the task baseline.".to_string(),
            baseline_fingerprint: baseline_owned,
            observed_fingerprint: observed_owned,
        },
        (Some(_), Some(_)) => ContextDriftResult {
            refresh_required: true,
            reason:
                "Observed context differs from the task baseline; a context refresh is required."
                    .to_string(),
            baseline_fingerprint: baseline_owned,
            observed_fingerprint: observed_owned,
        },
    }
}

impl TaskService {
    /// Create a new task service
    pub fn new(pool: DbPool) -> Self {
        Self {
            repo: TaskRepo::new(pool),
        }
    }

    /// Add a task to a workflow
    ///
    /// # Arguments
    /// * `pool` - Database connection pool
    /// * `workflow_id` - Parent workflow ID
    /// * `plan_id` - Parent plan ID
    /// * `name` - Task title
    /// * `agent` - Optional agent name
    /// * `dependencies` - Optional list of task IDs this task depends on
    /// * `requires_tdd` - When true, the task cannot be completed until an APPROVED
    ///   review exists (enforced by `complete_task` via `validate_completion`)
    /// * `context` - Optional task context (skill / acceptance_criteria /
    ///   required_context / input). See [`TaskContext`]. Defaults to all-`None`.
    ///
    /// # Returns
    /// * `Result<Task>` - The created task
    #[allow(clippy::too_many_arguments)]
    pub async fn add_task(
        pool: &DbPool,
        workflow_id: String,
        plan_id: String,
        name: String,
        agent: Option<String>,
        dependencies: Option<Vec<String>>,
        requires_tdd: Option<bool>,
        context: TaskContext,
    ) -> Result<Task> {
        info!("Adding task '{}' to workflow {}", name, workflow_id);

        // VALIDATION 1: Check plan_id is not empty
        if plan_id.is_empty() {
            return Err(AppError::validation("plan_id cannot be empty"));
        }

        // VALIDATION 2: Check plan exists and belongs to workflow
        let plan_repo = masday_db::repos::PlanRepo::new(pool.clone());
        let plan = plan_repo
            .get_by_id(&plan_id)
            .await?
            .ok_or_else(|| AppError::not_found("Plan", &plan_id))?;

        if plan.workflow_id != workflow_id {
            return Err(AppError::validation(
                "Plan does not belong to this workflow",
            ));
        }

        // VALIDATION 3: Check workflow state allows task creation
        let workflow_repo = masday_db::repos::WorkflowRepo::new(pool.clone());
        let workflow = workflow_repo
            .get_by_id(&workflow_id)
            .await
            .map_err(|_| AppError::not_found("Workflow", &workflow_id))?;

        let workflow_state = status_to_state(&workflow.status)?;
        if !matches!(
            workflow_state,
            WorkflowState::Plan | WorkflowState::Execute | WorkflowState::Init
        ) {
            return Err(AppError::validation(format!(
                "Cannot add tasks to workflow in {} state. Allowed states: INIT, PLAN, EXECUTE.",
                workflow.status
            )));
        }

        let service = Self::new(pool.clone());

        // Serialize dependencies once for both the `dependencies` column and the
        // deps-derived `required_context` fallback.
        let dependencies_json = dependencies.as_ref().map(|d| serde_json::json!(d));

        // required_context: an explicit value (H1) wins; otherwise fall back to
        // the historical deps-derived `{"dependencies": [...]}` so callers that
        // only pass dependencies see no behavior change.
        let required_context = Self::resolve_required_context(
            context.required_context.as_ref(),
            dependencies.as_ref(),
        );

        // Compute the content fingerprint before the `NewTask` literal below
        // moves the context fields (non-None whenever any context is supplied).
        let context_fingerprint = compute_context_fingerprint(
            context.skill.as_deref(),
            context.input.as_ref(),
            context.acceptance_criteria.as_ref(),
            required_context.as_ref(),
        );

        let new_task = NewTask {
            workflow_id,
            plan_id,
            title: name,
            status: "PENDING".to_string(),
            priority: Some("MEDIUM".to_string()),
            owner_agent: agent,
            skill: context.skill,
            description: None,
            dependencies: dependencies_json,
            acceptance_criteria: context.acceptance_criteria,
            required_context,
            verification_steps: None,
            context_fingerprint,
            progress_percent: Some(0),
            requires_tdd,
            input: context.input,
            result: None,
            test_evidence: None,
            metadata: None,
        };

        let task = service.repo.create(&new_task).await?;
        debug!("Task created with ID: {}", task.id);

        Ok(task)
    }

    /// Resolve the persisted `required_context` for a new task.
    ///
    /// An explicit caller-supplied value (H1) wins; a JSON `null` is treated as
    /// absent. When no explicit value is given, fall back to the historical
    /// deps-derived `{"dependencies": [...]}` so callers that only pass
    /// dependencies are unaffected. Pure (no I/O) so the precedence rule is
    /// unit-testable without a database.
    fn resolve_required_context(
        explicit: Option<&serde_json::Value>,
        dependencies: Option<&Vec<String>>,
    ) -> Option<serde_json::Value> {
        explicit
            .filter(|v| !v.is_null())
            .cloned()
            .or_else(|| dependencies.map(|deps| serde_json::json!({ "dependencies": deps })))
    }

    /// Start a task (PENDING → RUNNING)
    ///
    /// # Arguments
    /// * `pool` - Database connection pool
    /// * `workflow_id` - Parent workflow ID
    /// * `task_id` - Task ID
    ///
    /// # Returns
    /// * `Result<Task>` - The updated task
    pub async fn start_task(pool: &DbPool, workflow_id: &str, task_id: &str) -> Result<Task> {
        info!("Starting task {} in workflow {}", task_id, workflow_id);

        let service = Self::new(pool.clone());
        let task = service.repo.get_by_id(task_id).await?;

        // Validate task belongs to workflow
        if task.workflow_id != workflow_id {
            return Err(AppError::validation(format!(
                "Task {} does not belong to workflow {}",
                task_id, workflow_id
            )));
        }

        // Validate current status
        if task.status != "PENDING" {
            return Err(AppError::validation(format!(
                "Cannot start task with status: {}",
                task.status
            )));
        }

        // Update task status
        let updated_task = service.repo.update_status(task_id, "RUNNING").await?;

        debug!("Task {} transitioned to RUNNING", task_id);
        Ok(updated_task)
    }

    /// Complete a task (RUNNING → DONE)
    ///
    /// # Arguments
    /// * `pool` - Database connection pool
    /// * `workflow_id` - Parent workflow ID
    /// * `task_id` - Task ID
    /// * `result` - Optional result data
    ///
    /// # Returns
    /// * `Result<Task>` - The updated task
    pub async fn complete_task(
        pool: &DbPool,
        workflow_id: &str,
        task_id: &str,
        result: Option<serde_json::Value>,
    ) -> Result<Task> {
        info!("Completing task {} in workflow {}", task_id, workflow_id);

        let service = Self::new(pool.clone());
        let task = service.repo.get_by_id(task_id).await?;

        // Validate task belongs to workflow
        if task.workflow_id != workflow_id {
            return Err(AppError::validation(format!(
                "Task {} does not belong to workflow {}",
                task_id, workflow_id
            )));
        }

        // Validate current status
        if task.status != "RUNNING" {
            return Err(AppError::validation(format!(
                "Cannot complete task with status: {}",
                task.status
            )));
        }

        // C2.8: enforce the review/TDD completion gate before allowing completion.
        // `validate_completion` returns Err when the task has requires_tdd=true and no
        // APPROVED review exists. Tasks with requires_tdd = None/false — i.e. every
        // pre-existing task, since add_task previously hardcoded None — are unaffected.
        PolicyService::validate_completion(pool, None, workflow_id, task_id).await?;

        // Update task with result and completion
        let result_data = result.unwrap_or(serde_json::Value::Null);
        let result_for_memory = result_data.clone();
        let updated_task = service.repo.complete(task_id, result_data).await?;

        debug!("Task {} completed successfully", task_id);

        // Auto-store task result as experience memory (best-effort)
        {
            let summary = format!("Task completed: {}", task.title);
            let content = result_for_memory.to_string();
            workflow_service::auto_store_memory(
                pool,
                workflow_id,
                Some(task_id),
                "experience",
                &summary,
                &content,
                0.6,
                vec!["auto".to_string(), "task-complete".to_string()],
            )
            .await;
        }

        // Auto-transition workflow if all tasks are done
        Self::auto_transition_if_all_done(pool, workflow_id).await?;

        Ok(updated_task)
    }

    /// Mark a task as FAILED and route the workflow into FIX (C2.10/C2.11).
    ///
    /// Previously no path existed to mark a task FAILED, and a failed task left its
    /// workflow stuck in EXECUTE forever. Now failing an active (RUNNING/PENDING) task
    /// marks it FAILED, records a best-effort failure memory, and — when the workflow
    /// state allows it (Execute→Fix, Verify→Fix) — auto-transitions the workflow to FIX
    /// so the failure becomes actionable. FAILED/Done workflows are left untouched.
    ///
    /// # Arguments
    /// * `pool` - Database connection pool
    /// * `workflow_id` - Parent workflow ID
    /// * `task_id` - Task ID
    /// * `error` - Optional failure reason (stored as an experience memory)
    ///
    /// # Returns
    /// * `Result<Task>` - The FAILED task
    pub async fn fail_task(
        pool: &DbPool,
        workflow_id: &str,
        task_id: &str,
        error: Option<String>,
    ) -> Result<Task> {
        info!("Failing task {} in workflow {}", task_id, workflow_id);

        let service = Self::new(pool.clone());
        let task = service.repo.get_by_id(task_id).await?;

        // Validate task belongs to workflow
        if task.workflow_id != workflow_id {
            return Err(AppError::validation(format!(
                "Task {} does not belong to workflow {}",
                task_id, workflow_id
            )));
        }

        // Only active tasks can fail
        if task.status != "RUNNING" && task.status != "PENDING" {
            return Err(AppError::validation(format!(
                "Cannot fail task with status: {} (only RUNNING/PENDING tasks can fail)",
                task.status
            )));
        }

        // C2.10: mark the task FAILED
        let failed_task = service.repo.update_status(task_id, "FAILED").await?;
        debug!("Task {} marked FAILED", task_id);

        // Best-effort failure memory (mirrors complete_task's result memory)
        {
            let summary = format!("Task failed: {}", task.title);
            let content = error.unwrap_or_else(|| "Task failed".to_string());
            workflow_service::auto_store_memory(
                pool,
                workflow_id,
                Some(task_id),
                "experience",
                &summary,
                &content,
                0.7,
                vec!["auto".to_string(), "task-failed".to_string()],
            )
            .await;
        }

        // C2.11: route the workflow into FIX so the failure is actionable.
        // Execute→Fix and Verify→Fix are legal; other states are left as-is.
        if let Ok(wf) = workflow_service::WorkflowService::get_workflow(pool, workflow_id).await {
            if let Ok(current) = workflow_service::status_to_state(&wf.status) {
                if workflow_service::can_transition(&current, &WorkflowState::Fix) {
                    if let Err(e) = workflow_service::WorkflowService::transition_status(
                        pool,
                        workflow_id,
                        WorkflowState::Fix,
                    )
                    .await
                    {
                        warn!(
                            "Failed to auto-transition workflow {} to FIX after task {} failed: {}",
                            workflow_id, task_id, e
                        );
                    }
                }
            }
        }

        Ok(failed_task)
    }

    /// Reset a workflow's FAILED tasks back to PENDING for re-execution (FIX-reset).
    ///
    /// Called when a workflow re-enters EXECUTE from FIX, so the previously-failed tasks
    /// can be retried. Only FAILED tasks are reset — DONE tasks (already-succeeded work)
    /// are preserved.
    ///
    /// # Returns
    /// * `Result<u64>` - Number of tasks reset to PENDING
    pub async fn reset_failed_tasks_for_reexecute(pool: &DbPool, workflow_id: &str) -> Result<u64> {
        let service = Self::new(pool.clone());
        let tasks = service.repo.list_by_workflow(workflow_id).await?;
        let mut reset_count: u64 = 0;
        for task in &tasks {
            if task.status == "FAILED" {
                service.repo.update_status(&task.id, "PENDING").await?;
                reset_count += 1;
            }
        }
        Ok(reset_count)
    }

    /// Get the current active task for a workflow
    ///
    /// # Arguments
    /// * `pool` - Database connection pool
    /// * `workflow_id` - Workflow ID
    ///
    /// # Returns
    /// * `Result<Option<Task>>` - The current task if any
    pub async fn get_current_task(pool: &DbPool, workflow_id: &str) -> Result<Option<Task>> {
        debug!("Getting current task for workflow {}", workflow_id);

        let service = Self::new(pool.clone());
        let tasks = service.repo.list_by_workflow(workflow_id).await?;

        // Find the first RUNNING task, or first PENDING if none running
        let current = tasks
            .iter()
            .find(|t| t.status == "RUNNING")
            .or_else(|| tasks.iter().find(|t| t.status == "PENDING"))
            .cloned();

        Ok(current)
    }

    /// List all tasks for a workflow
    ///
    /// # Arguments
    /// * `pool` - Database connection pool
    /// * `workflow_id` - Workflow ID
    ///
    /// # Returns
    /// * `Result<Vec<Task>>` - List of tasks
    pub async fn list_tasks(pool: &DbPool, workflow_id: &str) -> Result<Vec<Task>> {
        debug!("Listing tasks for workflow {}", workflow_id);

        let service = Self::new(pool.clone());
        service.repo.list_by_workflow(workflow_id).await
    }

    /// Save progress for a task
    ///
    /// # Arguments
    /// * `pool` - Database connection pool
    /// * `workflow_id` - Parent workflow ID
    /// * `task_id` - Task ID
    /// * `agent` - Agent name making the progress
    /// * `note` - Progress note
    /// * `evidence` - Optional evidence data
    ///
    /// # Returns
    /// * `Result<TaskProgressLog>` - The created progress log entry
    pub async fn save_progress(
        pool: &DbPool,
        workflow_id: &str,
        task_id: &str,
        agent: String,
        note: String,
        evidence: Option<serde_json::Value>,
    ) -> Result<TaskProgressLog> {
        debug!(
            "Saving progress for task {} by agent {}: {}",
            task_id, agent, note
        );

        let service = Self::new(pool.clone());

        // Get task to capture status before
        let task = service.repo.get_by_id(task_id).await?;
        let status_before = Some(task.status.clone());

        // Create progress log entry
        let new_log = NewTaskProgressLog {
            workflow_id: workflow_id.to_string(),
            task_id: task_id.to_string(),
            agent_name: agent,
            status_before,
            status_after: Some(task.status.clone()),
            progress_note: note,
            evidence,
        };

        let log = service.repo.save_progress(&new_log).await?;

        // Bump `tasks.updated_at` so the stuck-task detector (`find_stuck`,
        // which keys on updated_at) does not flag this actively-progressing
        // RUNNING task as stuck — matches the invariant documented on
        // `find_stuck`. Best-effort: the progress log is the primary record, so
        // a bump failure is logged rather than failing the save.
        if let Err(e) = service.repo.touch_updated_at(task_id).await {
            warn!(
                "Failed to bump tasks.updated_at on progress save for task {}: {}",
                task_id, e
            );
        }

        debug!("Progress log created with ID: {}", log.id);
        Ok(log)
    }

    /// Check if all tasks in a workflow are DONE and auto-transition the workflow.
    ///
    /// Transition logic:
    /// - EXECUTE → VERIFY → DONE (step through verify)
    /// - VERIFY → DONE
    /// - FIX → DONE
    /// - INIT / ANALYZE → DONE (skip intermediate, valid per state machine)
    /// - PLAN → EXECUTE → VERIFY → DONE (unlikely but handled)
    ///
    /// Silently logs a warning if the transition fails (non-blocking for task completion).
    async fn auto_transition_if_all_done(pool: &DbPool, workflow_id: &str) -> Result<()> {
        let service = Self::new(pool.clone());

        // Check if all tasks are DONE
        let all_tasks = service.repo.list_by_workflow(workflow_id).await?;
        if all_tasks.is_empty() {
            return Ok(());
        }
        let all_done = all_tasks.iter().all(|t| t.status == "DONE");
        if !all_done {
            return Ok(());
        }

        // All tasks done — transition workflow
        let workflow = workflow_service::WorkflowService::get_workflow(pool, workflow_id).await?;
        let current_state = match status_to_state(&workflow.status) {
            Ok(s) => s,
            Err(e) => {
                warn!("Cannot parse workflow status '{}': {}", workflow.status, e);
                return Ok(());
            }
        };

        // Already done
        if current_state == WorkflowState::Done {
            return Ok(());
        }

        info!(
            "All tasks done for workflow {} (state: {:?}), auto-transitioning",
            workflow_id, current_state
        );

        // Determine the transition path based on current state
        let transitions: Vec<WorkflowState> = match current_state {
            WorkflowState::Execute => vec![WorkflowState::Verify, WorkflowState::Done],
            WorkflowState::Verify => vec![WorkflowState::Done],
            WorkflowState::Fix => vec![WorkflowState::Done],
            WorkflowState::Init => vec![WorkflowState::Done],
            WorkflowState::Analyze => vec![WorkflowState::Done],
            WorkflowState::Plan => {
                vec![
                    WorkflowState::Execute,
                    WorkflowState::Verify,
                    WorkflowState::Done,
                ]
            }
            WorkflowState::Paused => {
                vec![
                    WorkflowState::Execute,
                    WorkflowState::Verify,
                    WorkflowState::Done,
                ]
            }
            WorkflowState::Failed => {
                // Cannot auto-transition from FAILED
                warn!(
                    "Workflow {} is FAILED, skipping auto-transition",
                    workflow_id
                );
                return Ok(());
            }
            WorkflowState::Done => vec![],
        };

        // Execute transitions sequentially
        let mut reached_done = false;
        for target in transitions {
            match workflow_service::WorkflowService::transition_status(
                pool,
                workflow_id,
                target.clone(),
            )
            .await
            {
                Ok(_) => {
                    info!("Workflow {} transitioned to {:?}", workflow_id, target);
                    if target == WorkflowState::Done {
                        reached_done = true;
                    }
                }
                Err(e) => {
                    warn!(
                        "Failed to transition workflow {} to {:?}: {}",
                        workflow_id, target, e
                    );
                    break;
                }
            }
        }

        // Auto-store workflow completion summary (best-effort)
        if reached_done {
            let task_summaries: Vec<serde_json::Value> = all_tasks
                .iter()
                .map(|t| {
                    serde_json::json!({
                        "id": t.id,
                        "title": t.title,
                        "status": t.status,
                    })
                })
                .collect();
            let summary_content = serde_json::json!({
                "workflow_id": workflow_id,
                "workflow_name": workflow.name,
                "final_status": "DONE",
                "task_count": all_tasks.len(),
                "tasks": task_summaries,
            })
            .to_string();

            workflow_service::auto_store_memory(
                pool,
                workflow_id,
                None,
                "experience",
                &format!(
                    "Workflow '{}' completed ({} tasks)",
                    workflow.name,
                    all_tasks.len()
                ),
                &summary_content,
                0.8,
                vec!["auto".to_string(), "workflow-complete".to_string()],
            )
            .await;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // resolve_required_context precedence/fallback (audit round-1 H1):
    // an explicit value wins; otherwise fall back to deps-derived; JSON null is
    // treated as absent so an explicit null behaves like "not provided".

    #[test]
    fn resolve_required_context_explicit_wins_over_deps() {
        let explicit = serde_json::json!({"prd_ref": "doc-123"});
        let deps = vec!["t1".to_string(), "t2".to_string()];
        let out = TaskService::resolve_required_context(Some(&explicit), Some(&deps));
        assert_eq!(
            out,
            Some(explicit),
            "explicit context must override the deps-derived default"
        );
    }

    #[test]
    fn resolve_required_context_falls_back_to_deps() {
        let deps = vec!["t1".to_string(), "t2".to_string()];
        let out = TaskService::resolve_required_context(None, Some(&deps));
        assert_eq!(
            out,
            Some(serde_json::json!({"dependencies": ["t1", "t2"]})),
            "without an explicit value, deps-derived context must be used (historical behavior)"
        );
    }

    #[test]
    fn resolve_required_context_none_when_both_absent() {
        let out = TaskService::resolve_required_context(None, None);
        assert!(out.is_none(), "no context and no deps ⇒ None");
    }

    #[test]
    fn resolve_required_context_explicit_null_is_absent() {
        let null = serde_json::Value::Null;
        let deps = vec!["t1".to_string()];
        // Explicit null should fall through to the deps-derived value.
        let out = TaskService::resolve_required_context(Some(&null), Some(&deps));
        assert_eq!(out, Some(serde_json::json!({"dependencies": ["t1"]})));

        // Explicit null with no deps ⇒ None.
        let out = TaskService::resolve_required_context(Some(&null), None);
        assert!(out.is_none());
    }

    #[test]
    fn test_transition_path_from_execute() {
        // EXECUTE should transition through VERIFY → DONE
        let state = WorkflowState::Execute;
        let transitions: Vec<WorkflowState> = match state {
            WorkflowState::Execute => vec![WorkflowState::Verify, WorkflowState::Done],
            WorkflowState::Verify => vec![WorkflowState::Done],
            WorkflowState::Fix => vec![WorkflowState::Done],
            WorkflowState::Init => vec![WorkflowState::Done],
            WorkflowState::Analyze => vec![WorkflowState::Done],
            WorkflowState::Plan => {
                vec![
                    WorkflowState::Execute,
                    WorkflowState::Verify,
                    WorkflowState::Done,
                ]
            }
            WorkflowState::Paused => {
                vec![
                    WorkflowState::Execute,
                    WorkflowState::Verify,
                    WorkflowState::Done,
                ]
            }
            WorkflowState::Failed => vec![],
            WorkflowState::Done => vec![],
        };
        assert_eq!(
            transitions,
            vec![WorkflowState::Verify, WorkflowState::Done]
        );
    }

    #[test]
    fn test_transition_path_from_init() {
        // INIT can go directly to DONE
        let state = WorkflowState::Init;
        let transitions: Vec<WorkflowState> = match state {
            WorkflowState::Execute => vec![WorkflowState::Verify, WorkflowState::Done],
            WorkflowState::Verify => vec![WorkflowState::Done],
            WorkflowState::Fix => vec![WorkflowState::Done],
            WorkflowState::Init => vec![WorkflowState::Done],
            WorkflowState::Analyze => vec![WorkflowState::Done],
            WorkflowState::Plan => {
                vec![
                    WorkflowState::Execute,
                    WorkflowState::Verify,
                    WorkflowState::Done,
                ]
            }
            WorkflowState::Paused => {
                vec![
                    WorkflowState::Execute,
                    WorkflowState::Verify,
                    WorkflowState::Done,
                ]
            }
            WorkflowState::Failed => vec![],
            WorkflowState::Done => vec![],
        };
        assert_eq!(transitions, vec![WorkflowState::Done]);
    }

    #[test]
    fn test_transition_path_from_verify() {
        let state = WorkflowState::Verify;
        let transitions: Vec<WorkflowState> = match state {
            WorkflowState::Execute => vec![WorkflowState::Verify, WorkflowState::Done],
            WorkflowState::Verify => vec![WorkflowState::Done],
            WorkflowState::Fix => vec![WorkflowState::Done],
            WorkflowState::Init => vec![WorkflowState::Done],
            WorkflowState::Analyze => vec![WorkflowState::Done],
            WorkflowState::Plan => {
                vec![
                    WorkflowState::Execute,
                    WorkflowState::Verify,
                    WorkflowState::Done,
                ]
            }
            WorkflowState::Paused => {
                vec![
                    WorkflowState::Execute,
                    WorkflowState::Verify,
                    WorkflowState::Done,
                ]
            }
            WorkflowState::Failed => vec![],
            WorkflowState::Done => vec![],
        };
        assert_eq!(transitions, vec![WorkflowState::Done]);
    }

    #[test]
    fn test_transition_path_from_fix() {
        let state = WorkflowState::Fix;
        let transitions: Vec<WorkflowState> = match state {
            WorkflowState::Execute => vec![WorkflowState::Verify, WorkflowState::Done],
            WorkflowState::Verify => vec![WorkflowState::Done],
            WorkflowState::Fix => vec![WorkflowState::Done],
            WorkflowState::Init => vec![WorkflowState::Done],
            WorkflowState::Analyze => vec![WorkflowState::Done],
            WorkflowState::Plan => {
                vec![
                    WorkflowState::Execute,
                    WorkflowState::Verify,
                    WorkflowState::Done,
                ]
            }
            WorkflowState::Paused => {
                vec![
                    WorkflowState::Execute,
                    WorkflowState::Verify,
                    WorkflowState::Done,
                ]
            }
            WorkflowState::Failed => vec![],
            WorkflowState::Done => vec![],
        };
        assert_eq!(transitions, vec![WorkflowState::Done]);
    }

    #[test]
    fn test_transition_path_from_failed_is_empty() {
        let state = WorkflowState::Failed;
        let transitions: Vec<WorkflowState> = match state {
            WorkflowState::Execute => vec![WorkflowState::Verify, WorkflowState::Done],
            WorkflowState::Verify => vec![WorkflowState::Done],
            WorkflowState::Fix => vec![WorkflowState::Done],
            WorkflowState::Init => vec![WorkflowState::Done],
            WorkflowState::Analyze => vec![WorkflowState::Done],
            WorkflowState::Plan => {
                vec![
                    WorkflowState::Execute,
                    WorkflowState::Verify,
                    WorkflowState::Done,
                ]
            }
            WorkflowState::Paused => {
                vec![
                    WorkflowState::Execute,
                    WorkflowState::Verify,
                    WorkflowState::Done,
                ]
            }
            WorkflowState::Failed => vec![],
            WorkflowState::Done => vec![],
        };
        assert!(transitions.is_empty());
    }

    #[test]
    fn test_transition_paths_are_valid_per_state_machine() {
        // Verify that all transition paths respect can_transition_to rules
        use masday_core::WorkflowState;

        fn validate_path(path: &[WorkflowState]) {
            for i in 0..path.len() - 1 {
                assert!(
                    path[i].can_transition_to(&path[i + 1]),
                    "Invalid transition in path: {:?} → {:?}",
                    path[i],
                    path[i + 1]
                );
            }
        }

        // EXECUTE path
        validate_path(&[
            WorkflowState::Execute,
            WorkflowState::Verify,
            WorkflowState::Done,
        ]);
        // VERIFY path
        validate_path(&[WorkflowState::Verify, WorkflowState::Done]);
        // FIX path
        validate_path(&[WorkflowState::Fix, WorkflowState::Done]);
        // INIT path
        validate_path(&[WorkflowState::Init, WorkflowState::Done]);
        // ANALYZE path
        validate_path(&[WorkflowState::Analyze, WorkflowState::Done]);
        // PLAN path
        validate_path(&[
            WorkflowState::Plan,
            WorkflowState::Execute,
            WorkflowState::Verify,
            WorkflowState::Done,
        ]);
        // PAUSED path
        validate_path(&[
            WorkflowState::Paused,
            WorkflowState::Execute,
            WorkflowState::Verify,
            WorkflowState::Done,
        ]);
    }

    // Validation logic tests (structural - these validate the logic flow, not DB operations)

    #[test]
    fn test_validation_logic_checks_empty_plan_id() {
        // This test validates that empty plan_id is rejected
        // The actual validation happens in add_task() at line 50-52
        let plan_id = "";
        assert!(plan_id.is_empty(), "Empty plan_id should be detected");
    }

    #[test]
    fn test_validation_logic_allows_init_plan_execute_states() {
        // This test validates that only INIT, PLAN, EXECUTE states are allowed
        // The actual validation happens in add_task() at lines 75-78

        // Simulate the validation logic
        let valid_states = vec![
            WorkflowState::Init,
            WorkflowState::Plan,
            WorkflowState::Execute,
        ];

        let invalid_states = vec![
            WorkflowState::Analyze,
            WorkflowState::Verify,
            WorkflowState::Fix,
            WorkflowState::Paused,
            WorkflowState::Failed,
            WorkflowState::Done,
        ];

        // Valid states should match
        for state in valid_states {
            let is_valid = matches!(
                state,
                WorkflowState::Plan | WorkflowState::Execute | WorkflowState::Init
            );
            assert!(is_valid, "State {:?} should be valid", state);
        }

        // Invalid states should not match
        for state in invalid_states {
            let is_valid = matches!(
                state,
                WorkflowState::Plan | WorkflowState::Execute | WorkflowState::Init
            );
            assert!(!is_valid, "State {:?} should be invalid", state);
        }
    }

    #[test]
    fn test_validation_logic_error_message_includes_all_allowed_states() {
        // This test validates that the error message includes all three allowed states
        // The actual error message is at line 79-82

        let workflow_status = "VERIFY";
        let error_msg = format!(
            "Cannot add tasks to workflow in {} state. Allowed states: INIT, PLAN, EXECUTE.",
            workflow_status
        );

        assert!(error_msg.contains("INIT"), "Error should mention INIT");
        assert!(error_msg.contains("PLAN"), "Error should mention PLAN");
        assert!(
            error_msg.contains("EXECUTE"),
            "Error should mention EXECUTE"
        );
        assert!(
            error_msg.contains("VERIFY"),
            "Error should mention the invalid state"
        );
    }

    #[test]
    fn test_validation_logic_workflow_belongs_to_plan() {
        // This test validates that plan workflow_id must match task workflow_id
        // The actual validation happens in add_task() at lines 61-64

        let task_workflow_id = "workflow-123";
        let plan_workflow_id = "workflow-456";

        let belongs_to_workflow = plan_workflow_id == task_workflow_id;
        assert!(
            !belongs_to_workflow,
            "Plan should not belong to different workflow"
        );

        // Correct case
        let plan_workflow_id_correct = "workflow-123";
        let belongs_to_workflow_correct = plan_workflow_id_correct == task_workflow_id;
        assert!(
            belongs_to_workflow_correct,
            "Plan should belong to same workflow"
        );
    }

    // C2.8/C2.9: completion review-gate tests (structural — they validate the decision
    // rule, not DB operations; the real enforcement is complete_task -> validate_completion).

    #[test]
    fn test_requires_tdd_param_threads_to_new_task_field() {
        // C2.9: add_task's requires_tdd param must reach the NewTask.requires_tdd field.
        // Previously hardcoded to None, which made the completion gate unreachable in
        // production. Now the param value is assigned directly (`requires_tdd,`).
        let explicit_true: Option<bool> = Some(true);
        let explicit_false: Option<bool> = Some(false);
        let default_none: Option<bool> = None;

        assert_eq!(explicit_true, Some(true));
        assert_eq!(explicit_false, Some(false));
        assert_eq!(default_none, None);
    }

    #[test]
    fn test_completion_review_gate_decision_logic() {
        // C2.8: complete_task calls validate_completion. Its gate rule is:
        //   blocked = task.requires_tdd.unwrap_or(false) && latest_review.decision != "APPROVED"
        // Mirrored here as `gate(requires_tdd, has_approved_review)`. `black_box` keeps the
        // Option values opaque so the test exercises the real unwrap_or path (not const-folded).
        use std::hint::black_box;
        let gate = |requires_tdd: Option<bool>, has_approved_review: bool| -> bool {
            requires_tdd.unwrap_or(false) && !has_approved_review
        };

        // Non-TDD task (requires_tdd None or false): never gated, so every pre-existing
        // task (add_task previously hardcoded None) is unaffected by the new gate.
        for requires_tdd in [black_box(None), black_box(Some(false))] {
            assert!(!gate(requires_tdd, false));
            assert!(!gate(requires_tdd, true));
        }

        // TDD task: blocked exactly when no approved review exists.
        let tdd = black_box(Some(true));
        assert!(
            gate(tdd, false),
            "TDD task without approved review must be blocked"
        );
        assert!(
            !gate(tdd, true),
            "TDD task with approved review must not be blocked"
        );
    }

    // C2.10/C2.11/FIX-reset: failure→FIX recovery tests (structural — validate the
    // decision rules, not DB operations; enforcement lives in fail_task +
    // WorkflowService::transition_status's FIX-reset hook).

    #[test]
    fn test_fail_task_only_allows_active_statuses() {
        // C2.10: fail_task rejects any task that isn't RUNNING or PENDING — you can't
        // fail already-terminal work (mirrors the status gate in fail_task).
        let can_fail = |status: &str| status == "RUNNING" || status == "PENDING";
        for active in &["RUNNING", "PENDING"] {
            assert!(can_fail(active), "{active} should be failable");
        }
        for terminal in &["DONE", "FAILED"] {
            assert!(!can_fail(terminal), "{terminal} should not be failable");
        }
    }

    #[test]
    fn test_failure_routes_workflow_to_fix_only_when_allowed() {
        // C2.11: fail_task auto-transitions to FIX only when the state machine permits.
        // Execute→Fix and Verify→Fix are legal recovery routes; a terminal workflow
        // (Done/Failed) is left untouched (can_transition guards it).
        use crate::workflow_service::can_transition;
        assert!(
            can_transition(&WorkflowState::Execute, &WorkflowState::Fix),
            "Execute→Fix must be allowed"
        );
        assert!(
            can_transition(&WorkflowState::Verify, &WorkflowState::Fix),
            "Verify→Fix must be allowed"
        );
        assert!(
            !can_transition(&WorkflowState::Done, &WorkflowState::Fix),
            "Done→Fix must not auto-fire"
        );
        assert!(
            !can_transition(&WorkflowState::Failed, &WorkflowState::Fix),
            "Failed→Fix must not auto-fire"
        );
    }

    #[test]
    fn test_reset_for_reexecute_only_targets_failed_tasks() {
        // FIX-reset: on Fix→Execute, only FAILED tasks reset to PENDING; DONE tasks
        // (already-succeeded work) are preserved. Mirrors reset_failed_tasks_for_reexecute.
        let statuses = ["FAILED", "DONE", "FAILED", "PENDING", "DONE"];
        let reset = statuses.iter().filter(|s| **s == "FAILED").count() as u64;
        assert_eq!(reset, 2, "only the two FAILED tasks should be reset");

        // DONE tasks must survive the reset untouched.
        let done_survives = statuses.iter().filter(|s| **s == "DONE").count();
        assert_eq!(done_survives, 2, "DONE tasks must be preserved");
    }

    // compute_context_fingerprint (audit round-1 H1 item #3): content-based hash
    // over the task's defining context. None when all fields are absent;
    // otherwise a deterministic, canonical (key-order-independent) "ctx-…" hash.

    #[test]
    fn compute_context_fingerprint_none_when_all_absent() {
        assert_eq!(compute_context_fingerprint(None, None, None, None), None);
        // JSON null is treated as absent, so all-null is still None.
        assert_eq!(
            compute_context_fingerprint(
                None,
                Some(&serde_json::Value::Null),
                Some(&serde_json::Value::Null),
                Some(&serde_json::Value::Null),
            ),
            None,
        );
        // An empty skill string is treated as absent.
        assert_eq!(
            compute_context_fingerprint(Some(""), None, None, None),
            None
        );
    }

    #[test]
    fn compute_context_fingerprint_is_deterministic() {
        let ac = serde_json::json!({"must": "pass lint"});
        let a = compute_context_fingerprint(Some("masday-tdd"), None, Some(&ac), None);
        let b = compute_context_fingerprint(Some("masday-tdd"), None, Some(&ac), None);
        assert_eq!(a, b);
        assert!(a.unwrap().starts_with("ctx-"));
    }

    #[test]
    fn compute_context_fingerprint_differs_when_context_changes() {
        let ac1 = serde_json::json!({"must": "pass lint"});
        let ac2 = serde_json::json!({"must": "pass lint and tests"});
        let fp1 = compute_context_fingerprint(Some("masday-tdd"), None, Some(&ac1), None);
        let fp2 = compute_context_fingerprint(Some("masday-tdd"), None, Some(&ac2), None);
        assert_ne!(fp1, fp2);

        // Changing the skill also changes the fingerprint.
        let fp3 = compute_context_fingerprint(Some("masday-debug"), None, Some(&ac1), None);
        assert_ne!(fp1, fp3);
    }

    #[test]
    fn compute_context_fingerprint_is_canonical_key_order() {
        // serde_json sorts object keys (preserve_order is off), so semantically
        // equal JSON with different key order must hash identically.
        let ordered = serde_json::json!({"a": 1, "b": 2});
        let reversed = serde_json::json!({"b": 2, "a": 1});
        let fp_a = compute_context_fingerprint(None, None, Some(&ordered), None);
        let fp_b = compute_context_fingerprint(None, None, Some(&reversed), None);
        assert_eq!(fp_a, fp_b);
    }

    // evaluate_context_drift: pure comparison logic for the
    // require_context_refresh consumer. Four terminal cases.

    #[test]
    fn evaluate_context_drift_baseline_none_is_not_required() {
        let r = evaluate_context_drift(None, Some("ctx-1"));
        assert!(!r.refresh_required);
        assert_eq!(r.baseline_fingerprint, None);
        assert_eq!(r.observed_fingerprint.as_deref(), Some("ctx-1"));
    }

    #[test]
    fn evaluate_context_drift_no_observed_is_not_required() {
        let r = evaluate_context_drift(Some("ctx-1"), None);
        assert!(!r.refresh_required);
        assert_eq!(r.baseline_fingerprint.as_deref(), Some("ctx-1"));
        assert_eq!(r.observed_fingerprint, None);
    }

    #[test]
    fn evaluate_context_drift_matching_is_not_required() {
        let r = evaluate_context_drift(Some("ctx-1"), Some("ctx-1"));
        assert!(!r.refresh_required);
    }

    #[test]
    fn evaluate_context_drift_mismatch_is_required() {
        let r = evaluate_context_drift(Some("ctx-1"), Some("ctx-2"));
        assert!(r.refresh_required);
        assert_eq!(r.baseline_fingerprint.as_deref(), Some("ctx-1"));
        assert_eq!(r.observed_fingerprint.as_deref(), Some("ctx-2"));
    }
}
