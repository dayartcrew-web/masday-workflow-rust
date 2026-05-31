//! Parallel branch repository
//!
//! Table names are PascalCase (created by Drizzle/TypeScript): "ParallelBranch"
//! Column names are camelCase: "workflowId", "taskId", "branchKey", etc.

use crate::pool::DbPool;
use crate::schema::{NewParallelBranch, ParallelBranch};
use masday_core::{AppError, Result};
pub struct BranchRepo {
    pool: DbPool,
}

impl BranchRepo {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    /// Create multiple parallel branches for a workflow
    pub async fn create_branches(
        &self,
        branches: &[NewParallelBranch],
    ) -> Result<Vec<ParallelBranch>> {
        let client = self
            .pool
            .get()
            .await
            .map_err(|e| AppError::Database(format!("Failed to get connection: {}", e)))?;

        let mut result = Vec::new();
        let now: chrono::NaiveDateTime = chrono::Utc::now().naive_utc();

        for branch in branches {
            let id = uuid::Uuid::new_v4().to_string();

            let query = r#"
                INSERT INTO "ParallelBranch" (
                    id, "workflowId", "taskId", "branchKey", role, status, input, output, "createdAt", "updatedAt"
                )
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
                RETURNING *
            "#;

            let row = client
                .query_one(
                    query,
                    &[
                        &id,
                        &branch.workflow_id,
                        &branch.task_id,
                        &branch.branch_key,
                        &branch.role,
                        &branch.status,
                        &branch.input,
                        &branch.output,
                        &now,
                        &now,
                    ],
                )
                .await
                .map_err(|e| AppError::Database(format!("Failed to create branch: {}", e)))?;

            result.push(ParallelBranch::from_row(&row));
        }

        Ok(result)
    }

    /// Complete a parallel branch with output
    pub async fn complete_branch(
        &self,
        id: &str,
        output: serde_json::Value,
    ) -> Result<ParallelBranch> {
        let client = self
            .pool
            .get()
            .await
            .map_err(|e| AppError::Database(format!("Failed to get connection: {}", e)))?;

        let now: chrono::NaiveDateTime = chrono::Utc::now().naive_utc();
        let query = r#"
            UPDATE "ParallelBranch"
            SET status = 'DONE', output = $1, "updatedAt" = $2
            WHERE id = $3
            RETURNING *
        "#;

        let row = client
            .query_one(query, &[&output, &now, &id])
            .await
            .map_err(|e| AppError::Database(format!("Failed to complete branch: {}", e)))?;

        Ok(ParallelBranch::from_row(&row))
    }

    /// List all branches for a workflow
    pub async fn list_branches(&self, workflow_id: &str) -> Result<Vec<ParallelBranch>> {
        let client = self
            .pool
            .get()
            .await
            .map_err(|e| AppError::Database(format!("Failed to get connection: {}", e)))?;

        let query =
            r#"SELECT * FROM "ParallelBranch" WHERE "workflowId" = $1 ORDER BY "createdAt" ASC"#;
        let rows = client
            .query(query, &[&workflow_id])
            .await
            .map_err(|e| AppError::Database(format!("Failed to list branches: {}", e)))?;

        Ok(rows.iter().map(|r| ParallelBranch::from_row(r)).collect())
    }

    /// Get a branch by ID
    pub async fn get_by_id(&self, id: &str) -> Result<ParallelBranch> {
        let client = self
            .pool
            .get()
            .await
            .map_err(|e| AppError::Database(format!("Failed to get connection: {}", e)))?;

        let query = r#"SELECT * FROM "ParallelBranch" WHERE id = $1"#;
        let row = client
            .query_one(query, &[&id])
            .await
            .map_err(|_e| AppError::not_found("ParallelBranch", id))?;

        Ok(ParallelBranch::from_row(&row))
    }

    /// Update branch status
    pub async fn update_status(&self, id: &str, status: &str) -> Result<ParallelBranch> {
        let client = self
            .pool
            .get()
            .await
            .map_err(|e| AppError::Database(format!("Failed to get connection: {}", e)))?;

        let now: chrono::NaiveDateTime = chrono::Utc::now().naive_utc();
        let query =
            r#"UPDATE "ParallelBranch" SET status = $1, "updatedAt" = $2 WHERE id = $3 RETURNING *"#;
        let row = client
            .query_one(query, &[&status, &now, &id])
            .await
            .map_err(|e| AppError::Database(format!("Failed to update branch status: {}", e)))?;

        Ok(ParallelBranch::from_row(&row))
    }
}
