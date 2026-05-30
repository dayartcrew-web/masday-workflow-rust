//! Capability routes

use axum::{Json, Router};
use axum::routing::get;
use serde_json::Value;

pub fn capability_routes() -> Router {
    Router::new()
        .route("/capabilities/agents", get(list_agents))
        .route("/capabilities/skills", get(list_skills))
        .route("/capabilities/match", get(match_agent))
}

async fn list_agents() -> Json<Value> {
    // Placeholder implementation
    Json(serde_json::json!([]))
}

async fn list_skills() -> Json<Value> {
    // Placeholder implementation
    Json(serde_json::json!([]))
}

async fn match_agent() -> Json<Value> {
    // Placeholder implementation
    Json(serde_json::json!({"agent": null}))
}
