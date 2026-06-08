//! Review routes — wired to ReviewService via ApiError

use axum::routing::{get, post};
use axum::{
    extract::{Path, Query, State},
    Json, Router,
};
use serde_json::Value;

use crate::middleware::error_handler::ApiError;
use crate::AppState;

#[derive(serde::Deserialize)]
struct LatestReviewQuery {
    #[allow(dead_code)]
    workflow_id: Option<String>,
    task_id: Option<String>,
}

pub fn review_routes() -> Router<AppState> {
    Router::new()
        .route("/reviews", post(submit_review))
        .route("/reviews/latest", get(get_latest_review))
        .route("/reviews/task/{task_id}", get(get_review_by_task))
}

async fn submit_review(
    State(state): State<AppState>,
    Json(payload): Json<Value>,
) -> Result<Json<Value>, ApiError> {
    let workflow_id = payload
        .get("workflow_id")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let task_id = payload
        .get("task_id")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let decision = payload
        .get("decision")
        .and_then(|v| v.as_str())
        .unwrap_or("APPROVED")
        .to_string();
    let reviewer = payload
        .get("reviewer_agent")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown")
        .to_string();
    let notes = payload
        .get("notes")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let gaps = payload.get("gaps").cloned();

    let review = masday_service::ReviewService::submit_review(
        &state.pool,
        workflow_id,
        task_id,
        reviewer,
        decision,
        notes,
        gaps,
    )
    .await?;
    Ok(Json(serde_json::json!(review)))
}

async fn get_review_by_task(
    State(state): State<AppState>,
    Path(task_id): Path<String>,
) -> Json<Value> {
    let approved = masday_service::ReviewService::is_approved(&state.pool, &task_id).await;
    Json(serde_json::json!({"task_id": task_id, "approved": approved}))
}

/// GET /reviews/latest?workflow_id=X&task_id=Y — get latest review decision
async fn get_latest_review(
    State(state): State<AppState>,
    Query(params): Query<LatestReviewQuery>,
) -> Result<Json<Value>, ApiError> {
    let task_id = params
        .task_id
        .as_deref()
        .unwrap_or("");

    if task_id.is_empty() {
        return Ok(Json(serde_json::json!({
            "review": null,
            "message": "task_id query parameter is required"
        })));
    }

    let repo = masday_db::repos::ReviewRepo::new(state.pool.clone());
    match repo.get_latest(task_id).await? {
        Some(review) => Ok(Json(serde_json::json!(review))),
        None => Ok(Json(serde_json::json!({
            "review": null,
            "task_id": task_id
        }))),
    }
}
