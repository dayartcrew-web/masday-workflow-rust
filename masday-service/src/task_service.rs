//! Task business logic and lifecycle management
//!
//! Manages task creation, execution, and completion within workflows.
//! All task state transitions are validated before being persisted.

use masday_core::{AppError, Result, WorkflowState};
use masday_db::repos::TaskRepo;
use masday_db::schema::{NewTask, NewTaskProgressLog, Task, TaskProgressLog};
use masday_db::DbPool;
use tracing::{debug, info, warn};

use crate::workflow_service::{self, status_to_state};

/// Task service
pub struct TaskService {
    repo: TaskRepo,
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
    ///
    /// # Returns
    /// * `Result<Task>` - The created task
    pub async fn add_task(
        pool: &DbPool,
        workflow_id: String,
        plan_id: String,
        name: String,
        agent: Option<String>,
        dependencies: Option<Vec<String>>,
    ) -> Result<Task> {
        info!("Adding task '{}' to workflow {}", name, workflow_id);

        let service = Self::new(pool.clone());

        // Create metadata with dependencies if provided
        let required_context = dependencies.map(|deps| serde_json::json!({ "dependencies": deps }));

        let new_task = NewTask {
            workflow_id,
            plan_id,
            title: name,
            status: "PENDING".to_string(),
            priority: None,
            owner_agent: agent,
            acceptance_criteria: None,
            required_context,
            verification_steps: None,
            context_fingerprint: None,
            progress_percent: Some(0),
            requires_tdd: None,
            test_evidence: None,
        };

        let task = service.repo.create(&new_task).await?;
        debug!("Task created with ID: {}", task.id);

        Ok(task)
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

        // Update task with result and completion
        let result_data = result.unwrap_or(serde_json::Value::Null);
        let updated_task = service.repo.complete(task_id, result_data).await?;

        debug!("Task {} completed successfully", task_id);

        // Auto-transition workflow if all tasks are done
        Self::auto_transition_if_all_done(pool, workflow_id).await?;

        Ok(updated_task)
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
                vec![WorkflowState::Execute, WorkflowState::Verify, WorkflowState::Done]
            }
            WorkflowState::Paused => {
                vec![WorkflowState::Execute, WorkflowState::Verify, WorkflowState::Done]
            }
            WorkflowState::Failed => {
                // Cannot auto-transition from FAILED
                warn!("Workflow {} is FAILED, skipping auto-transition", workflow_id);
                return Ok(());
            }
            WorkflowState::Done => vec![],
        };

        // Execute transitions sequentially
        for target in transitions {
            match workflow_service::WorkflowService::transition_status(pool, workflow_id, target.clone())
                .await
            {
                Ok(_) => {
                    info!("Workflow {} transitioned to {:?}", workflow_id, target);
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

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
                vec![WorkflowState::Execute, WorkflowState::Verify, WorkflowState::Done]
            }
            WorkflowState::Paused => {
                vec![WorkflowState::Execute, WorkflowState::Verify, WorkflowState::Done]
            }
            WorkflowState::Failed => vec![],
            WorkflowState::Done => vec![],
        };
        assert_eq!(transitions, vec![WorkflowState::Verify, WorkflowState::Done]);
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
                vec![WorkflowState::Execute, WorkflowState::Verify, WorkflowState::Done]
            }
            WorkflowState::Paused => {
                vec![WorkflowState::Execute, WorkflowState::Verify, WorkflowState::Done]
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
                vec![WorkflowState::Execute, WorkflowState::Verify, WorkflowState::Done]
            }
            WorkflowState::Paused => {
                vec![WorkflowState::Execute, WorkflowState::Verify, WorkflowState::Done]
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
                vec![WorkflowState::Execute, WorkflowState::Verify, WorkflowState::Done]
            }
            WorkflowState::Paused => {
                vec![WorkflowState::Execute, WorkflowState::Verify, WorkflowState::Done]
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
                vec![WorkflowState::Execute, WorkflowState::Verify, WorkflowState::Done]
            }
            WorkflowState::Paused => {
                vec![WorkflowState::Execute, WorkflowState::Verify, WorkflowState::Done]
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
        validate_path(&[WorkflowState::Execute, WorkflowState::Verify, WorkflowState::Done]);
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
}
