//! Task repository
//!
//! Table names are PascalCase (created by Drizzle/TypeScript): "Task", "TaskProgressLog"
//! Column names are camelCase: "workflowId", "ownerAgent", etc.

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
        let now: chrono::NaiveDateTime = chrono::Utc::now().naive_utc();

        let query = r#"
            INSERT INTO "Task" (
                id, "workflowId", "planId", title, status, priority,
                "ownerAgent", "acceptanceCriteria", "requiredContext",
                "verificationSteps", "contextFingerprint", "progressPercent",
                "requiresTdd", "testEvidence", "createdAt", "updatedAt"
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16)
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
                    &task.acceptance_criteria,
                    &task.required_context,
                    &task.verification_steps,
                    &task.context_fingerprint,
                    &task.progress_percent,
                    &task.requires_tdd,
                    &task.test_evidence,
                    &now,
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

        let query = r#"SELECT * FROM "Task" WHERE id = $1"#;
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

        let query = r#"SELECT * FROM "Task" WHERE "workflowId" = $1 ORDER BY "createdAt" ASC"#;
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
        let query = r#"SELECT * FROM "Task" WHERE "workflowId" = $1 AND status = 'RUNNING' ORDER BY "createdAt" ASC LIMIT 1"#;
        let rows = client
            .query(query, &[&workflow_id])
            .await
            .map_err(|e| AppError::Database(format!("Failed to get current task: {}", e)))?;

        if !rows.is_empty() {
            return Ok(Some(Task::from_row(&rows[0])));
        }

        // If no RUNNING task, get first PENDING task
        let query = r#"SELECT * FROM "Task" WHERE "workflowId" = $1 AND status = 'PENDING' ORDER BY "createdAt" ASC LIMIT 1"#;
        let rows = client
            .query(query, &[&workflow_id])
            .await
            .map_err(|e| AppError::Database(format!("Failed to get current task: {}", e)))?;

        if rows.is_empty() {
            return Ok(None);
        }

        Ok(Some(Task::from_row(&rows[0])))
    }

    /// Update task status
    pub async fn update_status(&self, id: &str, status: &str) -> Result<Task> {
        let client = self
            .pool
            .get()
            .await
            .map_err(|e| AppError::Database(format!("Failed to get connection: {}", e)))?;

        let now: chrono::NaiveDateTime = chrono::Utc::now().naive_utc();
        let query = r#"UPDATE "Task" SET status = $1, "updatedAt" = $2 WHERE id = $3 RETURNING *"#;
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

        let now: chrono::NaiveDateTime = chrono::Utc::now().naive_utc();
        let query = r#"
            UPDATE "Task"
            SET status = 'DONE', "testEvidence" = $1, "progressPercent" = 100, "updatedAt" = $2
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
        let now: chrono::NaiveDateTime = chrono::Utc::now().naive_utc();

        let query = r#"
            INSERT INTO "TaskProgressLog" (
                id, "workflowId", "taskId", "agentName", "statusBefore",
                "statusAfter", "progressNote", evidence, "createdAt"
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
}
