//! Parallel branch repository

use crate::schema::{ParallelBranch, NewParallelBranch};
use crate::pool::DbPool;
use masday_core::{AppError, Result};
pub struct BranchRepo {
    pool: DbPool,
}

impl BranchRepo {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    /// Create multiple parallel branches for a workflow
    pub async fn create_branches(&self, branches: &[NewParallelBranch]) -> Result<Vec<ParallelBranch>> {
        let client = self.pool.get().await
            .map_err(|e| AppError::Database(format!("Failed to get connection: {}", e)))?;

        let mut result = Vec::new();
        let now: chrono::DateTime<chrono::Utc> = chrono::Utc::now();

        for branch in branches {
            let id = uuid::Uuid::new_v4().to_string();

            let query = "
                INSERT INTO parallel_branches (
                    id, workflow_id, task_id, branch_key, role, status, input, output, created_at, updated_at
                )
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
                RETURNING *
            ";

            let row = client.query_one(
                query,
                &[
                    &id, &branch.workflow_id, &branch.task_id, &branch.branch_key,
                    &branch.role, &branch.status, &branch.input, &branch.output,
                    &now, &now,
                ],
            ).await.map_err(|e| AppError::Database(format!("Failed to create branch: {}", e)))?;

            result.push(ParallelBranch {
                id: row.get("id"),
                workflow_id: row.get("workflow_id"),
                task_id: row.get("task_id"),
                branch_key: row.get("branch_key"),
                role: row.get("role"),
                status: row.get("status"),
                input: row.get("input"),
                output: row.try_get("output").unwrap_or(None),
                created_at: row.get("created_at"),
                updated_at: row.get("updated_at"),
            });
        }

        Ok(result)
    }

    /// Complete a parallel branch with output
    pub async fn complete_branch(&self, id: &str, output: serde_json::Value) -> Result<ParallelBranch> {
        let client = self.pool.get().await
            .map_err(|e| AppError::Database(format!("Failed to get connection: {}", e)))?;

        let now: chrono::DateTime<chrono::Utc> = chrono::Utc::now();
        let query = "
            UPDATE parallel_branches
            SET status = 'DONE', output = $1, updated_at = $2
            WHERE id = $3
            RETURNING *
        ";

        let row = client.query_one(query, &[&output, &now, &id]).await
            .map_err(|e| AppError::Database(format!("Failed to complete branch: {}", e)))?;

        Ok(ParallelBranch {
            id: row.get("id"),
            workflow_id: row.get("workflow_id"),
            task_id: row.get("task_id"),
            branch_key: row.get("branch_key"),
            role: row.get("role"),
            status: row.get("status"),
            input: row.get("input"),
            output: row.try_get("output").unwrap_or(None),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
        })
    }

    /// List all branches for a workflow
    pub async fn list_branches(&self, workflow_id: &str) -> Result<Vec<ParallelBranch>> {
        let client = self.pool.get().await
            .map_err(|e| AppError::Database(format!("Failed to get connection: {}", e)))?;

        let query = "SELECT * FROM parallel_branches WHERE workflow_id = $1 ORDER BY created_at ASC";
        let rows = client.query(query, &[&workflow_id]).await
            .map_err(|e| AppError::Database(format!("Failed to list branches: {}", e)))?;

        let branches = rows.iter().map(|row| ParallelBranch {
            id: row.get("id"),
            workflow_id: row.get("workflow_id"),
            task_id: row.get("task_id"),
            branch_key: row.get("branch_key"),
            role: row.get("role"),
            status: row.get("status"),
            input: row.get("input"),
            output: row.try_get("output").unwrap_or(None),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
        }).collect();

        Ok(branches)
    }

    /// Get a branch by ID
    pub async fn get_by_id(&self, id: &str) -> Result<ParallelBranch> {
        let client = self.pool.get().await
            .map_err(|e| AppError::Database(format!("Failed to get connection: {}", e)))?;

        let query = "SELECT * FROM parallel_branches WHERE id = $1";
        let row = client.query_one(query, &[&id]).await
            .map_err(|_e| AppError::not_found("ParallelBranch", id))?;

        Ok(ParallelBranch {
            id: row.get("id"),
            workflow_id: row.get("workflow_id"),
            task_id: row.get("task_id"),
            branch_key: row.get("branch_key"),
            role: row.get("role"),
            status: row.get("status"),
            input: row.get("input"),
            output: row.try_get("output").unwrap_or(None),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
        })
    }

    /// Update branch status
    pub async fn update_status(&self, id: &str, status: &str) -> Result<ParallelBranch> {
        let client = self.pool.get().await
            .map_err(|e| AppError::Database(format!("Failed to get connection: {}", e)))?;

        let now: chrono::DateTime<chrono::Utc> = chrono::Utc::now();
        let query = "UPDATE parallel_branches SET status = $1, updated_at = $2 WHERE id = $3 RETURNING *";
        let row = client.query_one(query, &[&status, &now, &id]).await
            .map_err(|e| AppError::Database(format!("Failed to update branch status: {}", e)))?;

        Ok(ParallelBranch {
            id: row.get("id"),
            workflow_id: row.get("workflow_id"),
            task_id: row.get("task_id"),
            branch_key: row.get("branch_key"),
            role: row.get("role"),
            status: row.get("status"),
            input: row.get("input"),
            output: row.try_get("output").unwrap_or(None),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
        })
    }
}
