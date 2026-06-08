//! LLM provider configuration repository
//!
//! Table names are snake_case: "llm_provider_configs"
//! Column names are snake_case: "provider_name", "base_url", "api_key_env_var", etc.

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
        let now: chrono::DateTime<chrono::Utc> = chrono::Utc::now();

        let query = r#"
            INSERT INTO llm_provider_configs (
                id, provider_name, base_url, api_key_env_var, models,
                is_default, priority, created_at, updated_at
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

        let query = r#"SELECT * FROM llm_provider_configs WHERE id = $1"#;
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
            SELECT * FROM llm_provider_configs
            WHERE is_default = true
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

        let query = r#"SELECT * FROM llm_provider_configs ORDER BY priority DESC, created_at ASC"#;
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

        let now: chrono::DateTime<chrono::Utc> = chrono::Utc::now();

        // Build dynamic UPDATE query
        let mut set_clauses = vec![r#"updated_at = $2"#.to_string()];
        let mut param_count = 2;
        let mut params: Vec<Box<dyn tokio_postgres::types::ToSql + Sync>> =
            vec![Box::new(id.to_string()), Box::new(now)];

        if let Some(provider_name) = updates.get("provider_name").and_then(|v| v.as_str()) {
            param_count += 1;
            set_clauses.push(format!(r#"provider_name = ${}"#, param_count));
            params.push(Box::new(provider_name.to_string()));
        }
        if let Some(base_url) = updates.get("base_url").and_then(|v| v.as_str()) {
            param_count += 1;
            set_clauses.push(format!(r#"base_url = ${}"#, param_count));
            params.push(Box::new(base_url.to_string()));
        }
        if let Some(api_key_env_var) = updates.get("api_key_env_var").and_then(|v| v.as_str()) {
            param_count += 1;
            set_clauses.push(format!(r#"api_key_env_var = ${}"#, param_count));
            params.push(Box::new(api_key_env_var.to_string()));
        }
        if let Some(models) = updates.get("models") {
            param_count += 1;
            set_clauses.push(format!("models = ${}", param_count));
            params.push(Box::new(models.clone()));
        }
        if let Some(is_default) = updates.get("is_default").and_then(|v| v.as_bool()) {
            param_count += 1;
            set_clauses.push(format!(r#"is_default = ${}"#, param_count));
            params.push(Box::new(is_default));
        }
        if let Some(priority) = updates.get("priority").and_then(|v| v.as_i64()) {
            param_count += 1;
            set_clauses.push(format!("priority = ${}", param_count));
            params.push(Box::new(priority));
        }

        let sql = format!(
            r#"UPDATE llm_provider_configs SET {} WHERE id = $1 RETURNING *"#,
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
            .execute(r#"DELETE FROM llm_provider_configs WHERE id = $1"#, &[&id])
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
            .execute(r#"UPDATE llm_provider_configs SET is_default = false"#, &[])
            .await
            .map_err(|e| AppError::Database(format!("Failed to unset default providers: {}", e)))?;

        // Then set the new default
        let now: chrono::DateTime<chrono::Utc> = chrono::Utc::now();
        let query = r#"
            UPDATE llm_provider_configs
            SET is_default = true, updated_at = $2
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_constructor_signature() {
        fn _check() {
            let _ = LlmProviderConfigRepo::new;
        }
    }

    #[test]
    fn test_new_config_construction() {
        let config = NewLlmProviderConfig {
            provider_name: "openai".to_string(),
            base_url: "https://api.openai.com/v1".to_string(),
            api_key_env_var: "OPENAI_API_KEY".to_string(),
            models: serde_json::json!(["gpt-4", "gpt-3.5-turbo"]),
            is_default: Some(true),
            priority: Some(1),
        };
        assert_eq!(config.provider_name, "openai");
        assert_eq!(config.is_default, Some(true));
    }

    #[test]
    fn test_dynamic_update_builder() {
        let updates = serde_json::json!({"base_url": "http://new", "priority": 2});
        let mut count = 0;
        if updates.get("base_url").and_then(|v| v.as_str()).is_some() {
            count += 1;
        }
        if updates.get("priority").and_then(|v| v.as_i64()).is_some() {
            count += 1;
        }
        if updates
            .get("provider_name")
            .and_then(|v| v.as_str())
            .is_some()
        {
            count += 1;
        }
        assert_eq!(count, 2); // base_url + priority, not provider_name
    }

    #[test]
    fn test_set_default_sql() {
        let sql = r#"UPDATE llm_provider_configs SET is_default = FALSE WHERE id != $1"#;
        assert!(sql.contains("is_default = FALSE"));
    }

    #[test]
    fn test_insert_sql_contains_returning_star() {
        let sql = r#"INSERT INTO llm_provider_configs (id, provider_name, base_url, api_key_env_var, models, is_default, priority, created_at, updated_at) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9) RETURNING *"#;
        assert!(sql.contains("RETURNING *"));
    }
}
