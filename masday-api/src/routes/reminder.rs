//! Reminder routes

use axum::{Json, Router};
use axum::routing::get;
use serde_json::Value;

pub fn reminder_routes() -> Router {
    Router::new()
        .route("/reminders/stale", get(check_stale))
        .route("/reminders/stuck", get(check_stuck))
}

async fn check_stale() -> Json<Value> {
    // Placeholder implementation
    Json(serde_json::json!([]))
}

async fn check_stuck() -> Json<Value> {
    // Placeholder implementation
    Json(serde_json::json!([]))
}
