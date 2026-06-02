//! Progress logs routes — CRUD for TaskProgressLog

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
use masday_db::repos::ProgressLogRepo;

#[derive(Deserialize)]
struct ListProgressLogsQuery {
    #[serde(default)]
    workflow_id: Option<String>,
    #[serde(default)]
    task_id: Option<String>,
}

pub fn progress_log_routes() -> Router<AppState> {
    Router::new()
        .route(
            "/progress-logs",
            post(create_progress_log).get(list_progress_logs),
        )
        .route(
            "/progress-logs/{id}",
            get(get_progress_log).delete(delete_progress_log),
        )
        .route(
            "/progress-logs/workflow/{workflow_id}",
            get(list_by_workflow),
        )
        .route("/progress-logs/task/{task_id}", get(list_by_task))
}

/// POST /progress-logs — Create a new progress log entry
async fn create_progress_log(
    State(state): State<AppState>,
    Json(payload): Json<Value>,
) -> Result<Json<Value>, ApiError> {
    let repo = ProgressLogRepo::new(state.pool.clone());
    let new_log = masday_db::schema::NewTaskProgressLog {
        workflow_id: payload
            .get("workflow_id")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        task_id: payload
            .get("task_id")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        agent_name: payload
            .get("agent_name")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string(),
        status_before: payload
            .get("status_before")
            .and_then(|v| v.as_str())
            .map(String::from),
        status_after: payload
            .get("status_after")
            .and_then(|v| v.as_str())
            .map(String::from),
        progress_note: payload
            .get("progress_note")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        evidence: payload.get("evidence").cloned(),
    };
    let log = repo.create(&new_log).await?;
    Ok(Json(serde_json::json!(log)))
}

/// GET /progress-logs — List progress logs (with optional filters)
async fn list_progress_logs(
    State(state): State<AppState>,
    Query(params): Query<ListProgressLogsQuery>,
    pagination: Pagination,
) -> Result<Json<Value>, ApiError> {
    let repo = ProgressLogRepo::new(state.pool.clone());
    let logs = if let Some(wid) = &params.workflow_id {
        repo.list_by_workflow(wid).await?
    } else if let Some(tid) = &params.task_id {
        repo.list_by_task(tid).await?
    } else {
        repo.list_all(Some(pagination.limit() as i64)).await?
    };
    Ok(Json(serde_json::json!(logs)))
}

/// GET /progress-logs/{id} — Get a progress log by ID
async fn get_progress_log(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Value>, ApiError> {
    let repo = ProgressLogRepo::new(state.pool.clone());
    let log = repo.get_by_id(&id).await?;
    Ok(Json(serde_json::json!(log)))
}

/// GET /progress-logs/workflow/{workflow_id} — List progress logs for a workflow
async fn list_by_workflow(
    State(state): State<AppState>,
    Path(workflow_id): Path<String>,
) -> Result<Json<Value>, ApiError> {
    let repo = ProgressLogRepo::new(state.pool.clone());
    let logs = repo.list_by_workflow(&workflow_id).await?;
    Ok(Json(serde_json::json!(logs)))
}

/// GET /progress-logs/task/{task_id} — List progress logs for a task
async fn list_by_task(
    State(state): State<AppState>,
    Path(task_id): Path<String>,
) -> Result<Json<Value>, ApiError> {
    let repo = ProgressLogRepo::new(state.pool.clone());
    let logs = repo.list_by_task(&task_id).await?;
    Ok(Json(serde_json::json!(logs)))
}

/// DELETE /progress-logs/{id} — Delete a progress log entry
async fn delete_progress_log(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Value>, ApiError> {
    let repo = ProgressLogRepo::new(state.pool.clone());
    let deleted = repo.delete(&id).await?;
    Ok(Json(serde_json::json!({"deleted": deleted, "id": id})))
}
