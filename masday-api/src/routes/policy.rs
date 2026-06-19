//! Policy routes — wired to PolicyService via ApiError

use axum::routing::post;
use axum::{
    extract::{Path, State},
    Json, Router,
};
use serde_json::Value;

use crate::middleware::error_handler::ApiError;
use crate::AppState;
use masday_core::AppError;

pub fn policy_routes() -> Router<AppState> {
    Router::new()
        .route("/policy/validate-execution", post(validate_execution))
        .route("/policy/validate-completion", post(validate_completion))
        .route("/policy/validate-parallel", post(validate_parallel))
        .route("/policy/drift/{workflow_id}", post(detect_drift))
        .route("/policy/session-readiness", post(check_session_readiness))
        .route("/policy/context-refresh", post(require_context_refresh))
}

/// Helper: run a policy validation, catching only NotFound errors as valid=false.
/// All other errors (auth, database, validation) propagate normally.
fn policy_result_to_valid(result: Result<bool, AppError>) -> Result<bool, ApiError> {
    match result {
        Ok(valid) => Ok(valid),
        Err(AppError::NotFound(_)) => Ok(false),
        Err(e) => Err(ApiError::from(e)),
    }
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

    // NotFound → valid=false; all other errors propagate
    let valid = policy_result_to_valid(
        masday_service::PolicyService::validate_execution(
            &state.pool,
            session_key,
            workflow_id,
            task_id,
        )
        .await,
    )?;

    Ok(Json(serde_json::json!({
        "valid": valid,
        "workflow_id": workflow_id,
        "task_id": task_id
    })))
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

    // NotFound → valid=false; all other errors propagate
    let valid = policy_result_to_valid(
        masday_service::PolicyService::validate_completion(
            &state.pool,
            session_key,
            workflow_id,
            task_id,
        )
        .await,
    )?;

    Ok(Json(serde_json::json!({
        "valid": valid,
        "workflow_id": workflow_id,
        "task_id": task_id
    })))
}

async fn validate_parallel(
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

    // Reuse completion validation for parallel — same logic applies
    let valid = policy_result_to_valid(
        masday_service::PolicyService::validate_completion(
            &state.pool,
            session_key,
            workflow_id,
            task_id,
        )
        .await,
    )?;

    Ok(Json(serde_json::json!({
        "valid": valid,
        "workflow_id": workflow_id,
        "task_id": task_id
    })))
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

async fn require_context_refresh(
    State(state): State<AppState>,
    Json(payload): Json<Value>,
) -> Json<Value> {
    let workflow_id = payload
        .get("workflow_id")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let task_id = payload
        .get("task_id")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    // Baseline = the task's recorded context fingerprint (None if no context).
    let baseline = if task_id.is_empty() {
        None
    } else {
        masday_db::repos::TaskRepo::new(state.pool.clone())
            .get_by_id(task_id)
            .await
            .ok()
            .and_then(|t| t.context_fingerprint)
    };

    // Observed: a caller-supplied last_fingerprint wins; otherwise compute it
    // from the observed context fields the caller declares.
    let observed = payload
        .get("last_fingerprint")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .or_else(|| {
            masday_service::compute_context_fingerprint(
                payload.get("skill").and_then(|v| v.as_str()),
                payload.get("input").filter(|v| !v.is_null()),
                payload.get("acceptance_criteria").filter(|v| !v.is_null()),
                payload.get("required_context").filter(|v| !v.is_null()),
            )
        });

    let result = masday_service::evaluate_context_drift(baseline.as_deref(), observed.as_deref());
    Json(serde_json::json!({
        "workflow_id": workflow_id,
        "task_id": task_id,
        "refresh_required": result.refresh_required,
        "reason": result.reason,
        "baseline_fingerprint": result.baseline_fingerprint,
        "observed_fingerprint": result.observed_fingerprint,
    }))
}
