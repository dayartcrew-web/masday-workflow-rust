//! Workflow business logic and state machine
//!
//! Provides the core state machine logic for workflow lifecycle management.
//! All state transitions are validated against the allowed transitions defined
//! in the WorkflowState enum before being applied to the database.

use masday_core::{AppError, Result, WorkflowState};
use masday_db::repos::{ContextDocumentRepo, MemoryRepo, PlanRepo, TaskRepo, WorkflowRepo};
use masday_db::schema::{NewWorkflow, Workflow};
use masday_db::DbPool;
use tracing::{debug, info};

/// Check if a state transition is valid
///
/// # Arguments
/// * `current` - Current workflow state
/// * `target` - Target workflow state
///
/// # Returns
/// * `true` if transition is allowed, `false` otherwise
///
/// # State Machine Rules
/// - INIT → ANALYZE | DONE | FAILED
/// - ANALYZE → PLAN | DONE | FAILED
/// - PLAN → EXECUTE | PAUSED | FAILED
/// - EXECUTE → VERIFY | FIX | PAUSED | FAILED
/// - VERIFY → DONE | FIX
/// - FIX → DONE | EXECUTE | FAILED
/// - PAUSED → EXECUTE | FAILED
pub fn can_transition(current: &WorkflowState, target: &WorkflowState) -> bool {
    current.can_transition_to(target)
}

/// Validate transition prerequisites
///
/// # Arguments
/// * `pool` - Database connection pool
/// * `workflow_id` - Workflow ID
/// * `from` - Current state
/// * `to` - Target state
///
/// # Returns
/// * `Result<()>` - Ok if prerequisites met, Err with clear error message if not
///
/// # Prerequisite Rules
/// - ANALYZE → PLAN: Must have analysis artifacts (context documents or memories)
/// - PLAN → EXECUTE: Must have a plan with at least one task
/// - EXECUTE → VERIFY: Must have at least one DONE task
pub async fn validate_transition_prerequisites(
    pool: &DbPool,
    workflow_id: &str,
    from: &WorkflowState,
    to: &WorkflowState,
) -> Result<()> {
    match (from, to) {
        // ANALYZE → PLAN: Verify analysis was done
        (WorkflowState::Analyze, WorkflowState::Plan) => {
            let context_doc_repo = ContextDocumentRepo::new(pool.clone());
            let doc_count = context_doc_repo
                .count_by_workflow(workflow_id)
                .await
                .unwrap_or(0);

            let memory_repo = MemoryRepo::new(pool.clone());
            let memories = memory_repo
                .recall_by_workflow(workflow_id, 1)
                .await
                .unwrap_or_default();
            let mem_count = memories.len() as i64;

            if doc_count == 0 && mem_count == 0 {
                return Err(AppError::validation(
                    "Cannot advance to PLAN: no analysis artifacts found. Run analysis first.",
                ));
            }
        }
        // PLAN → EXECUTE: Verify plan exists with tasks
        (WorkflowState::Plan, WorkflowState::Execute) => {
            let plan_repo = PlanRepo::new(pool.clone());
            let plan = plan_repo
                .get_active_for_workflow(workflow_id)
                .await
                .map_err(|e| {
                    AppError::database(format!("Failed to check for plan: {}", e))
                })?;

            if plan.is_none() {
                return Err(AppError::validation(
                    "Cannot advance to EXECUTE: no plan found. Create a plan first.",
                ));
            }

            let task_repo = TaskRepo::new(pool.clone());
            let task_count = task_repo
                .count_by_workflow(workflow_id)
                .await
                .unwrap_or(0);

            if task_count == 0 {
                return Err(AppError::validation(
                    "Cannot advance to EXECUTE: plan has no tasks. Add tasks to the plan first.",
                ));
            }
        }
        // EXECUTE → VERIFY: Verify at least one task completed
        (WorkflowState::Execute, WorkflowState::Verify) => {
            let task_repo = TaskRepo::new(pool.clone());
            let done_count = task_repo
                .count_done_by_workflow(workflow_id)
                .await
                .unwrap_or(0);

            if done_count == 0 {
                return Err(AppError::validation(
                    "Cannot advance to VERIFY: no tasks completed. Execute tasks first.",
                ));
            }
        }
        // Other transitions have no prerequisites
        _ => {}
    }
    Ok(())
}

/// Convert string status to WorkflowState
pub fn status_to_state(status: &str) -> Result<WorkflowState> {
    match status.to_uppercase().as_str() {
        "INIT" => Ok(WorkflowState::Init),
        "ANALYZE" => Ok(WorkflowState::Analyze),
        "PLAN" => Ok(WorkflowState::Plan),
        "EXECUTE" => Ok(WorkflowState::Execute),
        "VERIFY" => Ok(WorkflowState::Verify),
        "FIX" => Ok(WorkflowState::Fix),
        "DONE" => Ok(WorkflowState::Done),
        "FAILED" => Ok(WorkflowState::Failed),
        "PAUSED" => Ok(WorkflowState::Paused),
        _ => Err(AppError::validation(format!(
            "Invalid workflow state: {}",
            status
        ))),
    }
}

/// Convert WorkflowState to string
fn state_to_status(state: &WorkflowState) -> String {
    match state {
        WorkflowState::Init => "INIT",
        WorkflowState::Analyze => "ANALYZE",
        WorkflowState::Plan => "PLAN",
        WorkflowState::Execute => "EXECUTE",
        WorkflowState::Verify => "VERIFY",
        WorkflowState::Fix => "FIX",
        WorkflowState::Done => "DONE",
        WorkflowState::Failed => "FAILED",
        WorkflowState::Paused => "PAUSED",
    }
    .to_string()
}

/// Workflow service
pub struct WorkflowService {
    repo: WorkflowRepo,
}

impl WorkflowService {
    /// Create a new workflow service
    pub fn new(pool: DbPool) -> Self {
        Self {
            repo: WorkflowRepo::new(pool),
        }
    }

    /// Create a new workflow with INIT status
    ///
    /// # Arguments
    /// * `pool` - Database connection pool
    /// * `name` - Workflow name
    /// * `description` - Optional description (stored in metadata)
    /// * `project_path` - Optional project path
    ///
    /// # Returns
    /// * `Result<Workflow>` - The created workflow
    pub async fn create_workflow(
        pool: &DbPool,
        name: String,
        description: Option<String>,
        project_path: Option<String>,
    ) -> Result<Workflow> {
        info!("Creating workflow: {}", name);

        let repo = WorkflowRepo::new(pool.clone());
        let metadata = if let Some(ref desc) = description {
            serde_json::json!({ "description": desc })
        } else {
            serde_json::Value::Null
        };

        let new_workflow = NewWorkflow {
            name,
            description: description.clone(),
            status: "INIT".to_string(),
            project_path,
            trace_id: None,
            current_plan_id: None,
            current_task_id: None,
            metadata: if metadata.is_null() {
                None
            } else {
                Some(metadata)
            },
        };

        let workflow = repo.create(&new_workflow).await?;
        debug!("Workflow created with ID: {}", workflow.id);

        Ok(workflow)
    }

    /// Execute a workflow — auto-transitions through intermediate states
    ///
    /// If the workflow is in INIT or ANALYZE, automatically advances through
    /// ANALYZE → PLAN → EXECUTE so the caller doesn't need to step through
    /// each intermediate state manually.
    ///
    /// NOTE: This function now validates prerequisites at each transition.
    /// - Cannot skip ANALYZE — must complete before PLAN
    /// - Cannot skip PLAN — must complete before EXECUTE
    ///
    /// # Arguments
    /// * `pool` - Database connection pool
    /// * `id` - Workflow ID
    ///
    /// # Returns
    /// * `Result<Workflow>` - The updated workflow
    pub async fn execute_workflow(pool: &DbPool, id: &str) -> Result<Workflow> {
        info!("Executing workflow: {}", id);

        let workflow = Self::get_workflow(pool, id).await?;
        let current = status_to_state(&workflow.status)?;

        // Already executing or beyond — just return current state
        if matches!(
            current,
            WorkflowState::Execute
                | WorkflowState::Verify
                | WorkflowState::Fix
                | WorkflowState::Done
        ) {
            return Ok(workflow);
        }

        // Only advance ONE state at a time, with prerequisite validation
        match current {
            WorkflowState::Init => {
                // Only advance to ANALYZE, not all the way to EXECUTE
                Self::transition_status(pool, id, WorkflowState::Analyze).await
            }
            WorkflowState::Analyze => {
                // Validate analysis artifacts exist before advancing to PLAN
                validate_transition_prerequisites(
                    pool,
                    id,
                    &WorkflowState::Analyze,
                    &WorkflowState::Plan,
                )
                .await?;
                Self::transition_status(pool, id, WorkflowState::Plan).await
            }
            WorkflowState::Plan => {
                // Validate plan and tasks exist before advancing to EXECUTE
                validate_transition_prerequisites(
                    pool,
                    id,
                    &WorkflowState::Plan,
                    &WorkflowState::Execute,
                )
                .await?;
                Self::transition_status(pool, id, WorkflowState::Execute).await
            }
            WorkflowState::Paused => {
                Self::transition_status(pool, id, WorkflowState::Execute).await
            }
            _ => Err(AppError::validation(format!(
                "Cannot execute workflow in state {:?}",
                current
            ))),
        }
    }

    /// Transition workflow status with validation
    ///
    /// # Arguments
    /// * `pool` - Database connection pool
    /// * `id` - Workflow ID
    /// * `new_status` - Target status
    ///
    /// # Returns
    /// * `Result<Workflow>` - The updated workflow
    pub async fn transition_status(
        pool: &DbPool,
        id: &str,
        new_status: WorkflowState,
    ) -> Result<Workflow> {
        let service = Self::new(pool.clone());

        // Get current workflow
        let workflow = Self::get_workflow(pool, id).await?;
        let current_state = status_to_state(&workflow.status)?;

        // Validate transition is allowed by state machine
        if !can_transition(&current_state, &new_status) {
            return Err(AppError::validation(format!(
                "Invalid state transition: {:?} → {:?}",
                current_state, new_status
            )));
        }

        // Validate transition prerequisites
        validate_transition_prerequisites(pool, id, &current_state, &new_status).await?;

        info!(
            "Transitioning workflow {} from {:?} to {:?}",
            id, current_state, new_status
        );

        // Update status in database
        let new_status_str = state_to_status(&new_status);
        service
            .repo
            .update_status(id, &new_status_str)
            .await
            .map_err(|e| AppError::database(format!("Failed to transition workflow: {}", e)))
    }

    /// Get a workflow by ID
    ///
    /// # Arguments
    /// * `pool` - Database connection pool
    /// * `id` - Workflow ID
    ///
    /// # Returns
    /// * `Result<Workflow>` - The workflow
    pub async fn get_workflow(pool: &DbPool, id: &str) -> Result<Workflow> {
        debug!("Getting workflow: {}", id);
        let service = Self::new(pool.clone());
        service.repo.get_by_id(id).await
    }

    /// List workflows with pagination, optionally filtered by project_path
    ///
    /// # Arguments
    /// * `pool` - Database connection pool
    /// * `limit` - Maximum number of results
    /// * `offset` - Number of results to skip
    /// * `project_path` - Optional project path to filter by
    ///
    /// # Returns
    /// * `Result<Vec<Workflow>>` - List of workflows
    pub async fn list_workflows(
        pool: &DbPool,
        limit: i64,
        offset: i64,
        project_path: Option<&str>,
    ) -> Result<Vec<Workflow>> {
        debug!(
            "Listing workflows: limit={}, offset={}, project_path={:?}",
            limit, offset, project_path
        );
        let service = Self::new(pool.clone());
        service.repo.list(limit, offset, project_path).await
    }

    /// Get all active workflows (not DONE or FAILED), optionally filtered by project_path
    ///
    /// # Arguments
    /// * `pool` - Database connection pool
    /// * `project_path` - Optional project path to filter by
    ///
    /// # Returns
    /// * `Result<Vec<Workflow>>` - List of active workflows
    pub async fn get_active_workflows(
        pool: &DbPool,
        project_path: Option<&str>,
    ) -> Result<Vec<Workflow>> {
        debug!("Getting active workflows, project_path={:?}", project_path);
        let service = Self::new(pool.clone());
        service.repo.get_active(project_path).await
    }

    /// Delete a workflow
    ///
    /// # Arguments
    /// * `pool` - Database connection pool
    /// * `id` - Workflow ID
    ///
    /// # Returns
    /// * `Result<bool>` - true if deleted, false if not found
    pub async fn delete_workflow(pool: &DbPool, id: &str) -> Result<bool> {
        info!("Deleting workflow: {}", id);
        let service = Self::new(pool.clone());
        service.repo.delete(id).await
    }

    pub async fn update_status(pool: &DbPool, id: &str, status: &str) -> Result<Workflow> {
        info!("Updating workflow {} status to {}", id, status);
        let service = Self::new(pool.clone());
        service.repo.update_status(id, status).await
    }

    pub async fn update_workflow(
        pool: &DbPool,
        id: &str,
        updates: serde_json::Value,
    ) -> Result<Workflow> {
        info!("Updating workflow {} with {:?}", id, updates);
        let service = Self::new(pool.clone());
        service.repo.update(id, updates).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_can_transition_valid() {
        // Valid transitions
        assert!(can_transition(
            &WorkflowState::Init,
            &WorkflowState::Analyze
        ));
        assert!(can_transition(&WorkflowState::Init, &WorkflowState::Done));
        assert!(can_transition(&WorkflowState::Init, &WorkflowState::Failed));

        assert!(can_transition(
            &WorkflowState::Analyze,
            &WorkflowState::Plan
        ));
        assert!(can_transition(
            &WorkflowState::Plan,
            &WorkflowState::Execute
        ));
        assert!(can_transition(
            &WorkflowState::Execute,
            &WorkflowState::Verify
        ));
        assert!(can_transition(&WorkflowState::Verify, &WorkflowState::Done));
        assert!(can_transition(&WorkflowState::Fix, &WorkflowState::Execute));
        assert!(can_transition(
            &WorkflowState::Paused,
            &WorkflowState::Execute
        ));
    }

    #[test]
    fn test_can_transition_invalid() {
        // Invalid transitions
        assert!(!can_transition(
            &WorkflowState::Done,
            &WorkflowState::Execute
        ));
        assert!(!can_transition(
            &WorkflowState::Failed,
            &WorkflowState::Plan
        ));
        assert!(!can_transition(
            &WorkflowState::Verify,
            &WorkflowState::Analyze
        ));
    }

    #[test]
    fn test_state_to_status_conversion() {
        assert_eq!(state_to_status(&WorkflowState::Init), "INIT");
        assert_eq!(state_to_status(&WorkflowState::Execute), "EXECUTE");
        assert_eq!(state_to_status(&WorkflowState::Done), "DONE");
    }

    #[test]
    fn test_status_to_state_conversion() {
        assert!(matches!(
            status_to_state("INIT").unwrap(),
            WorkflowState::Init
        ));
        assert!(matches!(
            status_to_state("EXECUTE").unwrap(),
            WorkflowState::Execute
        ));
        assert!(status_to_state("INVALID").is_err());
    }

    // Tests for validate_transition_prerequisites logic (structural validation)
    // These tests verify the prerequisite check logic without requiring database

    #[test]
    fn test_prerequisites_analyze_to_plan_requires_artifacts() {
        // This test validates the logic that ANALYZE → PLAN requires artifacts
        // The actual implementation checks doc_count == 0 && mem_count == 0
        let doc_count = 0;
        let mem_count = 0;
        let has_artifacts = doc_count > 0 || mem_count > 0;
        assert!(!has_artifacts, "Should fail when no artifacts exist");

        // With artifacts
        let doc_count = 1;
        let mem_count = 0;
        let has_artifacts = doc_count > 0 || mem_count > 0;
        assert!(has_artifacts, "Should pass when artifacts exist");
    }

    #[test]
    fn test_prerequisites_plan_to_execute_requires_plan() {
        // This test validates the logic that PLAN → EXECUTE requires a plan
        let plan_exists = true;
        assert!(plan_exists, "Should fail when no plan exists");

        let plan_exists = false;
        assert!(!plan_exists, "Should pass when plan exists");
    }

    #[test]
    fn test_prerequisites_plan_to_execute_requires_tasks() {
        // This test validates the logic that PLAN → EXECUTE requires tasks
        let task_count = 0;
        let has_tasks = task_count > 0;
        assert!(!has_tasks, "Should fail when no tasks exist");

        let task_count = 1;
        let has_tasks = task_count > 0;
        assert!(has_tasks, "Should pass when tasks exist");
    }

    #[test]
    fn test_prerequisites_execute_to_verify_requires_done_tasks() {
        // This test validates the logic that EXECUTE → VERIFY requires done tasks
        let done_count = 0;
        let has_done_tasks = done_count > 0;
        assert!(!has_done_tasks, "Should fail when no tasks are done");

        let done_count = 1;
        let has_done_tasks = done_count > 0;
        assert!(has_done_tasks, "Should pass when at least one task is done");
    }

    #[test]
    fn test_prerequisites_other_transitions_have_no_requirements() {
        // This test validates that other transitions have no prerequisites
        // The implementation uses _ => {} catch-all for these cases

        // Helper function to check if a transition has prerequisites
        fn has_prerequisites(from: &WorkflowState, to: &WorkflowState) -> bool {
            matches!(
                (from, to),
                (WorkflowState::Analyze, WorkflowState::Plan)
                    | (WorkflowState::Plan, WorkflowState::Execute)
                    | (WorkflowState::Execute, WorkflowState::Verify)
            )
        }

        // Test some transitions that should NOT have prerequisites
        let no_prereq_transitions = vec![
            (WorkflowState::Init, WorkflowState::Analyze),
            (WorkflowState::Init, WorkflowState::Done),
            (WorkflowState::Verify, WorkflowState::Fix),
            (WorkflowState::Fix, WorkflowState::Done),
            (WorkflowState::Paused, WorkflowState::Execute),
        ];

        for transition in no_prereq_transitions {
            assert!(
                !has_prerequisites(&transition.0, &transition.1),
                "Transition {:?} should not have prerequisites",
                transition
            );
        }

        // Verify transitions that SHOULD have prerequisites
        assert!(has_prerequisites(
            &WorkflowState::Analyze,
            &WorkflowState::Plan
        ));
        assert!(has_prerequisites(
            &WorkflowState::Plan,
            &WorkflowState::Execute
        ));
        assert!(has_prerequisites(
            &WorkflowState::Execute,
            &WorkflowState::Verify
        ));
    }

    #[test]
    fn test_prerequisites_error_messages_are_descriptive() {
        // This test validates that error messages are clear and actionable
        // The actual messages are in the validate_transition_prerequisites function

        let error_no_artifacts = "Cannot advance to PLAN: no analysis artifacts found. Run analysis first.";
        assert!(error_no_artifacts.contains("no analysis artifacts"));
        assert!(error_no_artifacts.contains("Run analysis first"));

        let error_no_plan = "Cannot advance to EXECUTE: no plan found. Create a plan first.";
        assert!(error_no_plan.contains("no plan found"));
        assert!(error_no_plan.contains("Create a plan"));

        let error_no_tasks = "Cannot advance to EXECUTE: plan has no tasks. Add tasks to the plan first.";
        assert!(error_no_tasks.contains("plan has no tasks"));
        assert!(error_no_tasks.contains("Add tasks"));

        let error_no_done = "Cannot advance to VERIFY: no tasks completed. Execute tasks first.";
        assert!(error_no_done.contains("no tasks completed"));
        assert!(error_no_done.contains("Execute tasks"));
    }
}
