//! Workflow business logic and state machine
//!
//! Provides the core state machine logic for workflow lifecycle management.
//! All state transitions are validated against the allowed transitions defined
//! in the WorkflowState enum before being applied to the database.

use masday_core::{AppError, Result, WorkflowState};
use masday_db::repos::WorkflowRepo;
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
        let metadata = if let Some(desc) = description {
            serde_json::json!({ "description": desc })
        } else {
            serde_json::Value::Null
        };

        let new_workflow = NewWorkflow {
            name,
            status: "INIT".to_string(),
            project_path,
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
            WorkflowState::Execute | WorkflowState::Verify | WorkflowState::Fix | WorkflowState::Done
        ) {
            return Ok(workflow);
        }

        // Auto-advance through intermediate states to reach EXECUTE
        match current {
            WorkflowState::Init => {
                Self::transition_status(pool, id, WorkflowState::Analyze).await?;
                Self::transition_status(pool, id, WorkflowState::Plan).await?;
                Self::transition_status(pool, id, WorkflowState::Execute).await
            }
            WorkflowState::Analyze => {
                Self::transition_status(pool, id, WorkflowState::Plan).await?;
                Self::transition_status(pool, id, WorkflowState::Execute).await
            }
            WorkflowState::Plan => {
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

        // Validate transition
        if !can_transition(&current_state, &new_status) {
            return Err(AppError::validation(format!(
                "Invalid state transition: {:?} → {:?}",
                current_state, new_status
            )));
        }

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

    /// List workflows with pagination
    ///
    /// # Arguments
    /// * `pool` - Database connection pool
    /// * `limit` - Maximum number of results
    /// * `offset` - Number of results to skip
    ///
    /// # Returns
    /// * `Result<Vec<Workflow>>` - List of workflows
    pub async fn list_workflows(pool: &DbPool, limit: i64, offset: i64) -> Result<Vec<Workflow>> {
        debug!("Listing workflows: limit={}, offset={}", limit, offset);
        let service = Self::new(pool.clone());
        service.repo.list(limit, offset).await
    }

    /// Get all active workflows (not DONE or FAILED)
    ///
    /// # Arguments
    /// * `pool` - Database connection pool
    ///
    /// # Returns
    /// * `Result<Vec<Workflow>>` - List of active workflows
    pub async fn get_active_workflows(pool: &DbPool) -> Result<Vec<Workflow>> {
        debug!("Getting active workflows");
        let service = Self::new(pool.clone());
        service.repo.get_active().await
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
}
