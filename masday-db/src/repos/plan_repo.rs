//! Plan repository

use crate::schema::{Plan, NewPlan};
use deadpool_postgres::Pool;
use masday_core::{AppError, Result};

pub struct PlanRepo {
    pool: Pool,
}

impl PlanRepo {
    pub fn new(pool: Pool) -> Self {
        Self { pool }
    }

    /// Create a new plan
    pub async fn create(&self, plan: &NewPlan) -> Result<Plan> {
        let client = self.pool.get().await
            .map_err(|e| AppError::Database(format!("Failed to get connection: {}", e)))?;

        let id = uuid::Uuid::new_v4().to_string();
        let now: chrono::DateTime<chrono::Utc> = chrono::Utc::now();

        let query = "
            INSERT INTO plans (
                id, workflow_id, version, status, summary, content, created_by_agent, created_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            RETURNING *
        ";

        let row = client.query_one(
            query,
            &[
                &id, &plan.workflow_id, &plan.version, &plan.status,
                &plan.summary, &plan.content, &plan.created_by_agent, &now,
            ],
        ).await.map_err(|e| AppError::Database(format!("Failed to create plan: {}", e)))?;

        Ok(Plan {
            id: row.get("id"),
            workflow_id: row.get("workflow_id"),
            version: row.get("version"),
            status: row.get("status"),
            summary: row.get("summary"),
            content: row.get("content"),
            created_by_agent: row.get("created_by_agent"),
            created_at: row.get("created_at"),
        })
    }

    /// Get plan by workflow ID
    pub async fn get_by_workflow(&self, workflow_id: &str) -> Result<Option<Plan>> {
        let client = self.pool.get().await
            .map_err(|e| AppError::Database(format!("Failed to get connection: {}", e)))?;

        let query = "SELECT * FROM plans WHERE workflow_id = $1 ORDER BY version DESC LIMIT 1";
        let rows = client.query(query, &[&workflow_id]).await
            .map_err(|e| AppError::Database(format!("Failed to get plan: {}", e)))?;

        if rows.is_empty() {
            return Ok(None);
        }

        let row = &rows[0];
        Ok(Some(Plan {
            id: row.get("id"),
            workflow_id: row.get("workflow_id"),
            version: row.get("version"),
            status: row.get("status"),
            summary: row.get("summary"),
            content: row.get("content"),
            created_by_agent: row.get("created_by_agent"),
            created_at: row.get("created_at"),
        }))
    }

    /// Update plan status
    pub async fn update_status(&self, id: &str, status: &str) -> Result<Plan> {
        let client = self.pool.get().await
            .map_err(|e| AppError::Database(format!("Failed to get connection: {}", e)))?;

        let query = "UPDATE plans SET status = $1 WHERE id = $2 RETURNING *";
        let row = client.query_one(query, &[&status, &id]).await
            .map_err(|e| AppError::Database(format!("Failed to update plan status: {}", e)))?;

        Ok(Plan {
            id: row.get("id"),
            workflow_id: row.get("workflow_id"),
            version: row.get("version"),
            status: row.get("status"),
            summary: row.get("summary"),
            content: row.get("content"),
            created_by_agent: row.get("created_by_agent"),
            created_at: row.get("created_at"),
        })
    }
}
