//! Policy routes — wired to PolicyService via ApiError

use axum::routing::post;
use axum::{
    extract::{Path, State},
    Json, Router,
};
use serde_json::Value;

use crate::middleware::error_handler::ApiError;
use crate::AppState;

pub fn policy_routes() -> Router<AppState> {
    Router::new()
        .route("/policy/validate-execution", post(validate_execution))
        .route("/policy/validate-completion", post(validate_completion))
        .route("/policy/drift/:workflow_id", post(detect_drift))
        .route("/policy/session-readiness", post(check_session_readiness))
}

async fn validate_execution(
    State(state): State<AppState>,
    Json(payload): Json<Value>,
) -> Result<Json<Value>, ApiError> {
    let session_key = payload.get("session_key").and_then(|v| v.as_str());
    let workflow_id = payload
        .get("workflow_id")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let task_id = payload
        .get("task_id")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    let valid = masday_service::PolicyService::validate_execution(
        &state.pool,
        session_key,
        workflow_id,
        task_id,
    )
    .await?;
    Ok(Json(serde_json::json!({"valid": valid})))
}

async fn validate_completion(
    State(state): State<AppState>,
    Json(payload): Json<Value>,
) -> Result<Json<Value>, ApiError> {
    let session_key = payload.get("session_key").and_then(|v| v.as_str());
    let workflow_id = payload
        .get("workflow_id")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let task_id = payload
        .get("task_id")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    let valid = masday_service::PolicyService::validate_completion(
        &state.pool,
        session_key,
        workflow_id,
        task_id,
    )
    .await?;
    Ok(Json(serde_json::json!({"valid": valid})))
}

async fn detect_drift(Path(workflow_id): Path<String>, Json(payload): Json<Value>) -> Json<Value> {
    let task_id = payload
        .get("task_id")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let output = payload
        .get("output_text")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let drift =
        masday_service::PolicyService::detect_scope_drift(&workflow_id, task_id, output).await;
    Json(serde_json::json!({"workflow_id": workflow_id, "drift": drift}))
}

async fn check_session_readiness(Json(_payload): Json<Value>) -> Json<Value> {
    Json(serde_json::json!({"ready": true}))
}
