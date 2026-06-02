//! Token usage repository
//!
//! Table names are PascalCase (created by Drizzle/TypeScript): "TokenUsage"
//! Column names are camelCase: "promptTokens", "completionTokens", "totalTokens", etc.

use crate::pool::DbPool;
use crate::schema::{NewTokenUsage, TokenUsage};
use masday_core::{AppError, Result};

pub struct TokenUsageRepo {
    pool: DbPool,
}

impl TokenUsageRepo {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    /// Create a new token usage entry
    pub async fn create(&self, usage: &NewTokenUsage) -> Result<TokenUsage> {
        let client = self
            .pool
            .get()
            .await
            .map_err(|e| AppError::Database(format!("Failed to get connection: {}", e)))?;

        let id = uuid::Uuid::new_v4().to_string();
        let now: chrono::NaiveDateTime = chrono::Utc::now().naive_utc();

        let query = r#"
            INSERT INTO "TokenUsage" (
                id, source, route, model, "promptTokens", "completionTokens",
                "totalTokens", "latencyMs", metadata, "createdAt"
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            RETURNING *
        "#;

        let row = client
            .query_one(
                query,
                &[
                    &id,
                    &usage.source,
                    &usage.route,
                    &usage.model,
                    &usage.prompt_tokens,
                    &usage.completion_tokens,
                    &usage.total_tokens,
                    &usage.latency_ms,
                    &usage.metadata,
                    &now,
                ],
            )
            .await
            .map_err(|e| AppError::Database(format!("Failed to create token usage: {}", e)))?;

        Ok(TokenUsage::from_row(&row))
    }

    /// Get token usage by ID
    pub async fn get_by_id(&self, id: &str) -> Result<TokenUsage> {
        let client = self
            .pool
            .get()
            .await
            .map_err(|e| AppError::Database(format!("Failed to get connection: {}", e)))?;

        let query = r#"SELECT * FROM "TokenUsage" WHERE id = $1"#;
        let row = client
            .query_one(query, &[&id])
            .await
            .map_err(|_e| AppError::not_found("TokenUsage", id))?;

        Ok(TokenUsage::from_row(&row))
    }

    /// List all token usage entries for a source
    pub async fn list_by_source(
        &self,
        source: &str,
        limit: Option<i64>,
    ) -> Result<Vec<TokenUsage>> {
        let client = self
            .pool
            .get()
            .await
            .map_err(|e| AppError::Database(format!("Failed to get connection: {}", e)))?;

        let capped = limit.unwrap_or(100).min(1000);
        let query = r#"
            SELECT * FROM "TokenUsage"
            WHERE source = $1
            ORDER BY "createdAt" DESC
            LIMIT $2
        "#;
        let rows = client
            .query(query, &[&source, &capped])
            .await
            .map_err(|e| {
                AppError::Database(format!("Failed to list token usage by source: {}", e))
            })?;

        Ok(rows.iter().map(TokenUsage::from_row).collect())
    }

    /// List all token usage entries (with optional limit)
    pub async fn list_all(&self, limit: Option<i64>) -> Result<Vec<TokenUsage>> {
        let client = self
            .pool
            .get()
            .await
            .map_err(|e| AppError::Database(format!("Failed to get connection: {}", e)))?;

        let capped = limit.unwrap_or(100).min(1000);
        let query = r#"SELECT * FROM "TokenUsage" ORDER BY "createdAt" DESC LIMIT $1"#;
        let rows = client
            .query(query, &[&capped])
            .await
            .map_err(|e| AppError::Database(format!("Failed to list all token usage: {}", e)))?;

        Ok(rows.iter().map(TokenUsage::from_row).collect())
    }

    /// Delete a token usage entry
    pub async fn delete(&self, id: &str) -> Result<bool> {
        let client = self
            .pool
            .get()
            .await
            .map_err(|e| AppError::Database(format!("Failed to get connection: {}", e)))?;

        let result = client
            .execute(r#"DELETE FROM "TokenUsage" WHERE id = $1"#, &[&id])
            .await
            .map_err(|e| AppError::Database(format!("Failed to delete token usage: {}", e)))?;

        Ok(result > 0)
    }

    /// Get total token usage statistics by source
    pub async fn get_stats_by_source(&self, source: &str) -> Result<serde_json::Value> {
        let client = self
            .pool
            .get()
            .await
            .map_err(|e| AppError::Database(format!("Failed to get connection: {}", e)))?;

        let query = r#"
            SELECT
                COUNT(*) as count,
                COALESCE(SUM("totalTokens"), 0) as total_tokens,
                COALESCE(SUM("promptTokens"), 0) as prompt_tokens,
                COALESCE(SUM("completionTokens"), 0) as completion_tokens,
                COALESCE(AVG("latencyMs"), 0) as avg_latency_ms
            FROM "TokenUsage"
            WHERE source = $1
        "#;
        let row = client
            .query_one(query, &[&source])
            .await
            .map_err(|e| AppError::Database(format!("Failed to get token usage stats: {}", e)))?;

        Ok(serde_json::json!({
            "source": source,
            "count": row.get::<_, i64>("count"),
            "total_tokens": row.get::<_, i64>("total_tokens"),
            "prompt_tokens": row.get::<_, i64>("prompt_tokens"),
            "completion_tokens": row.get::<_, i64>("completion_tokens"),
            "avg_latency_ms": row.get::<_, f64>("avg_latency_ms"),
        }))
    }
}
