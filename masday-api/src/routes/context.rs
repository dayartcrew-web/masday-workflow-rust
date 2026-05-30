//! Context routes

use axum::{Json, Router};
use axum::routing::{get, post};
use axum::extract::Path;
use uuid::Uuid;
use serde_json::Value;

pub fn context_routes() -> Router {
    Router::new()
        .route("/context/pack/:workflow_id", get(build_context_pack))
        .route("/context/fingerprint", post(compute_fingerprint))
}

async fn build_context_pack(Path(workflow_id): Path<Uuid>) -> Json<Value> {
    // Placeholder implementation
    Json(serde_json::json!({"workflow_id": workflow_id, "context": {}}))
}

async fn compute_fingerprint() -> Json<Value> {
    // Placeholder implementation
    Json(serde_json::json!({"fingerprint": ""}))
}
