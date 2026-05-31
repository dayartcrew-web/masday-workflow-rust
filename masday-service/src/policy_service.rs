//! Policy validation and drift detection
//!
//! Provides validation functions for workflow execution and completion,
//! plus basic scope drift detection using keyword analysis.

use masday_core::{AppError, Result};
use masday_db::repos::{ReviewRepo, TaskRepo};
use masday_db::DbPool;
use tracing::{debug, info};

/// Policy service
pub struct PolicyService {
    task_repo: TaskRepo,
    review_repo: ReviewRepo,
}

impl PolicyService {
    /// Create a new policy service
    pub fn new(pool: DbPool) -> Self {
        Self {
            task_repo: TaskRepo::new(pool.clone()),
            review_repo: ReviewRepo::new(pool),
        }
    }

    /// Validate that a task is ready for execution
    ///
    /// # Arguments
    /// * `session_key` - Optional session identifier (for future session validation)
    /// * `workflow_id` - Workflow ID
    /// * `task_id` - Task ID
    ///
    /// # Returns
    /// * `Result<bool>` - true if execution is validated
    pub async fn validate_execution(
        pool: &DbPool,
        _session_key: Option<&str>,
        workflow_id: &str,
        task_id: &str,
    ) -> Result<bool> {
        debug!(
            "Validating execution for task {} in workflow {}",
            task_id, workflow_id
        );

        let service = Self::new(pool.clone());

        // Get task to check its state
        let task = service.task_repo.get_by_id(task_id).await?;

        // Validate task belongs to workflow
        if task.workflow_id != workflow_id {
            return Err(AppError::validation(format!(
                "Task {} does not belong to workflow {}",
                task_id, workflow_id
            )));
        }

        // Check if task is in appropriate state for execution
        // Tasks can be executed if they are PENDING or RUNNING
        let can_execute = task.status == "PENDING" || task.status == "RUNNING";

        if !can_execute {
            return Err(AppError::validation(format!(
                "Task {} is not in a valid state for execution: {}",
                task_id, task.status
            )));
        }

        info!("Task {} validated for execution", task_id);
        Ok(true)
    }

    /// Validate that a task completion meets policy requirements
    ///
    /// # Arguments
    /// * `session_key` - Optional session identifier (for future session validation)
    /// * `workflow_id` - Workflow ID
    /// * `task_id` - Task ID
    ///
    /// # Returns
    /// * `Result<bool>` - true if completion is validated
    pub async fn validate_completion(
        pool: &DbPool,
        _session_key: Option<&str>,
        workflow_id: &str,
        task_id: &str,
    ) -> Result<bool> {
        debug!(
            "Validating completion for task {} in workflow {}",
            task_id, workflow_id
        );

        let service = Self::new(pool.clone());

        // Get task to check its state
        let task = service.task_repo.get_by_id(task_id).await?;

        // Validate task belongs to workflow
        if task.workflow_id != workflow_id {
            return Err(AppError::validation(format!(
                "Task {} does not belong to workflow {}",
                task_id, workflow_id
            )));
        }

        // Check if task requires review
        if task.requires_tdd.unwrap_or(false) {
            // Check if review exists and is approved
            let review = service.review_repo.get_latest(task_id).await?;

            match review {
                Some(review_decision) => {
                    if review_decision.decision != "APPROVED" {
                        return Err(AppError::validation(format!(
                            "Task {} review not approved: {}",
                            task_id, review_decision.decision
                        )));
                    }
                }
                None => {
                    return Err(AppError::validation(format!(
                        "Task {} requires review but none found",
                        task_id
                    )));
                }
            }
        }

        info!("Task {} validated for completion", task_id);
        Ok(true)
    }

    /// Detect scope drift using basic keyword analysis
    ///
    /// # Arguments
    /// * `workflow_id` - Workflow ID
    /// * `task_id` - Task ID
    /// * `output_text` - Output text to analyze for drift
    ///
    /// # Returns
    /// * `Option<String>` - Some(drift_description) if drift detected, None otherwise
    pub async fn detect_scope_drift(
        _workflow_id: &str,
        _task_id: &str,
        output_text: &str,
    ) -> Option<String> {
        debug!("Detecting scope drift in output");

        // Basic keyword-based drift detection
        // In production, this would use semantic analysis

        let drift_keywords = vec![
            "out of scope",
            "unrelated",
            "not in requirements",
            "scope creep",
            "additional feature",
            "extra functionality",
        ];

        let output_lower = output_text.to_lowercase();

        for keyword in drift_keywords {
            if output_lower.contains(keyword) {
                return Some(format!(
                    "Potential scope drift detected: keyword '{}' found in output",
                    keyword
                ));
            }
        }

        // Check for unusually long output (might indicate doing too much)
        let word_count = output_text.split_whitespace().count();
        if word_count > 5000 {
            return Some(format!(
                "Output unusually long ({} words), may indicate scope drift",
                word_count
            ));
        }

        debug!("No scope drift detected");
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate() {
        // Placeholder test
    }

    #[tokio::test]
    async fn test_detect_scope_drift() {
        // Test drift detection
        let output_with_drift = "This implementation includes out of scope features";
        assert!(
            PolicyService::detect_scope_drift("wf1", "task1", output_with_drift)
                .await
                .is_some()
        );

        let output_normal = "This is a normal implementation";
        assert!(
            PolicyService::detect_scope_drift("wf1", "task1", output_normal)
                .await
                .is_none()
        );

        // Test length-based drift
        let long_output = "word ".repeat(6000);
        assert!(
            PolicyService::detect_scope_drift("wf1", "task1", &long_output)
                .await
                .is_some()
        );
    }
}
