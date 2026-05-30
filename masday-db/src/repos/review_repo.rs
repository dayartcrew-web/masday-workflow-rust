//! Review decision repository

use crate::schema::{ReviewDecision, NewReviewDecision};
use deadpool_postgres::Pool;
use masday_core::{AppError, Result};

pub struct ReviewRepo {
    pool: Pool,
}

impl ReviewRepo {
    pub fn new(pool: Pool) -> Self {
        Self { pool }
    }

    /// Submit a review decision
    pub async fn submit(&self, review: &NewReviewDecision) -> Result<ReviewDecision> {
        let client = self.pool.get().await
            .map_err(|e| AppError::Database(format!("Failed to get connection: {}", e)))?;

        let id = uuid::Uuid::new_v4().to_string();
        let now: chrono::DateTime<chrono::Utc> = chrono::Utc::now();

        let query = "
            INSERT INTO review_decisions (
                id, workflow_id, task_id, reviewer_agent, decision, notes,
                gaps, tests_verified, test_summary, created_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            RETURNING *
        ";

        let row = client.query_one(
            query,
            &[
                &id, &review.workflow_id, &review.task_id, &review.reviewer_agent,
                &review.decision, &review.notes, &review.gaps, &review.tests_verified,
                &review.test_summary, &now,
            ],
        ).await.map_err(|e| AppError::Database(format!("Failed to submit review: {}", e)))?;

        Ok(ReviewDecision {
            id: row.get("id"),
            workflow_id: row.get("workflow_id"),
            task_id: row.get("task_id"),
            reviewer_agent: row.get("reviewer_agent"),
            decision: row.get("decision"),
            notes: row.get("notes"),
            gaps: row.try_get("gaps").unwrap_or(None),
            tests_verified: row.get("tests_verified"),
            test_summary: row.try_get("test_summary").unwrap_or(None),
            created_at: row.get("created_at"),
        })
    }

    /// Get the latest review for a task
    pub async fn get_latest(&self, task_id: &str) -> Result<Option<ReviewDecision>> {
        let client = self.pool.get().await
            .map_err(|e| AppError::Database(format!("Failed to get connection: {}", e)))?;

        let query = "SELECT * FROM review_decisions WHERE task_id = $1 ORDER BY created_at DESC LIMIT 1";
        let rows = client.query(query, &[&task_id]).await
            .map_err(|e| AppError::Database(format!("Failed to get latest review: {}", e)))?;

        if rows.is_empty() {
            return Ok(None);
        }

        let row = &rows[0];
        Ok(Some(ReviewDecision {
            id: row.get("id"),
            workflow_id: row.get("workflow_id"),
            task_id: row.get("task_id"),
            reviewer_agent: row.get("reviewer_agent"),
            decision: row.get("decision"),
            notes: row.get("notes"),
            gaps: row.try_get("gaps").unwrap_or(None),
            tests_verified: row.get("tests_verified"),
            test_summary: row.try_get("test_summary").unwrap_or(None),
            created_at: row.get("created_at"),
        }))
    }

    /// List all reviews for a task
    pub async fn list_by_task(&self, task_id: &str) -> Result<Vec<ReviewDecision>> {
        let client = self.pool.get().await
            .map_err(|e| AppError::Database(format!("Failed to get connection: {}", e)))?;

        let query = "SELECT * FROM review_decisions WHERE task_id = $1 ORDER BY created_at ASC";
        let rows = client.query(query, &[&task_id]).await
            .map_err(|e| AppError::Database(format!("Failed to list reviews: {}", e)))?;

        let reviews = rows.iter().map(|row| ReviewDecision {
            id: row.get("id"),
            workflow_id: row.get("workflow_id"),
            task_id: row.get("task_id"),
            reviewer_agent: row.get("reviewer_agent"),
            decision: row.get("decision"),
            notes: row.get("notes"),
            gaps: row.try_get("gaps").unwrap_or(None),
            tests_verified: row.get("tests_verified"),
            test_summary: row.try_get("test_summary").unwrap_or(None),
            created_at: row.get("created_at"),
        }).collect();

        Ok(reviews)
    }
}
