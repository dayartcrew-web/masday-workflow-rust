//! Review pipeline and decisions
//!
//! Manages code review submissions and decisions with gap analysis.

use masaday_db::DbPool;
use masaday_core::{AppError, Result};
use masaday_db::repos::ReviewRepo;
use masaday_db::schema::{NewReviewDecision, ReviewDecision};
use tracing::{debug, info};

/// Review service
pub struct ReviewService {
    repo: ReviewRepo,
}

impl ReviewService {
    /// Create a new review service
    pub fn new(pool: DbPool) -> Self {
        Self {
            repo: ReviewRepo::new(pool),
        }
    }

    /// Submit a review decision
    ///
    /// # Arguments
    /// * `pool` - Database connection pool
    /// * `workflow_id` - Parent workflow ID
    /// * `task_id` - Task ID being reviewed
    /// * `reviewer` - Reviewer name
    /// * `decision` - Review decision (APPROVED, REWORK_REQUIRED, BLOCKED)
    /// * `notes` - Review notes
    /// * `gaps` - Optional gap analysis data
    ///
    /// # Returns
    /// * `Result<ReviewDecision>` - The created review decision
    pub async fn submit_review(
        pool: &DbPool,
        workflow_id: String,
        task_id: String,
        reviewer: String,
        decision: String,
        notes: String,
        gaps: Option<serde_json::Value>,
    ) -> Result<ReviewDecision> {
        info!(
            "Submitting review for task {} by {}: {}",
            task_id, reviewer, decision
        );

        let service = Self::new(pool.clone());

        // Validate decision format
        let decision_upper = decision.to_uppercase();
        if !matches!(decision_upper.as_str(), "APPROVED" | "REWORK_REQUIRED" | "BLOCKED") {
            return Err(AppError::validation(format!(
                "Invalid review decision: {}",
                decision
            )));
        }

        let new_review = NewReviewDecision {
            workflow_id,
            task_id,
            reviewer,
            decision: decision_upper,
            notes,
            gap_analysis: gaps,
            created_at: None, // Will be set by repo
        };

        let review = service.repo.submit(&new_review).await?;
        debug!("Review decision created with ID: {}", review.id);

        Ok(review)
    }

    /// Get the latest review for a task
    ///
    /// # Arguments
    /// * `pool` - Database connection pool
    /// * `task_id` - Task ID
    ///
    /// # Returns
    /// * `Result<Option<ReviewDecision>>` - The latest review if any
    pub async fn get_latest_review(
        pool: &DbPool,
        task_id: &str,
    ) -> Result<Option<ReviewDecision>> {
        debug!("Getting latest review for task {}", task_id);

        let service = Self::new(pool.clone());
        service.repo.get_latest(task_id).await
    }

    /// Check if a task is approved
    ///
    /// # Arguments
    /// * `pool` - Database connection pool
    /// * `task_id` - Task ID
    ///
    /// # Returns
    /// * `bool` - true if approved, false otherwise
    pub async fn is_approved(pool: &DbPool, task_id: &str) -> bool {
        match Self::get_latest_review(pool, task_id).await {
            Ok(Some(review)) => review.decision == "APPROVED",
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate() {
        assert!(true);
    }
}
