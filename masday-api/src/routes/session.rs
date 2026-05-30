//! Session routes

use axum::{Json, Router};
use axum::routing::{get, patch};
use axum::extract::Path;
use serde_json::Value;

pub fn session_routes() -> Router {
    Router::new()
        .route("/sessions/:id", get(get_session).patch(update_session))
}

async fn get_session(Path(id): Path<String>) -> Json<Value> {
    // Placeholder implementation
    Json(serde_json::json!({"session_id": id, "state": null}))
}

async fn update_session(Path(id): Path<String>, Json(payload): Json<Value>) -> Json<Value> {
    // Placeholder implementation
    Json(serde_json::json!({"session_id": id, "updated": true}))
}
