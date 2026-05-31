//! Workflow repository

use crate::pool::DbPool;
use crate::schema::{NewWorkflow, Workflow};
use masday_core::{AppError, Result};
use tracing::debug;

pub struct WorkflowRepo {
    pool: DbPool,
}

impl WorkflowRepo {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    /// Create a new workflow
    pub async fn create(&self, workflow: &NewWorkflow) -> Result<Workflow> {
        let client = self
            .pool
            .get()
            .await
            .map_err(|e| AppError::Database(format!("Failed to get connection: {}", e)))?;

        let id = uuid::Uuid::new_v4().to_string();
        let now: chrono::DateTime<chrono::Utc> = chrono::Utc::now();

        let query = "
            INSERT INTO workflows (
                id, name, status, project_path, current_plan_id,
                current_task_id, metadata, created_at, updated_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            RETURNING *
        ";

        let row = client
            .query_one(
                query,
                &[
                    &id,
                    &workflow.name,
                    &workflow.status,
                    &workflow.project_path,
                    &workflow.current_plan_id,
                    &workflow.current_task_id,
                    &workflow.metadata,
                    &now,
                    &now,
                ],
            )
            .await
            .map_err(|e| AppError::Database(format!("Failed to create workflow: {}", e)))?;

        Ok(Workflow {
            id: row.get("id"),
            name: row.get("name"),
            status: row.get("status"),
            project_path: row.get("project_path"),
            current_plan_id: row.get("current_plan_id"),
            current_task_id: row.get("current_task_id"),
            metadata: row.try_get("metadata").unwrap_or(None),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
        })
    }

    /// Get a workflow by ID
    pub async fn get_by_id(&self, id: &str) -> Result<Workflow> {
        let client = self
            .pool
            .get()
            .await
            .map_err(|e| AppError::Database(format!("Failed to get connection: {}", e)))?;

        let query = "SELECT * FROM workflows WHERE id = $1";
        let row = client
            .query_one(query, &[&id])
            .await
            .map_err(|_e| AppError::not_found("Workflow", id))?;

        Ok(Workflow {
            id: row.get("id"),
            name: row.get("name"),
            status: row.get("status"),
            project_path: row.get("project_path"),
            current_plan_id: row.get("current_plan_id"),
            current_task_id: row.get("current_task_id"),
            metadata: row.try_get("metadata").unwrap_or(None),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
        })
    }

    /// List workflows with pagination
    pub async fn list(&self, limit: i64, offset: i64) -> Result<Vec<Workflow>> {
        let client = self
            .pool
            .get()
            .await
            .map_err(|e| AppError::Database(format!("Failed to get connection: {}", e)))?;

        let query = "SELECT * FROM workflows ORDER BY created_at DESC LIMIT $1 OFFSET $2";
        let rows = client
            .query(query, &[&limit, &offset])
            .await
            .map_err(|e| AppError::Database(format!("Failed to list workflows: {}", e)))?;

        let workflows = rows
            .iter()
            .map(|row| Workflow {
                id: row.get("id"),
                name: row.get("name"),
                status: row.get("status"),
                project_path: row.get("project_path"),
                current_plan_id: row.get("current_plan_id"),
                current_task_id: row.get("current_task_id"),
                metadata: row.try_get("metadata").unwrap_or(None),
                created_at: row.get("created_at"),
                updated_at: row.get("updated_at"),
            })
            .collect();

        Ok(workflows)
    }

    /// Get all active workflows (not DONE or FAILED)
    pub async fn get_active(&self) -> Result<Vec<Workflow>> {
        let client = self
            .pool
            .get()
            .await
            .map_err(|e| AppError::Database(format!("Failed to get connection: {}", e)))?;

        let query = "SELECT * FROM workflows WHERE status NOT IN ('DONE', 'FAILED') ORDER BY created_at DESC";
        let rows = client
            .query(query, &[])
            .await
            .map_err(|e| AppError::Database(format!("Failed to get active workflows: {}", e)))?;

        let workflows = rows
            .iter()
            .map(|row| Workflow {
                id: row.get("id"),
                name: row.get("name"),
                status: row.get("status"),
                project_path: row.get("project_path"),
                current_plan_id: row.get("current_plan_id"),
                current_task_id: row.get("current_task_id"),
                metadata: row.try_get("metadata").unwrap_or(None),
                created_at: row.get("created_at"),
                updated_at: row.get("updated_at"),
            })
            .collect();

        Ok(workflows)
    }

    /// Update workflow status
    pub async fn update_status(&self, id: &str, status: &str) -> Result<Workflow> {
        let client = self
            .pool
            .get()
            .await
            .map_err(|e| AppError::Database(format!("Failed to get connection: {}", e)))?;

        let now: chrono::DateTime<chrono::Utc> = chrono::Utc::now();
        let query = "UPDATE workflows SET status = $1, updated_at = $2 WHERE id = $3 RETURNING *";
        let row = client
            .query_one(query, &[&status, &now, &id])
            .await
            .map_err(|e| AppError::Database(format!("Failed to update workflow status: {}", e)))?;

        Ok(Workflow {
            id: row.get("id"),
            name: row.get("name"),
            status: row.get("status"),
            project_path: row.get("project_path"),
            current_plan_id: row.get("current_plan_id"),
            current_task_id: row.get("current_task_id"),
            metadata: row.try_get("metadata").unwrap_or(None),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
        })
    }

    /// Update workflow with JSON patch
    pub async fn update(&self, id: &str, updates: serde_json::Value) -> Result<Workflow> {
        let client = self
            .pool
            .get()
            .await
            .map_err(|e| AppError::Database(format!("Failed to get connection: {}", e)))?;

        let now: chrono::DateTime<chrono::Utc> = chrono::Utc::now();

        // Build dynamic UPDATE query based on provided fields
        let mut set_clauses = vec!["updated_at = $2".to_string()];
        let mut param_count = 2;
        let mut params: Vec<Box<dyn tokio_postgres::types::ToSql + Sync>> =
            vec![Box::new(id.to_string()), Box::new(now)];

        if let Some(name) = updates.get("name").and_then(|v| v.as_str()) {
            param_count += 1;
            set_clauses.push(format!("name = ${}", param_count));
            params.push(Box::new(name.to_string()));
        }
        if let Some(status) = updates.get("status").and_then(|v| v.as_str()) {
            param_count += 1;
            set_clauses.push(format!("status = ${}", param_count));
            params.push(Box::new(status.to_string()));
        }
        if let Some(project_path) = updates.get("project_path").and_then(|v| v.as_str()) {
            param_count += 1;
            set_clauses.push(format!("project_path = ${}", param_count));
            params.push(Box::new(project_path.to_string()));
        }
        if let Some(current_plan_id) = updates.get("current_plan_id").and_then(|v| v.as_str()) {
            param_count += 1;
            set_clauses.push(format!("current_plan_id = ${}", param_count));
            params.push(Box::new(current_plan_id.to_string()));
        }
        if let Some(current_task_id) = updates.get("current_task_id").and_then(|v| v.as_str()) {
            param_count += 1;
            set_clauses.push(format!("current_task_id = ${}", param_count));
            params.push(Box::new(current_task_id.to_string()));
        }
        if let Some(metadata) = updates.get("metadata") {
            param_count += 1;
            set_clauses.push(format!("metadata = ${}", param_count));
            params.push(Box::new(metadata.clone()));
        }

        let query = format!(
            "UPDATE workflows SET {} WHERE id = $1 RETURNING *",
            set_clauses.join(", ")
        );

        debug!("Executing update query: {}", query);

        // Convert params to slice of references
        let params_refs: Vec<&(dyn tokio_postgres::types::ToSql + Sync)> =
            params.iter().map(|p| p.as_ref()).collect();

        let row = client
            .query_one(&query, params_refs.as_slice())
            .await
            .map_err(|e| AppError::Database(format!("Failed to update workflow: {}", e)))?;

        Ok(Workflow {
            id: row.get("id"),
            name: row.get("name"),
            status: row.get("status"),
            project_path: row.get("project_path"),
            current_plan_id: row.get("current_plan_id"),
            current_task_id: row.get("current_task_id"),
            metadata: row.try_get("metadata").unwrap_or(None),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
        })
    }

    /// Delete a workflow
    pub async fn delete(&self, id: &str) -> Result<bool> {
        let client = self
            .pool
            .get()
            .await
            .map_err(|e| AppError::Database(format!("Failed to get connection: {}", e)))?;

        let query = "DELETE FROM workflows WHERE id = $1";
        let rows_affected = client
            .execute(query, &[&id])
            .await
            .map_err(|e| AppError::Database(format!("Failed to delete workflow: {}", e)))?;

        Ok(rows_affected > 0)
    }
}
