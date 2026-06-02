//! LLM provider configuration repository
//!
//! Table names are PascalCase (created by Drizzle/TypeScript): "LlmProviderConfig"
//! Column names are camelCase: "providerName", "baseUrl", "apiKeyEnvVar", etc.

use crate::pool::DbPool;
use crate::schema::{LlmProviderConfig, NewLlmProviderConfig};
use masday_core::{AppError, Result};

pub struct LlmProviderConfigRepo {
    pool: DbPool,
}

impl LlmProviderConfigRepo {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    /// Create a new LLM provider configuration
    pub async fn create(&self, config: &NewLlmProviderConfig) -> Result<LlmProviderConfig> {
        let client = self
            .pool
            .get()
            .await
            .map_err(|e| AppError::Database(format!("Failed to get connection: {}", e)))?;

        let id = uuid::Uuid::new_v4().to_string();
        let now: chrono::NaiveDateTime = chrono::Utc::now().naive_utc();

        let query = r#"
            INSERT INTO "LlmProviderConfig" (
                id, "providerName", "baseUrl", "apiKeyEnvVar", models,
                "isDefault", priority, "createdAt", "updatedAt"
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            RETURNING *
        "#;

        let row = client
            .query_one(
                query,
                &[
                    &id,
                    &config.provider_name,
                    &config.base_url,
                    &config.api_key_env_var,
                    &config.models,
                    &config.is_default,
                    &config.priority,
                    &now,
                    &now,
                ],
            )
            .await
            .map_err(|e| {
                AppError::Database(format!("Failed to create LLM provider config: {}", e))
            })?;

        Ok(LlmProviderConfig::from_row(&row))
    }

    /// Get LLM provider configuration by ID
    pub async fn get_by_id(&self, id: &str) -> Result<LlmProviderConfig> {
        let client = self
            .pool
            .get()
            .await
            .map_err(|e| AppError::Database(format!("Failed to get connection: {}", e)))?;

        let query = r#"SELECT * FROM "LlmProviderConfig" WHERE id = $1"#;
        let row = client
            .query_one(query, &[&id])
            .await
            .map_err(|_e| AppError::not_found("LlmProviderConfig", id))?;

        Ok(LlmProviderConfig::from_row(&row))
    }

    /// Get default LLM provider configuration
    pub async fn get_default(&self) -> Result<Option<LlmProviderConfig>> {
        let client = self
            .pool
            .get()
            .await
            .map_err(|e| AppError::Database(format!("Failed to get connection: {}", e)))?;

        let query = r#"
            SELECT * FROM "LlmProviderConfig"
            WHERE "isDefault" = true
            ORDER BY priority DESC
            LIMIT 1
        "#;
        let rows = client.query(query, &[]).await.map_err(|e| {
            AppError::Database(format!("Failed to get default LLM provider config: {}", e))
        })?;

        if rows.is_empty() {
            return Ok(None);
        }

        Ok(Some(LlmProviderConfig::from_row(&rows[0])))
    }

    /// List all LLM provider configurations
    pub async fn list_all(&self) -> Result<Vec<LlmProviderConfig>> {
        let client = self
            .pool
            .get()
            .await
            .map_err(|e| AppError::Database(format!("Failed to get connection: {}", e)))?;

        let query = r#"SELECT * FROM "LlmProviderConfig" ORDER BY priority DESC, "createdAt" ASC"#;
        let rows = client.query(query, &[]).await.map_err(|e| {
            AppError::Database(format!("Failed to list LLM provider configs: {}", e))
        })?;

        Ok(rows.iter().map(LlmProviderConfig::from_row).collect())
    }

    /// Update an LLM provider configuration
    pub async fn update(&self, id: &str, updates: serde_json::Value) -> Result<LlmProviderConfig> {
        let client = self
            .pool
            .get()
            .await
            .map_err(|e| AppError::Database(format!("Failed to get connection: {}", e)))?;

        let now: chrono::NaiveDateTime = chrono::Utc::now().naive_utc();

        // Build dynamic UPDATE query
        let mut set_clauses = vec![r#""updatedAt" = $2"#.to_string()];
        let mut param_count = 2;
        let mut params: Vec<Box<dyn tokio_postgres::types::ToSql + Sync>> =
            vec![Box::new(id.to_string()), Box::new(now)];

        if let Some(provider_name) = updates.get("provider_name").and_then(|v| v.as_str()) {
            param_count += 1;
            set_clauses.push(format!(r#""providerName" = ${}"#, param_count));
            params.push(Box::new(provider_name.to_string()));
        }
        if let Some(base_url) = updates.get("base_url").and_then(|v| v.as_str()) {
            param_count += 1;
            set_clauses.push(format!(r#""baseUrl" = ${}"#, param_count));
            params.push(Box::new(base_url.to_string()));
        }
        if let Some(api_key_env_var) = updates.get("api_key_env_var").and_then(|v| v.as_str()) {
            param_count += 1;
            set_clauses.push(format!(r#""apiKeyEnvVar" = ${}"#, param_count));
            params.push(Box::new(api_key_env_var.to_string()));
        }
        if let Some(models) = updates.get("models") {
            param_count += 1;
            set_clauses.push(format!("models = ${}", param_count));
            params.push(Box::new(models.clone()));
        }
        if let Some(is_default) = updates.get("is_default").and_then(|v| v.as_bool()) {
            param_count += 1;
            set_clauses.push(format!(r#""isDefault" = ${}"#, param_count));
            params.push(Box::new(is_default));
        }
        if let Some(priority) = updates.get("priority").and_then(|v| v.as_i64()) {
            param_count += 1;
            set_clauses.push(format!("priority = ${}", param_count));
            params.push(Box::new(priority));
        }

        let sql = format!(
            r#"UPDATE "LlmProviderConfig" SET {} WHERE id = $1 RETURNING *"#,
            set_clauses.join(", ")
        );

        let params_refs: Vec<&(dyn tokio_postgres::types::ToSql + Sync)> =
            params.iter().map(|p| p.as_ref()).collect();

        let row = client
            .query_one(&sql, params_refs.as_slice())
            .await
            .map_err(|e| {
                AppError::Database(format!("Failed to update LLM provider config: {}", e))
            })?;

        Ok(LlmProviderConfig::from_row(&row))
    }

    /// Delete an LLM provider configuration
    pub async fn delete(&self, id: &str) -> Result<bool> {
        let client = self
            .pool
            .get()
            .await
            .map_err(|e| AppError::Database(format!("Failed to get connection: {}", e)))?;

        let result = client
            .execute(r#"DELETE FROM "LlmProviderConfig" WHERE id = $1"#, &[&id])
            .await
            .map_err(|e| {
                AppError::Database(format!("Failed to delete LLM provider config: {}", e))
            })?;

        Ok(result > 0)
    }

    /// Set a provider as the default (unsets others)
    pub async fn set_default(&self, id: &str) -> Result<LlmProviderConfig> {
        let client = self
            .pool
            .get()
            .await
            .map_err(|e| AppError::Database(format!("Failed to get connection: {}", e)))?;

        // First, unset all defaults
        client
            .execute(r#"UPDATE "LlmProviderConfig" SET "isDefault" = false"#, &[])
            .await
            .map_err(|e| AppError::Database(format!("Failed to unset default providers: {}", e)))?;

        // Then set the new default
        let now: chrono::NaiveDateTime = chrono::Utc::now().naive_utc();
        let query = r#"
            UPDATE "LlmProviderConfig"
            SET "isDefault" = true, "updatedAt" = $2
            WHERE id = $1
            RETURNING *
        "#;
        let row = client
            .query_one(query, &[&id, &now])
            .await
            .map_err(|e| AppError::Database(format!("Failed to set default provider: {}", e)))?;

        Ok(LlmProviderConfig::from_row(&row))
    }
}
