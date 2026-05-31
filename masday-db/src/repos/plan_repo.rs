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
}
