//! Health check routes

use axum::{Json, Router};
use axum::routing::get;
use serde_json::Value;

pub fn health_routes() -> Router {
    Router::new()
        .route("/health", get(health_check))
        .route("/health/db", get(db_health))
}

async fn health_check() -> Json<Value> {
    Json(serde_json::json!({
        "status": "healthy",
        "timestamp": chrono::Utc::now()
    }))
}

async fn db_health() -> Json<Value> {
    // Placeholder implementation - would check DB connection
    Json(serde_json::json!({
        "status": "healthy",
        "database": "connected"
    }))
}
