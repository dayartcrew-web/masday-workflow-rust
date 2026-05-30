//! Task business logic and lifecycle management
//!
//! Manages task creation, execution, and completion within workflows.
//! All task state transitions are validated before being persisted.

use deadpool_postgres::Pool as DbPool;
use masaday_core::{AppError, Result};
use masaday_db::repos::TaskRepo;
use masaday_db::schema::{NewTask, Task, TaskProgressLog, NewTaskProgressLog};
use tracing::{debug, info};

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
        let required_context = if let Some(deps) = dependencies {
            Some(serde_json::json!({ "dependencies": deps }))
        } else {
            None
        };

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
    pub async fn start_task(
        pool: &DbPool,
        workflow_id: &str,
        task_id: &str,
    ) -> Result<Task> {
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
        let updated_task = service
            .repo
            .update_status(task_id, "RUNNING")
            .await?;

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
    pub async fn get_current_task(
        pool: &DbPool,
        workflow_id: &str,
    ) -> Result<Option<Task>> {
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
    pub async fn list_tasks(
        pool: &DbPool,
        workflow_id: &str,
    ) -> Result<Vec<Task>> {
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate() {
        // Placeholder test
        assert!(true);
    }
}
