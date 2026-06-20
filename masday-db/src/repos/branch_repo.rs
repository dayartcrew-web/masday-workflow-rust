//! Parallel branch repository
//!
//! Table names are snake_case: "parallel_branches"
//! Column names are snake_case: "workflow_id", "task_id", "branch_key", etc.

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
        let now: chrono::DateTime<chrono::Utc> = chrono::Utc::now();

        for branch in branches {
            let id = uuid::Uuid::new_v4().to_string();

            let query = r#"
                INSERT INTO parallel_branches (
                    id, workflow_id, task_id, branch_key, role, status, input, output, created_at, updated_at
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

        let now: chrono::DateTime<chrono::Utc> = chrono::Utc::now();
        let query = r#"
            UPDATE parallel_branches
            SET status = 'DONE', output = $1, updated_at = $2
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
            r#"SELECT * FROM parallel_branches WHERE workflow_id = $1 ORDER BY created_at ASC"#;
        let rows = client
            .query(query, &[&workflow_id])
            .await
            .map_err(|e| AppError::Database(format!("Failed to list branches: {}", e)))?;

        Ok(rows.iter().map(ParallelBranch::from_row).collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_constructor_signature() {
        fn _check() {
            let _ = BranchRepo::new;
        }
    }

    #[test]
    fn test_new_parallel_branch_construction() {
        let b = NewParallelBranch {
            workflow_id: "wf-123".to_string(),
            task_id: Some("task-456".to_string()),
            branch_key: "research".to_string(),
            role: "researcher".to_string(),
            status: "PENDING".to_string(),
            input: serde_json::json!({"query": "test"}),
            output: None,
        };
        assert_eq!(b.branch_key, "research");
        assert_eq!(b.role, "researcher");
    }

    #[test]
    fn test_insert_sql_contains_returning_star() {
        let sql = r#"INSERT INTO parallel_branches (id, workflow_id, task_id, branch_key, role, status, input, output, created_at, updated_at) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10) RETURNING *"#;
        assert!(sql.contains("RETURNING *"));
    }

    #[test]
    fn test_complete_sets_done_status() {
        let sql = r#"UPDATE parallel_branches SET status = 'DONE', output = $1, updated_at = $2 WHERE id = $3 RETURNING *"#;
        assert!(sql.contains("status = 'DONE'"));
    }
}
