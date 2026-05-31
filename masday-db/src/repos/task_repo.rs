//! Task repository

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

        let query = "
            INSERT INTO tasks (
                id, workflow_id, plan_id, title, status, priority, owner_agent,
                acceptance_criteria, required_context, verification_steps,
                context_fingerprint, progress_percent, requires_tdd, test_evidence,
                created_at, updated_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16)
            RETURNING *
        ";

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

        Ok(Task {
            id: row.get("id"),
            workflow_id: row.get("workflow_id"),
            plan_id: row.get("plan_id"),
            title: row.get("title"),
            status: row.get("status"),
            priority: row.get("priority"),
            owner_agent: row.get("owner_agent"),
            acceptance_criteria: row.try_get("acceptance_criteria").unwrap_or(None),
            required_context: row.try_get("required_context").unwrap_or(None),
            verification_steps: row.try_get("verification_steps").unwrap_or(None),
            context_fingerprint: row.get("context_fingerprint"),
            progress_percent: row.get("progress_percent"),
            requires_tdd: row.get("requires_tdd"),
            test_evidence: row.try_get("test_evidence").unwrap_or(None),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
        })
    }

    /// Get a task by ID
    pub async fn get_by_id(&self, id: &str) -> Result<Task> {
        let client = self
            .pool
            .get()
            .await
            .map_err(|e| AppError::Database(format!("Failed to get connection: {}", e)))?;

        let query = "SELECT * FROM tasks WHERE id = $1";
        let row = client
            .query_one(query, &[&id])
            .await
            .map_err(|_e| AppError::not_found("Task", id))?;

        Ok(Task {
            id: row.get("id"),
            workflow_id: row.get("workflow_id"),
            plan_id: row.get("plan_id"),
            title: row.get("title"),
            status: row.get("status"),
            priority: row.get("priority"),
            owner_agent: row.get("owner_agent"),
            acceptance_criteria: row.try_get("acceptance_criteria").unwrap_or(None),
            required_context: row.try_get("required_context").unwrap_or(None),
            verification_steps: row.try_get("verification_steps").unwrap_or(None),
            context_fingerprint: row.get("context_fingerprint"),
            progress_percent: row.get("progress_percent"),
            requires_tdd: row.get("requires_tdd"),
            test_evidence: row.try_get("test_evidence").unwrap_or(None),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
        })
    }

    /// List all tasks for a workflow
    pub async fn list_by_workflow(&self, workflow_id: &str) -> Result<Vec<Task>> {
        let client = self
            .pool
            .get()
            .await
            .map_err(|e| AppError::Database(format!("Failed to get connection: {}", e)))?;

        let query = "SELECT * FROM tasks WHERE workflow_id = $1 ORDER BY created_at ASC";
        let rows = client
            .query(query, &[&workflow_id])
            .await
            .map_err(|e| AppError::Database(format!("Failed to list tasks: {}", e)))?;

        let tasks = rows
            .iter()
            .map(|row| Task {
                id: row.get("id"),
                workflow_id: row.get("workflow_id"),
                plan_id: row.get("plan_id"),
                title: row.get("title"),
                status: row.get("status"),
                priority: row.get("priority"),
                owner_agent: row.get("owner_agent"),
                acceptance_criteria: row.try_get("acceptance_criteria").unwrap_or(None),
                required_context: row.try_get("required_context").unwrap_or(None),
                verification_steps: row.try_get("verification_steps").unwrap_or(None),
                context_fingerprint: row.get("context_fingerprint"),
                progress_percent: row.get("progress_percent"),
                requires_tdd: row.get("requires_tdd"),
                test_evidence: row.try_get("test_evidence").unwrap_or(None),
                created_at: row.get("created_at"),
                updated_at: row.get("updated_at"),
            })
            .collect();

        Ok(tasks)
    }

    /// Get the current task for a workflow (first RUNNING or first PENDING)
    pub async fn get_current(&self, workflow_id: &str) -> Result<Option<Task>> {
        let client = self
            .pool
            .get()
            .await
            .map_err(|e| AppError::Database(format!("Failed to get connection: {}", e)))?;

        // First try to find a RUNNING task
        let query = "SELECT * FROM tasks WHERE workflow_id = $1 AND status = 'RUNNING' ORDER BY created_at ASC LIMIT 1";
        let rows = client
            .query(query, &[&workflow_id])
            .await
            .map_err(|e| AppError::Database(format!("Failed to get current task: {}", e)))?;

        if !rows.is_empty() {
            let row = &rows[0];
            return Ok(Some(Task {
                id: row.get("id"),
                workflow_id: row.get("workflow_id"),
                plan_id: row.get("plan_id"),
                title: row.get("title"),
                status: row.get("status"),
                priority: row.get("priority"),
                owner_agent: row.get("owner_agent"),
                acceptance_criteria: row.try_get("acceptance_criteria").unwrap_or(None),
                required_context: row.try_get("required_context").unwrap_or(None),
                verification_steps: row.try_get("verification_steps").unwrap_or(None),
                context_fingerprint: row.get("context_fingerprint"),
                progress_percent: row.get("progress_percent"),
                requires_tdd: row.get("requires_tdd"),
                test_evidence: row.try_get("test_evidence").unwrap_or(None),
                created_at: row.get("created_at"),
                updated_at: row.get("updated_at"),
            }));
        }

        // If no RUNNING task, get first PENDING task
        let query = "SELECT * FROM tasks WHERE workflow_id = $1 AND status = 'PENDING' ORDER BY created_at ASC LIMIT 1";
        let rows = client
            .query(query, &[&workflow_id])
            .await
            .map_err(|e| AppError::Database(format!("Failed to get current task: {}", e)))?;

        if rows.is_empty() {
            return Ok(None);
        }

        let row = &rows[0];
        Ok(Some(Task {
            id: row.get("id"),
            workflow_id: row.get("workflow_id"),
            plan_id: row.get("plan_id"),
            title: row.get("title"),
            status: row.get("status"),
            priority: row.get("priority"),
            owner_agent: row.get("owner_agent"),
            acceptance_criteria: row.try_get("acceptance_criteria").unwrap_or(None),
            required_context: row.try_get("required_context").unwrap_or(None),
            verification_steps: row.try_get("verification_steps").unwrap_or(None),
            context_fingerprint: row.get("context_fingerprint"),
            progress_percent: row.get("progress_percent"),
            requires_tdd: row.get("requires_tdd"),
            test_evidence: row.try_get("test_evidence").unwrap_or(None),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
        }))
    }

    /// Update task status
    pub async fn update_status(&self, id: &str, status: &str) -> Result<Task> {
        let client = self
            .pool
            .get()
            .await
            .map_err(|e| AppError::Database(format!("Failed to get connection: {}", e)))?;

        let now: chrono::DateTime<chrono::Utc> = chrono::Utc::now();
        let query = "UPDATE tasks SET status = $1, updated_at = $2 WHERE id = $3 RETURNING *";
        let row = client
            .query_one(query, &[&status, &now, &id])
            .await
            .map_err(|e| AppError::Database(format!("Failed to update task status: {}", e)))?;

        Ok(Task {
            id: row.get("id"),
            workflow_id: row.get("workflow_id"),
            plan_id: row.get("plan_id"),
            title: row.get("title"),
            status: row.get("status"),
            priority: row.get("priority"),
            owner_agent: row.get("owner_agent"),
            acceptance_criteria: row.try_get("acceptance_criteria").unwrap_or(None),
            required_context: row.try_get("required_context").unwrap_or(None),
            verification_steps: row.try_get("verification_steps").unwrap_or(None),
            context_fingerprint: row.get("context_fingerprint"),
            progress_percent: row.get("progress_percent"),
            requires_tdd: row.get("requires_tdd"),
            test_evidence: row.try_get("test_evidence").unwrap_or(None),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
        })
    }

    /// Complete a task with result
    pub async fn complete(&self, id: &str, result: serde_json::Value) -> Result<Task> {
        let client = self
            .pool
            .get()
            .await
            .map_err(|e| AppError::Database(format!("Failed to get connection: {}", e)))?;

        let now: chrono::DateTime<chrono::Utc> = chrono::Utc::now();
        let query = "
            UPDATE tasks
            SET status = 'DONE', test_evidence = $1, progress_percent = 100, updated_at = $2
            WHERE id = $3
            RETURNING *
        ";
        let row = client
            .query_one(query, &[&result, &now, &id])
            .await
            .map_err(|e| AppError::Database(format!("Failed to complete task: {}", e)))?;

        Ok(Task {
            id: row.get("id"),
            workflow_id: row.get("workflow_id"),
            plan_id: row.get("plan_id"),
            title: row.get("title"),
            status: row.get("status"),
            priority: row.get("priority"),
            owner_agent: row.get("owner_agent"),
            acceptance_criteria: row.try_get("acceptance_criteria").unwrap_or(None),
            required_context: row.try_get("required_context").unwrap_or(None),
            verification_steps: row.try_get("verification_steps").unwrap_or(None),
            context_fingerprint: row.get("context_fingerprint"),
            progress_percent: row.get("progress_percent"),
            requires_tdd: row.get("requires_tdd"),
            test_evidence: row.try_get("test_evidence").unwrap_or(None),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
        })
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

        let query = "
            INSERT INTO task_progress_logs (
                id, workflow_id, task_id, agent_name, status_before,
                status_after, progress_note, evidence, created_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            RETURNING *
        ";

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

        Ok(TaskProgressLog {
            id: row.get("id"),
            workflow_id: row.get("workflow_id"),
            task_id: row.get("task_id"),
            agent_name: row.get("agent_name"),
            status_before: row.get("status_before"),
            status_after: row.get("status_after"),
            progress_note: row.get("progress_note"),
            evidence: row.try_get("evidence").unwrap_or(None),
            created_at: row.get("created_at"),
        })
    }
}
