//! Review decision repository
//!
//! Table names are PascalCase (created by Drizzle/TypeScript): "ReviewDecision"
//! Column names are camelCase: "workflowId", "reviewerAgent", "testsVerified", etc.

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
        let now: chrono::NaiveDateTime = chrono::Utc::now().naive_utc();

        let query = r#"
            INSERT INTO "ReviewDecision" (
                id, "workflowId", "taskId", "reviewerAgent", decision, notes,
                gaps, "testsVerified", "testSummary", "createdAt"
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

    /// Get the latest review for a task
    pub async fn get_latest(&self, task_id: &str) -> Result<Option<ReviewDecision>> {
        let client = self
            .pool
            .get()
            .await
            .map_err(|e| AppError::Database(format!("Failed to get connection: {}", e)))?;

        let query =
            r#"SELECT * FROM "ReviewDecision" WHERE "taskId" = $1 ORDER BY "createdAt" DESC LIMIT 1"#;
        let rows = client
            .query(query, &[&task_id])
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

        let query =
            r#"SELECT * FROM "ReviewDecision" WHERE "taskId" = $1 ORDER BY "createdAt" ASC"#;
        let rows = client
            .query(query, &[&task_id])
            .await
            .map_err(|e| AppError::Database(format!("Failed to list reviews: {}", e)))?;

        Ok(rows.iter().map(ReviewDecision::from_row).collect())
    }
}
