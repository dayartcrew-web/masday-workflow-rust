//! Review routes

use axum::{Json, Router};
use axum::routing::{get, post};
use axum::extract::Path;
use serde_json::Value;
use uuid::Uuid;

pub fn review_routes() -> Router {
    Router::new()
        .route("/reviews", post(submit_review))
        .route("/reviews/task/:task_id", get(get_review_by_task))
}

async fn submit_review(Json(payload): Json<Value>) -> Json<Value> {
    // Placeholder implementation
    Json(serde_json::json!({"id": Uuid::new_v4()}))
}

async fn get_review_by_task(Path(task_id): Path<Uuid>) -> Json<Value> {
    // Placeholder implementation
    Json(serde_json::json!({"task_id": task_id, "review": null}))
}
