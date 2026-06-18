//! Task routes — wired to TaskService via ApiError

use axum::routing::{get, post};
use axum::{
    extract::{Path, State},
    Json, Router,
};
use serde_json::Value;

use crate::middleware::error_handler::ApiError;
use crate::AppState;

pub fn task_routes() -> Router<AppState> {
    Router::new()
        .route("/tasks/{id}/start", post(start_task))
        .route("/tasks/{id}/complete", post(complete_task))
        .route("/tasks/{id}/fail", post(fail_task))
        .route("/tasks/{id}/progress", post(save_progress))
        .route("/tasks/{id}", get(get_task))
}

async fn start_task(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(payload): Json<Value>,
) -> Result<Json<Value>, ApiError> {
    let workflow_id = payload
        .get("workflow_id")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let task = masday_service::TaskService::start_task(&state.pool, workflow_id, &id).await?;
    Ok(Json(serde_json::json!(task)))
}

async fn complete_task(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(payload): Json<Value>,
) -> Result<Json<Value>, ApiError> {
    let workflow_id = payload
        .get("workflow_id")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let result = payload.get("result").cloned();
    let task =
        masday_service::TaskService::complete_task(&state.pool, workflow_id, &id, result).await?;
    Ok(Json(serde_json::json!(task)))
}

async fn fail_task(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(payload): Json<Value>,
) -> Result<Json<Value>, ApiError> {
    let workflow_id = payload
        .get("workflow_id")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let error = payload
        .get("error")
        .and_then(|v| v.as_str())
        .map(String::from);
    let task = masday_service::TaskService::fail_task(&state.pool, workflow_id, &id, error).await?;
    Ok(Json(serde_json::json!(task)))
}

async fn save_progress(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(payload): Json<Value>,
) -> Result<Json<Value>, ApiError> {
    let workflow_id = payload
        .get("workflow_id")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let agent = payload
        .get("agent_name")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown")
        .to_string();
    let note = payload
        .get("progress_note")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    masday_service::TaskService::save_progress(&state.pool, workflow_id, &id, agent, note, None)
        .await?;
    Ok(Json(serde_json::json!({"task_id": id, "saved": true})))
}

async fn get_task(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Value>, ApiError> {
    let task = masday_service::TaskService::get_current_task(&state.pool, &id).await?;
    Ok(Json(serde_json::json!(task)))
}
