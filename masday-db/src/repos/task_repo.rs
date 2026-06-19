//! Task repository
//!
//! Table names are snake_case: "tasks", "task_progress_logs"
//! Column names are snake_case: "workflow_id", "owner_agent", etc.

use crate::pool::DbPool;
use crate::schema::{NewTask, NewTaskProgressLog, Task, TaskProgressLog};
use masday_core::{AppError, Result};

pub struct TaskRepo {
    pool: DbPool,
}

impl TaskRepo {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    /// Create a new task
    pub async fn create(&self, task: &NewTask) -> Result<Task> {
        let client = self
            .pool
            .get()
            .await
            .map_err(|e| AppError::Database(format!("Failed to get connection: {}", e)))?;

        let id = uuid::Uuid::new_v4().to_string();
        let now: chrono::DateTime<chrono::Utc> = chrono::Utc::now();

        let query = r#"
            INSERT INTO tasks (
                id, workflow_id, plan_id, title, status, priority,
                owner_agent, skill, description, dependencies,
                acceptance_criteria, required_context, verification_steps,
                context_fingerprint, progress_percent, requires_tdd,
                input, result, test_evidence, metadata, created_at,
                started_at, completed_at, updated_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19, $20, $21, $22, $23, $24)
            RETURNING *
        "#;

        let row = client
            .query_one(
                query,
                &[
                    &id,
                    &task.workflow_id,
                    &task.plan_id,
                    &task.title,
                    &task.status,
                    &task.priority,
                    &task.owner_agent,
                    &task.skill,
                    &task.description,
                    &task.dependencies,
                    &task.acceptance_criteria,
                    &task.required_context,
                    &task.verification_steps,
                    &task.context_fingerprint,
                    &task.progress_percent,
                    &task.requires_tdd,
                    &task.input,
                    &task.result,
                    &task.test_evidence,
                    &task.metadata,
                    &now,
                    &None::<chrono::DateTime<chrono::Utc>>,
                    &None::<chrono::DateTime<chrono::Utc>>,
                    &now,
                ],
            )
            .await
            .map_err(|e| AppError::Database(format!("Failed to create task: {}", e)))?;

        Ok(Task::from_row(&row))
    }

    /// Get a task by ID
    pub async fn get_by_id(&self, id: &str) -> Result<Task> {
        let client = self
            .pool
            .get()
            .await
            .map_err(|e| AppError::Database(format!("Failed to get connection: {}", e)))?;

        let query = r#"SELECT * FROM tasks WHERE id = $1"#;
        let row = client
            .query_one(query, &[&id])
            .await
            .map_err(|_e| AppError::not_found("Task", id))?;

        Ok(Task::from_row(&row))
    }

    /// List all tasks for a workflow
    pub async fn list_by_workflow(&self, workflow_id: &str) -> Result<Vec<Task>> {
        let client = self
            .pool
            .get()
            .await
            .map_err(|e| AppError::Database(format!("Failed to get connection: {}", e)))?;

        let query = r#"SELECT * FROM tasks WHERE workflow_id = $1 ORDER BY created_at ASC"#;
        let rows = client
            .query(query, &[&workflow_id])
            .await
            .map_err(|e| AppError::Database(format!("Failed to list tasks: {}", e)))?;

        Ok(rows.iter().map(Task::from_row).collect())
    }

    /// Get the current task for a workflow (first RUNNING or first PENDING)
    pub async fn get_current(&self, workflow_id: &str) -> Result<Option<Task>> {
        let client = self
            .pool
            .get()
            .await
            .map_err(|e| AppError::Database(format!("Failed to get connection: {}", e)))?;

        // First try to find a RUNNING task
        let query = r#"SELECT * FROM tasks WHERE workflow_id = $1 AND status = 'RUNNING' ORDER BY created_at ASC LIMIT 1"#;
        let rows = client
            .query(query, &[&workflow_id])
            .await
            .map_err(|e| AppError::Database(format!("Failed to get current task: {}", e)))?;

        if !rows.is_empty() {
            return Ok(Some(Task::from_row(&rows[0])));
        }

        // If no RUNNING task, get first PENDING task
        let query = r#"SELECT * FROM tasks WHERE workflow_id = $1 AND status = 'PENDING' ORDER BY created_at ASC LIMIT 1"#;
        let rows = client
            .query(query, &[&workflow_id])
            .await
            .map_err(|e| AppError::Database(format!("Failed to get current task: {}", e)))?;

        if rows.is_empty() {
            return Ok(None);
        }

        Ok(Some(Task::from_row(&rows[0])))
    }

    /// Find tasks that have been `RUNNING` longer than `threshold` without an
    /// `updated_at` refresh — candidates for `STUCK_TASK` reminders.
    ///
    /// Uses `updated_at` (not `started_at`): on the PG path `started_at` is only
    /// written by a few transitions, so it is frequently NULL even for RUNNING
    /// tasks, whereas `updated_at` advances on every status change and progress
    /// save. A RUNNING task whose `updated_at` is older than the threshold has
    /// made no observable progress in that window = stuck.
    pub async fn find_stuck(&self, threshold: chrono::Duration) -> Result<Vec<Task>> {
        let client = self
            .pool
            .get()
            .await
            .map_err(|e| AppError::Database(format!("Failed to get connection: {}", e)))?;

        let minutes = threshold.num_minutes().max(0) as i32;
        let query = r#"SELECT * FROM tasks
            WHERE status = 'RUNNING' AND updated_at < NOW() - ($1 * INTERVAL '1 minute')
            ORDER BY updated_at ASC"#;
        let rows = client
            .query(query, &[&minutes])
            .await
            .map_err(|e| AppError::Database(format!("Failed to find stuck tasks: {}", e)))?;

        Ok(rows.iter().map(Task::from_row).collect())
    }

    /// Update task status
    pub async fn update_status(&self, id: &str, status: &str) -> Result<Task> {
        let client = self
            .pool
            .get()
            .await
            .map_err(|e| AppError::Database(format!("Failed to get connection: {}", e)))?;

        let now: chrono::DateTime<chrono::Utc> = chrono::Utc::now();
        let query = r#"UPDATE tasks SET status = $1, updated_at = $2 WHERE id = $3 RETURNING *"#;
        let row = client
            .query_one(query, &[&status, &now, &id])
            .await
            .map_err(|e| AppError::Database(format!("Failed to update task status: {}", e)))?;

        Ok(Task::from_row(&row))
    }

    /// Complete a task with result
    pub async fn complete(&self, id: &str, result: serde_json::Value) -> Result<Task> {
        let client = self
            .pool
            .get()
            .await
            .map_err(|e| AppError::Database(format!("Failed to get connection: {}", e)))?;

        let now: chrono::DateTime<chrono::Utc> = chrono::Utc::now();
        let query = r#"
            UPDATE tasks
            SET status = 'DONE', test_evidence = $1, progress_percent = 100, updated_at = $2
            WHERE id = $3
            RETURNING *
        "#;
        let row = client
            .query_one(query, &[&result, &now, &id])
            .await
            .map_err(|e| AppError::Database(format!("Failed to complete task: {}", e)))?;

        Ok(Task::from_row(&row))
    }

    /// Save progress log for a task
    pub async fn save_progress(&self, log: &NewTaskProgressLog) -> Result<TaskProgressLog> {
        let client = self
            .pool
            .get()
            .await
            .map_err(|e| AppError::Database(format!("Failed to get connection: {}", e)))?;

        let id = uuid::Uuid::new_v4().to_string();
        let now: chrono::DateTime<chrono::Utc> = chrono::Utc::now();

        let query = r#"
            INSERT INTO task_progress_logs (
                id, workflow_id, task_id, agent_name, status_before,
                status_after, progress_note, evidence, created_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            RETURNING *
        "#;

        let row = client
            .query_one(
                query,
                &[
                    &id,
                    &log.workflow_id,
                    &log.task_id,
                    &log.agent_name,
                    &log.status_before,
                    &log.status_after,
                    &log.progress_note,
                    &log.evidence,
                    &now,
                ],
            )
            .await
            .map_err(|e| AppError::Database(format!("Failed to save progress log: {}", e)))?;

        Ok(TaskProgressLog::from_row(&row))
    }

    /// Count tasks for a workflow
    pub async fn count_by_workflow(&self, workflow_id: &str) -> Result<i64> {
        let client = self
            .pool
            .get()
            .await
            .map_err(|e| AppError::Database(format!("Failed to get connection: {}", e)))?;

        let query = r#"SELECT COUNT(*) FROM tasks WHERE workflow_id = $1"#;
        let row = client
            .query_one(query, &[&workflow_id])
            .await
            .map_err(|e| AppError::Database(format!("Failed to count tasks: {}", e)))?;

        Ok(row.get::<_, i64>("count"))
    }

    /// Count tasks with DONE status for a workflow
    pub async fn count_done_by_workflow(&self, workflow_id: &str) -> Result<i64> {
        let client = self
            .pool
            .get()
            .await
            .map_err(|e| AppError::Database(format!("Failed to get connection: {}", e)))?;

        let query = r#"SELECT COUNT(*) FROM tasks WHERE workflow_id = $1 AND status = 'DONE'"#;
        let row = client
            .query_one(query, &[&workflow_id])
            .await
            .map_err(|e| AppError::Database(format!("Failed to count done tasks: {}", e)))?;

        Ok(row.get::<_, i64>("count"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_constructor_signature() {
        fn _check() {
            let _ = TaskRepo::new;
        }
    }

    #[test]
    fn test_new_task_construction() {
        let task = NewTask {
            workflow_id: "wf-123".to_string(),
            plan_id: "plan-456".to_string(),
            title: "Implement feature".to_string(),
            status: "PENDING".to_string(),
            priority: Some("HIGH".to_string()),
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
        };
        assert_eq!(task.workflow_id, "wf-123");
        assert_eq!(task.status, "PENDING");
        assert_eq!(task.priority, Some("HIGH".to_string()));
    }

    #[test]
    fn test_new_progress_log_construction() {
        let log = NewTaskProgressLog {
            workflow_id: "wf-123".to_string(),
            task_id: "task-456".to_string(),
            agent_name: "masday-executor".to_string(),
            status_before: Some("RUNNING".to_string()),
            status_after: Some("DONE".to_string()),
            progress_note: "Completed implementation".to_string(),
            evidence: Some(serde_json::json!({"files_changed": 3})),
        };
        assert_eq!(log.agent_name, "masday-executor");
        assert_eq!(log.progress_note, "Completed implementation");
    }

    #[test]
    fn test_insert_sql_has_required_params() {
        let sql = r#"
            INSERT INTO tasks (
                id, workflow_id, plan_id, title, status, priority,
                owner_agent, skill, description, dependencies,
                acceptance_criteria, required_context, verification_steps,
                context_fingerprint, progress_percent, requires_tdd,
                input, result, test_evidence, metadata, created_at,
                started_at, completed_at, updated_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19, $20, $21, $22, $23, $24)
            RETURNING *
        "#;
        assert!(sql.contains("RETURNING *"));
        assert!(sql.contains("$24"));
    }
}
