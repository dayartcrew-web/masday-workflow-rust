//! Context routes — wired to ContextService and SearchService via ApiError

use axum::routing::{get, post};
use axum::{
    extract::{Path, Query, State},
    Json, Router,
};
use serde::Deserialize;
use serde_json::Value;

use crate::middleware::error_handler::ApiError;
use crate::AppState;

pub fn context_routes() -> Router<AppState> {
    Router::new()
        .route("/context/pack/{workflow_id}", get(build_context_pack))
        .route("/context/fingerprint", post(compute_fingerprint))
        .route(
            "/context/pack/{workflow_id}/{plan_id}/{task_id}",
            get(build_context_pack_full),
        )
        // Semantic search routes
        .route("/context/search", get(code_search))
        .route("/context/hybrid-search", post(hybrid_search))
        .route("/context/fingerprint-search", post(fingerprint_search))
}

async fn build_context_pack(
    State(state): State<AppState>,
    Path(workflow_id): Path<String>,
) -> Result<Json<Value>, ApiError> {
    let pack = masday_service::ContextService::build_context_pack(
        &state.pool,
        &workflow_id,
        &workflow_id,
        &workflow_id,
    )
    .await?;
    Ok(Json(pack))
}

async fn build_context_pack_full(
    State(state): State<AppState>,
    Path((workflow_id, plan_id, task_id)): Path<(String, String, String)>,
) -> Result<Json<Value>, ApiError> {
    let pack = masday_service::ContextService::build_context_pack(
        &state.pool,
        &workflow_id,
        &plan_id,
        &task_id,
    )
    .await?;
    Ok(Json(pack))
}

#[derive(Deserialize)]
struct FingerprintInput {
    workflow_id: String,
    plan_id: String,
    task_id: String,
}

async fn compute_fingerprint(Json(input): Json<FingerprintInput>) -> Json<Value> {
    let fp = masday_service::ContextService::compute_fingerprint(
        &input.workflow_id,
        &input.plan_id,
        &input.task_id,
    );
    Json(serde_json::json!({"fingerprint": fp}))
}

#[derive(Deserialize)]
struct SearchQuery {
    query: String,
}

/// Code search — uses filesystem + BM25 search
async fn code_search(
    State(_state): State<AppState>,
    Query(params): Query<SearchQuery>,
) -> Result<Json<Value>, ApiError> {
    let results = masday_service::SearchService::code_search(&params.query, ".")
        .await
        .map_err(|e| masday_core::AppError::internal(e.to_string()))?;
    let count = results
        .get("results")
        .and_then(|v| v.as_array())
        .map(|a| a.len())
        .unwrap_or(0);

    Ok(Json(serde_json::json!({
        "query": params.query,
        "results": results.get("results").cloned().unwrap_or(serde_json::json!([])),
        "count": count
    })))
}

#[derive(Deserialize)]
struct HybridSearchInput {
    workflow_id: String,
    plan_id: String,
    task_id: String,
}

/// Hybrid context pack — combines semantic search + fingerprinting
async fn hybrid_search(
    State(state): State<AppState>,
    Json(input): Json<HybridSearchInput>,
) -> Result<Json<Value>, ApiError> {
    let pack = masday_service::ContextService::build_context_pack(
        &state.pool,
        &input.workflow_id,
        &input.plan_id,
        &input.task_id,
    )
    .await?;

    // Compute fingerprint for change detection
    let fingerprint = masday_service::ContextService::compute_fingerprint(
        &input.workflow_id,
        &input.plan_id,
        &input.task_id,
    );

    Ok(Json(serde_json::json!({
        "contextPack": pack,
        "fingerprint": fingerprint,
        "workflowId": input.workflow_id,
        "planId": input.plan_id,
        "taskId": input.task_id
    })))
}

#[derive(Deserialize)]
struct FingerprintSearchInput {
    workflow_id: String,
    plan_id: String,
    task_id: String,
}

/// Fingerprint search — compute and compare context fingerprints
async fn fingerprint_search(
    Json(input): Json<FingerprintSearchInput>,
) -> Result<Json<Value>, ApiError> {
    let fingerprint = masday_service::ContextService::compute_fingerprint(
        &input.workflow_id,
        &input.plan_id,
        &input.task_id,
    );

    Ok(Json(serde_json::json!({
        "fingerprint": fingerprint,
        "workflowId": input.workflow_id,
        "planId": input.plan_id,
        "taskId": input.task_id
    })))
}
