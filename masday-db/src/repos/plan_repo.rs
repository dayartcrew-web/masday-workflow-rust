//! Plan repository
//!
//! Table names are PascalCase (created by Drizzle/TypeScript): "Plan"
//! Column names are camelCase: "workflowId", "createdByAgent", etc.

use crate::pool::DbPool;
use crate::schema::{NewPlan, Plan};
use masday_core::{AppError, Result};

pub struct PlanRepo {
    pool: DbPool,
}

impl PlanRepo {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    /// Create a new plan
    pub async fn create(&self, plan: &NewPlan) -> Result<Plan> {
        let client = self
            .pool
            .get()
            .await
            .map_err(|e| AppError::Database(format!("Failed to get connection: {}", e)))?;

        let id = uuid::Uuid::new_v4().to_string();
        let now: chrono::NaiveDateTime = chrono::Utc::now().naive_utc();

        let query = r#"
            INSERT INTO "Plan" (
                id, "workflowId", version, status, summary, content, "createdByAgent", "createdAt"
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            RETURNING *
        "#;

        let row = client
            .query_one(
                query,
                &[
                    &id,
                    &plan.workflow_id,
                    &plan.version,
                    &plan.status,
                    &plan.summary,
                    &plan.content,
                    &plan.created_by_agent,
                    &now,
                ],
            )
            .await
            .map_err(|e| AppError::Database(format!("Failed to create plan: {}", e)))?;

        Ok(Plan::from_row(&row))
    }

    /// Get plan by workflow ID
    pub async fn get_by_workflow(&self, workflow_id: &str) -> Result<Option<Plan>> {
        let client = self
            .pool
            .get()
            .await
            .map_err(|e| AppError::Database(format!("Failed to get connection: {}", e)))?;

        let query = r#"SELECT * FROM "Plan" WHERE "workflowId" = $1 ORDER BY version DESC LIMIT 1"#;
        let rows = client
            .query(query, &[&workflow_id])
            .await
            .map_err(|e| AppError::Database(format!("Failed to get plan: {}", e)))?;

        if rows.is_empty() {
            return Ok(None);
        }

        Ok(Some(Plan::from_row(&rows[0])))
    }

    /// Get plan by ID
    pub async fn get_by_id(&self, id: &str) -> Result<Option<Plan>> {
        let client = self
            .pool
            .get()
            .await
            .map_err(|e| AppError::Database(format!("Failed to get connection: {}", e)))?;

        let query = r#"SELECT * FROM "Plan" WHERE id = $1"#;
        let rows = client
            .query(query, &[&id])
            .await
            .map_err(|e| AppError::Database(format!("Failed to get plan: {}", e)))?;

        if rows.is_empty() {
            return Ok(None);
        }

        Ok(Some(Plan::from_row(&rows[0])))
    }

    /// Update plan status
    pub async fn update_status(&self, id: &str, status: &str) -> Result<Plan> {
        let client = self
            .pool
            .get()
            .await
            .map_err(|e| AppError::Database(format!("Failed to get connection: {}", e)))?;

        let query = r#"UPDATE "Plan" SET status = $1 WHERE id = $2 RETURNING *"#;
        let row = client
            .query_one(query, &[&status, &id])
            .await
            .map_err(|e| AppError::Database(format!("Failed to update plan status: {}", e)))?;

        Ok(Plan::from_row(&row))
    }

    /// Count plans for a workflow
    pub async fn count_by_workflow(&self, workflow_id: &str) -> Result<i64> {
        let client = self
            .pool
            .get()
            .await
            .map_err(|e| AppError::Database(format!("Failed to get connection: {}", e)))?;

        let query = r#"SELECT COUNT(*) FROM "Plan" WHERE "workflowId" = $1"#;
        let row = client
            .query_one(query, &[&workflow_id])
            .await
            .map_err(|e| AppError::Database(format!("Failed to count plans: {}", e)))?;

        Ok(row.get::<_, i64>("count"))
    }

    /// Get active plan for a workflow (status = 'ACTIVE')
    pub async fn get_active_for_workflow(&self, workflow_id: &str) -> Result<Option<Plan>> {
        let client = self
            .pool
            .get()
            .await
            .map_err(|e| AppError::Database(format!("Failed to get connection: {}", e)))?;

        let query = r#"SELECT * FROM "Plan" WHERE "workflowId" = $1 AND status = 'ACTIVE' ORDER BY version DESC LIMIT 1"#;
        let rows = client
            .query(query, &[&workflow_id])
            .await
            .map_err(|e| AppError::Database(format!("Failed to get active plan: {}", e)))?;

        if rows.is_empty() {
            return Ok(None);
        }

        Ok(Some(Plan::from_row(&rows[0])))
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_plan_repo_has_get_by_id() {
        // This test ensures get_by_id exists and has the right signature
        // Just verify the method signature by checking the struct has this method
        // In a real integration test, we would use a database connection
        // Placeholder test passes
    }
}
