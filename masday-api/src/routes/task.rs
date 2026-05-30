//! Task routes

use axum::{Json, Router};
use axum::routing::{get, post};
use axum::extract::Path;
use uuid::Uuid;
use serde_json::Value;

pub fn task_routes() -> Router {
    Router::new()
        .route("/tasks/:id/start", post(start_task))
        .route("/tasks/:id/complete", post(complete_task))
        .route("/tasks/:id/progress", post(save_progress))
}

async fn start_task(Path(id): Path<Uuid>) -> Json<Value> {
    // Placeholder implementation
    Json(serde_json::json!({"task_id": id, "status": "RUNNING"}))
}

async fn complete_task(Path(id): Path<Uuid>) -> Json<Value> {
    // Placeholder implementation
    Json(serde_json::json!({"task_id": id, "status": "DONE"}))
}

async fn save_progress(Path(id): Path<Uuid>, Json(payload): Json<Value>) -> Json<Value> {
    // Placeholder implementation
    Json(serde_json::json!({"task_id": id, "saved": true}))
}
