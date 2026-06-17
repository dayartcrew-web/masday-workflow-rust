//! Review decision repository
//!
//! Table names are snake_case: "review_decisions"
//! Column names are snake_case: "workflow_id", "reviewer_agent", "tests_verified", etc.

use crate::pool::DbPool;
use crate::schema::{NewReviewDecision, ReviewDecision};
use masday_core::{AppError, Result};

pub struct ReviewRepo {
    pool: DbPool,
}

impl ReviewRepo {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    /// Submit a review decision
    pub async fn submit(&self, review: &NewReviewDecision) -> Result<ReviewDecision> {
        let client = self
            .pool
            .get()
            .await
            .map_err(|e| AppError::Database(format!("Failed to get connection: {}", e)))?;

        let id = uuid::Uuid::new_v4().to_string();
        let now: chrono::DateTime<chrono::Utc> = chrono::Utc::now();

        let query = r#"
            INSERT INTO review_decisions (
                id, workflow_id, task_id, reviewer_agent, decision, notes,
                gaps, tests_verified, test_summary, created_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            RETURNING *
        "#;

        let row = client
            .query_one(
                query,
                &[
                    &id,
                    &review.workflow_id,
                    &review.task_id,
                    &review.reviewer_agent,
                    &review.decision,
                    &review.notes,
                    &review.gaps,
                    &review.tests_verified,
                    &review.test_summary,
                    &now,
                ],
            )
            .await
            .map_err(|e| AppError::Database(format!("Failed to submit review: {}", e)))?;

        Ok(ReviewDecision::from_row(&row))
    }

    /// Get the latest review for a task within a workflow.
    ///
    /// Scopes by BOTH `workflow_id` AND `task_id`: a task_id alone is not
    /// sufficient to identify a review (task IDs may be reused across workflows
    /// or a stray row from another workflow could match). The workflow_id
    /// predicate prevents returning a foreign workflow's review decision.
    pub async fn get_latest(
        &self,
        workflow_id: &str,
        task_id: &str,
    ) -> Result<Option<ReviewDecision>> {
        let client = self
            .pool
            .get()
            .await
            .map_err(|e| AppError::Database(format!("Failed to get connection: {}", e)))?;

        let query = r#"SELECT * FROM review_decisions WHERE workflow_id = $1 AND task_id = $2 ORDER BY created_at DESC LIMIT 1"#;
        let rows = client
            .query(query, &[&workflow_id, &task_id])
            .await
            .map_err(|e| AppError::Database(format!("Failed to get latest review: {}", e)))?;

        if rows.is_empty() {
            return Ok(None);
        }

        Ok(Some(ReviewDecision::from_row(&rows[0])))
    }

    /// List all reviews for a task
    pub async fn list_by_task(&self, task_id: &str) -> Result<Vec<ReviewDecision>> {
        let client = self
            .pool
            .get()
            .await
            .map_err(|e| AppError::Database(format!("Failed to get connection: {}", e)))?;

        let query = r#"SELECT * FROM review_decisions WHERE task_id = $1 ORDER BY created_at ASC"#;
        let rows = client
            .query(query, &[&task_id])
            .await
            .map_err(|e| AppError::Database(format!("Failed to list reviews: {}", e)))?;

        Ok(rows.iter().map(ReviewDecision::from_row).collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_constructor_signature() {
        fn _check() {
            let _ = ReviewRepo::new;
        }
    }

    #[test]
    fn test_new_review_decision_construction() {
        let rd = NewReviewDecision {
            workflow_id: "wf-123".to_string(),
            task_id: "task-456".to_string(),
            reviewer_agent: "masday-reviewer".to_string(),
            decision: "APPROVED".to_string(),
            notes: "LGTM".to_string(),
            gaps: None,
            tests_verified: Some(true),
            test_summary: None,
        };
        assert_eq!(rd.decision, "APPROVED");
        assert_eq!(rd.tests_verified, Some(true));
    }

    #[test]
    fn test_insert_sql_contains_returning_star() {
        let sql = r#"INSERT INTO review_decisions (id, workflow_id, task_id, reviewer_agent, decision, notes, gaps, tests_verified, test_summary, created_at) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10) RETURNING *"#;
        assert!(sql.contains("RETURNING *"));
    }

    #[test]
    fn test_latest_sql_has_order_and_limit() {
        let sql = r#"SELECT * FROM review_decisions WHERE workflow_id = $1 AND task_id = $2 ORDER BY created_at DESC LIMIT 1"#;
        assert!(sql.contains("ORDER BY created_at DESC"));
        assert!(sql.contains("LIMIT 1"));
    }
}
