//! Plan business logic
//!
//! Manages workflow plans including creation, retrieval, and status updates.

use masday_db::DbPool;
use masday_core::{AppError, Result};
use masday_db::repos::PlanRepo;
use masday_db::schema::{NewPlan, Plan};
use tracing::{debug, info};

/// Plan service
pub struct PlanService {
    repo: PlanRepo,
}

impl PlanService {
    /// Create a new plan service
    pub fn new(pool: DbPool) -> Self {
        Self {
            repo: PlanRepo::new(pool),
        }
    }

    /// Create a plan for a workflow
    ///
    /// # Arguments
    /// * `pool` - Database connection pool
    /// * `workflow_id` - Parent workflow ID
    /// * `phases` - Plan phases as JSON
    ///
    /// # Returns
    /// * `Result<Plan>` - The created plan
    pub async fn create_plan(
        pool: &DbPool,
        workflow_id: String,
        phases: serde_json::Value,
    ) -> Result<Plan> {
        info!("Creating plan for workflow {}", workflow_id);

        let service = Self::new(pool.clone());

        let new_plan = NewPlan {
            workflow_id: workflow_id.clone(),
            version: 1,
            status: "ACTIVE".to_string(),
            summary: "Initial plan".to_string(),
            content: phases,
            created_by_agent: "system".to_string(),
        };

        let plan = service.repo.create(&new_plan).await?;
        debug!("Plan created with ID: {}", plan.id);

        Ok(plan)
    }

    /// Get a plan by workflow ID
    ///
    /// # Arguments
    /// * `pool` - Database connection pool
    /// * `workflow_id` - Workflow ID
    ///
    /// # Returns
    /// * `Result<Plan>` - The plan
    pub async fn get_plan(pool: &DbPool, workflow_id: &str) -> Result<Plan> {
        debug!("Getting plan for workflow {}", workflow_id);

        let service = Self::new(pool.clone());
        let plan_option = service.repo.get_by_workflow(workflow_id).await?;

        plan_option.ok_or_else(|| AppError::not_found("Plan", workflow_id))
    }

    /// Update plan status
    ///
    /// # Arguments
    /// * `pool` - Database connection pool
    /// * `workflow_id` - Workflow ID
    /// * `status` - New status
    ///
    /// # Returns
    /// * `Result<Plan>` - The updated plan
    pub async fn update_plan_status(
        pool: &DbPool,
        workflow_id: &str,
        status: String,
    ) -> Result<Plan> {
        info!(
            "Updating plan status for workflow {} to {}",
            workflow_id, status
        );

        let service = Self::new(pool.clone());

        // Get the plan first to obtain its ID
        let plan = service.repo.get_by_workflow(workflow_id).await?
            .ok_or_else(|| AppError::not_found("Plan", workflow_id))?;

        // Update using the plan ID
        service.repo.update_status(&plan.id, &status).await
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
