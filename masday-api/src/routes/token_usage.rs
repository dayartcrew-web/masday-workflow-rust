//! Token usage routes — CRUD for TokenUsage

use axum::routing::{get, post};
use axum::{
    extract::{Path, Query, State},
    Json, Router,
};
use serde::Deserialize;
use serde_json::Value;

use crate::extractors::pagination::Pagination;
use crate::middleware::error_handler::ApiError;
use crate::AppState;
use masday_db::repos::TokenUsageRepo;

#[derive(Deserialize)]
struct ListTokenUsageQuery {
    #[serde(default)]
    source: Option<String>,
}

pub fn token_usage_routes() -> Router<AppState> {
    Router::new()
        .route(
            "/token-usage",
            post(create_token_usage).get(list_token_usage),
        )
        .route(
            "/token-usage/{id}",
            get(get_token_usage).delete(delete_token_usage),
        )
        .route("/token-usage/source/{source}", get(list_by_source))
        .route("/token-usage/stats/{source}", get(get_stats))
}

/// POST /token-usage — Create a new token usage entry
async fn create_token_usage(
    State(state): State<AppState>,
    Json(payload): Json<Value>,
) -> Result<Json<Value>, ApiError> {
    let repo = TokenUsageRepo::new(state.pool.clone());
    let new_usage = masday_db::schema::NewTokenUsage {
        source: payload
            .get("source")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string(),
        route: payload
            .get("route")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        model: payload
            .get("model")
            .and_then(|v| v.as_str())
            .map(String::from),
        prompt_tokens: payload
            .get("prompt_tokens")
            .and_then(|v| v.as_i64())
            .map(|v| v as i32),
        completion_tokens: payload
            .get("completion_tokens")
            .and_then(|v| v.as_i64())
            .map(|v| v as i32),
        total_tokens: payload
            .get("total_tokens")
            .and_then(|v| v.as_i64())
            .map(|v| v as i32),
        latency_ms: payload
            .get("latency_ms")
            .and_then(|v| v.as_i64())
            .map(|v| v as i32),
        metadata: payload.get("metadata").cloned(),
    };
    let usage = repo.create(&new_usage).await?;
    Ok(Json(serde_json::json!(usage)))
}

/// GET /token-usage — List token usage entries (with optional source filter)
async fn list_token_usage(
    State(state): State<AppState>,
    Query(params): Query<ListTokenUsageQuery>,
    pagination: Pagination,
) -> Result<Json<Value>, ApiError> {
    let repo = TokenUsageRepo::new(state.pool.clone());
    let usage = if let Some(source) = &params.source {
        repo.list_by_source(source, Some(pagination.limit() as i64))
            .await?
    } else {
        repo.list_all(Some(pagination.limit() as i64)).await?
    };
    Ok(Json(serde_json::json!(usage)))
}

/// GET /token-usage/{id} — Get a token usage entry by ID
async fn get_token_usage(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Value>, ApiError> {
    let repo = TokenUsageRepo::new(state.pool.clone());
    let usage = repo.get_by_id(&id).await?;
    Ok(Json(serde_json::json!(usage)))
}

/// GET /token-usage/source/{source} — List token usage entries for a source
async fn list_by_source(
    State(state): State<AppState>,
    Path(source): Path<String>,
    pagination: Pagination,
) -> Result<Json<Value>, ApiError> {
    let repo = TokenUsageRepo::new(state.pool.clone());
    let usage = repo
        .list_by_source(&source, Some(pagination.limit() as i64))
        .await?;
    Ok(Json(serde_json::json!(usage)))
}

/// GET /token-usage/stats/{source} — Get token usage statistics for a source
async fn get_stats(
    State(state): State<AppState>,
    Path(source): Path<String>,
) -> Result<Json<Value>, ApiError> {
    let repo = TokenUsageRepo::new(state.pool.clone());
    let stats = repo.get_stats_by_source(&source).await?;
    Ok(Json(stats))
}

/// DELETE /token-usage/{id} — Delete a token usage entry
async fn delete_token_usage(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Value>, ApiError> {
    let repo = TokenUsageRepo::new(state.pool.clone());
    let deleted = repo.delete(&id).await?;
    Ok(Json(serde_json::json!({"deleted": deleted, "id": id})))
}
