//! Workflow repository
//!
//! Table names are PascalCase (created by Drizzle/TypeScript): "Workflow"
//! Column names are camelCase: "projectPath", "currentPlanId", etc.

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
        let now: chrono::NaiveDateTime = chrono::Utc::now().naive_utc();

        let query = r#"
            INSERT INTO "Workflow" (
                id, name, status, "projectPath", "currentPlanId",
                "currentTaskId", metadata, "createdAt", "updatedAt"
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            RETURNING *
        "#;

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

        Ok(Workflow::from_row(&row))
    }

    /// Get a workflow by ID
    pub async fn get_by_id(&self, id: &str) -> Result<Workflow> {
        let client = self
            .pool
            .get()
            .await
            .map_err(|e| AppError::Database(format!("Failed to get connection: {}", e)))?;

        let query = r#"SELECT * FROM "Workflow" WHERE id = $1"#;
        let row = client
            .query_one(query, &[&id])
            .await
            .map_err(|_e| AppError::not_found("Workflow", id))?;

        Ok(Workflow::from_row(&row))
    }

    /// List workflows with pagination, optionally filtered by project_path
    pub async fn list(
        &self,
        limit: i64,
        offset: i64,
        project_path: Option<&str>,
    ) -> Result<Vec<Workflow>> {
        let client = self
            .pool
            .get()
            .await
            .map_err(|e| AppError::Database(format!("Failed to get connection: {}", e)))?;

        let rows = if let Some(pp) = project_path {
            let query = r#"SELECT * FROM "Workflow" WHERE "projectPath" = $1 ORDER BY "createdAt" DESC LIMIT $2 OFFSET $3"#;
            client
                .query(query, &[&pp, &limit, &offset])
                .await
                .map_err(|e| AppError::Database(format!("Failed to list workflows: {}", e)))?
        } else {
            let query = r#"SELECT * FROM "Workflow" ORDER BY "createdAt" DESC LIMIT $1 OFFSET $2"#;
            client
                .query(query, &[&limit, &offset])
                .await
                .map_err(|e| AppError::Database(format!("Failed to list workflows: {}", e)))?
        };

        Ok(rows.iter().map(Workflow::from_row).collect())
    }

    /// Get all active workflows (not DONE or FAILED), optionally filtered by project_path
    pub async fn get_active(&self, project_path: Option<&str>) -> Result<Vec<Workflow>> {
        let client = self
            .pool
            .get()
            .await
            .map_err(|e| AppError::Database(format!("Failed to get connection: {}", e)))?;

        let rows = if let Some(pp) = project_path {
            let query = r#"SELECT * FROM "Workflow" WHERE status NOT IN ('DONE', 'FAILED') AND "projectPath" = $1 ORDER BY "createdAt" DESC"#;
            client
                .query(query, &[&pp])
                .await
                .map_err(|e| AppError::Database(format!("Failed to get active workflows: {}", e)))?
        } else {
            let query = r#"SELECT * FROM "Workflow" WHERE status NOT IN ('DONE', 'FAILED') ORDER BY "createdAt" DESC"#;
            client
                .query(query, &[])
                .await
                .map_err(|e| AppError::Database(format!("Failed to get active workflows: {}", e)))?
        };

        Ok(rows.iter().map(Workflow::from_row).collect())
    }

    /// Update workflow status
    pub async fn update_status(&self, id: &str, status: &str) -> Result<Workflow> {
        let client = self
            .pool
            .get()
            .await
            .map_err(|e| AppError::Database(format!("Failed to get connection: {}", e)))?;

        let now: chrono::NaiveDateTime = chrono::Utc::now().naive_utc();
        let query =
            r#"UPDATE "Workflow" SET status = $1, "updatedAt" = $2 WHERE id = $3 RETURNING *"#;
        let row = client
            .query_one(query, &[&status, &now, &id])
            .await
            .map_err(|e| AppError::Database(format!("Failed to update workflow status: {}", e)))?;

        Ok(Workflow::from_row(&row))
    }

    /// Update workflow with JSON patch
    pub async fn update(&self, id: &str, updates: serde_json::Value) -> Result<Workflow> {
        let client = self
            .pool
            .get()
            .await
            .map_err(|e| AppError::Database(format!("Failed to get connection: {}", e)))?;

        let now: chrono::NaiveDateTime = chrono::Utc::now().naive_utc();

        // Build dynamic UPDATE query based on provided fields
        let mut set_clauses = vec![r#""updatedAt" = $2"#.to_string()];
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
            set_clauses.push(format!(r#""projectPath" = ${}"#, param_count));
            params.push(Box::new(project_path.to_string()));
        }
        if let Some(current_plan_id) = updates.get("current_plan_id").and_then(|v| v.as_str()) {
            param_count += 1;
            set_clauses.push(format!(r#""currentPlanId" = ${}"#, param_count));
            params.push(Box::new(current_plan_id.to_string()));
        }
        if let Some(current_task_id) = updates.get("current_task_id").and_then(|v| v.as_str()) {
            param_count += 1;
            set_clauses.push(format!(r#""currentTaskId" = ${}"#, param_count));
            params.push(Box::new(current_task_id.to_string()));
        }
        if let Some(metadata) = updates.get("metadata") {
            param_count += 1;
            set_clauses.push(format!("metadata = ${}", param_count));
            params.push(Box::new(metadata.clone()));
        }

        let query = format!(
            r#"UPDATE "Workflow" SET {} WHERE id = $1 RETURNING *"#,
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

        Ok(Workflow::from_row(&row))
    }

    /// Delete a workflow
    pub async fn delete(&self, id: &str) -> Result<bool> {
        let client = self
            .pool
            .get()
            .await
            .map_err(|e| AppError::Database(format!("Failed to get connection: {}", e)))?;

        let query = r#"DELETE FROM "Workflow" WHERE id = $1"#;
        let rows_affected = client
            .execute(query, &[&id])
            .await
            .map_err(|e| AppError::Database(format!("Failed to delete workflow: {}", e)))?;

        Ok(rows_affected > 0)
    }
}
