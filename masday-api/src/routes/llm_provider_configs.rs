//! LLM provider configuration routes — CRUD for LlmProviderConfig

use axum::routing::{delete, get, post, put};
use axum::{
    extract::{Path, State},
    Json, Router,
};
use serde_json::Value;

use crate::middleware::error_handler::ApiError;
use crate::AppState;
use masday_db::repos::LlmProviderConfigRepo;

pub fn llm_provider_config_routes() -> Router<AppState> {
    Router::new()
        .route(
            "/llm-provider-configs",
            post(create_llm_provider_config).get(list_llm_provider_configs),
        )
        .route("/llm-provider-configs/default", get(get_default_config))
        .route("/llm-provider-configs/{id}", get(get_llm_provider_config))
        .route(
            "/llm-provider-configs/{id}/update",
            post(update_llm_provider_config),
        )
        .route(
            "/llm-provider-configs/{id}",
            delete(delete_llm_provider_config),
        )
        .route(
            "/llm-provider-configs/{id}/set-default",
            put(set_default_config),
        )
}

/// POST /llm-provider-configs — Create a new LLM provider configuration
pub async fn create_llm_provider_config(
    State(state): State<AppState>,
    Json(payload): Json<Value>,
) -> Result<Json<Value>, ApiError> {
    let repo = LlmProviderConfigRepo::new(state.pool.clone());
    let new_config = masday_db::schema::NewLlmProviderConfig {
        provider_name: payload
            .get("provider_name")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        base_url: payload
            .get("base_url")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        api_key_env_var: payload
            .get("api_key_env_var")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        models: payload
            .get("models")
            .cloned()
            .unwrap_or(serde_json::json!([])),
        is_default: payload.get("is_default").and_then(|v| v.as_bool()),
        priority: payload
            .get("priority")
            .and_then(|v| v.as_i64())
            .map(|v| v as i32),
    };
    let config = repo.create(&new_config).await?;
    Ok(Json(serde_json::json!(config)))
}

/// GET /llm-provider-configs — List all LLM provider configurations
pub async fn list_llm_provider_configs(
    State(state): State<AppState>,
) -> Result<Json<Value>, ApiError> {
    let repo = LlmProviderConfigRepo::new(state.pool.clone());
    let configs = repo.list_all().await?;
    Ok(Json(serde_json::json!(configs)))
}

/// GET /llm-provider-configs/{id} — Get an LLM provider configuration by ID
pub async fn get_llm_provider_config(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Value>, ApiError> {
    let repo = LlmProviderConfigRepo::new(state.pool.clone());
    let config = repo.get_by_id(&id).await?;
    Ok(Json(serde_json::json!(config)))
}

/// GET /llm-provider-configs/default — Get the default LLM provider configuration
pub async fn get_default_config(State(state): State<AppState>) -> Result<Json<Value>, ApiError> {
    let repo = LlmProviderConfigRepo::new(state.pool.clone());
    if let Some(config) = repo.get_default().await? {
        Ok(Json(serde_json::json!(config)))
    } else {
        Ok(Json(serde_json::json!(null)))
    }
}

/// POST /llm-provider-configs/{id}/update — Update an LLM provider configuration
pub async fn update_llm_provider_config(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(_payload): Json<Value>,
) -> Result<Json<Value>, ApiError> {
    // For now, just return the existing config (not a true update)
    let repo = LlmProviderConfigRepo::new(state.pool.clone());
    let config = repo.get_by_id(&id).await?;
    Ok(Json(serde_json::json!(config)))
}

/// PUT /llm-provider-configs/{id}/set-default — Set a provider as the default
pub async fn set_default_config(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Value>, ApiError> {
    let repo = LlmProviderConfigRepo::new(state.pool.clone());
    let config = repo.set_default(&id).await?;
    Ok(Json(serde_json::json!(config)))
}

/// DELETE /llm-provider-configs/{id} — Delete an LLM provider configuration
pub async fn delete_llm_provider_config(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Value>, ApiError> {
    let repo = LlmProviderConfigRepo::new(state.pool.clone());
    let deleted = repo.delete(&id).await?;
    Ok(Json(serde_json::json!({"deleted": deleted, "id": id})))
}
