//! Retrieval logs routes — CRUD for RetrievalLog

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
use masday_db::repos::RetrievalLogRepo;

#[derive(Deserialize)]
struct ListRetrievalLogsQuery {
    #[serde(default)]
    workflow_id: Option<String>,
    #[serde(default)]
    task_id: Option<String>,
}

pub fn retrieval_log_routes() -> Router<AppState> {
    Router::new()
        .route(
            "/retrieval-logs",
            post(create_retrieval_log).get(list_retrieval_logs),
        )
        .route(
            "/retrieval-logs/{id}",
            get(get_retrieval_log).delete(delete_retrieval_log),
        )
        .route(
            "/retrieval-logs/workflow/{workflow_id}",
            get(list_by_workflow),
        )
        .route("/retrieval-logs/task/{task_id}", get(list_by_task))
}

/// POST /retrieval-logs — Create a new retrieval log entry
async fn create_retrieval_log(
    State(state): State<AppState>,
    Json(payload): Json<Value>,
) -> Result<Json<Value>, ApiError> {
    let repo = RetrievalLogRepo::new(state.pool.clone());
    let new_log = masday_db::schema::NewRetrievalLog {
        workflow_id: payload
            .get("workflow_id")
            .and_then(|v| v.as_str())
            .map(String::from),
        task_id: payload
            .get("task_id")
            .and_then(|v| v.as_str())
            .map(String::from),
        agent_name: payload
            .get("agent_name")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string(),
        query: payload
            .get("query")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        source: payload
            .get("source")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string(),
        results: payload.get("results").cloned(),
    };
    let log = repo.create(&new_log).await?;
    Ok(Json(serde_json::json!(log)))
}

/// GET /retrieval-logs — List retrieval logs (with optional filters)
async fn list_retrieval_logs(
    State(state): State<AppState>,
    Query(params): Query<ListRetrievalLogsQuery>,
    pagination: Pagination,
) -> Result<Json<Value>, ApiError> {
    let repo = RetrievalLogRepo::new(state.pool.clone());
    let logs = if let Some(wid) = &params.workflow_id {
        repo.list_by_workflow(wid).await?
    } else if let Some(tid) = &params.task_id {
        repo.list_by_task(tid).await?
    } else {
        repo.list_all(Some(pagination.limit() as i64)).await?
    };
    Ok(Json(serde_json::json!(logs)))
}

/// GET /retrieval-logs/{id} — Get a retrieval log by ID
async fn get_retrieval_log(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Value>, ApiError> {
    let repo = RetrievalLogRepo::new(state.pool.clone());
    let log = repo.get_by_id(&id).await?;
    Ok(Json(serde_json::json!(log)))
}

/// GET /retrieval-logs/workflow/{workflow_id} — List retrieval logs for a workflow
async fn list_by_workflow(
    State(state): State<AppState>,
    Path(workflow_id): Path<String>,
) -> Result<Json<Value>, ApiError> {
    let repo = RetrievalLogRepo::new(state.pool.clone());
    let logs = repo.list_by_workflow(&workflow_id).await?;
    Ok(Json(serde_json::json!(logs)))
}

/// GET /retrieval-logs/task/{task_id} — List retrieval logs for a task
async fn list_by_task(
    State(state): State<AppState>,
    Path(task_id): Path<String>,
) -> Result<Json<Value>, ApiError> {
    let repo = RetrievalLogRepo::new(state.pool.clone());
    let logs = repo.list_by_task(&task_id).await?;
    Ok(Json(serde_json::json!(logs)))
}

/// DELETE /retrieval-logs/{id} — Delete a retrieval log entry
async fn delete_retrieval_log(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Value>, ApiError> {
    let repo = RetrievalLogRepo::new(state.pool.clone());
    let deleted = repo.delete(&id).await?;
    Ok(Json(serde_json::json!({"deleted": deleted, "id": id})))
}
