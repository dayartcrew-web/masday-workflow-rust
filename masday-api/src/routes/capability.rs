//! Capability routes — wired to CapabilityService via ApiError

use axum::routing::{get, post};
use axum::{
    extract::{Query, State},
    Json, Router,
};
use serde::Deserialize;
use serde_json::Value;

use crate::middleware::error_handler::ApiError;
use crate::AppState;

pub fn capability_routes() -> Router<AppState> {
    Router::new()
        .route("/capabilities/agents", get(list_agents))
        .route("/capabilities/skills", get(list_skills))
        .route("/capabilities/match", get(match_agent))
        .route("/capabilities/templates", get(list_templates))
        .route("/capabilities/scaffold-feature", post(scaffold_feature))
        .route("/capabilities/system-readiness", get(system_readiness))
}

#[derive(Deserialize)]
struct ProjectRootQuery {
    #[serde(default)]
    project_root: Option<String>,
    #[serde(default)]
    task_description: Option<String>,
}

async fn list_agents(Query(query): Query<ProjectRootQuery>) -> Result<Json<Value>, ApiError> {
    let root = query.project_root.as_deref().unwrap_or(".");
    let agents = masday_service::CapabilityService::list_agents(root).await?;
    Ok(Json(serde_json::json!(agents)))
}

async fn list_skills(_query: Query<ProjectRootQuery>) -> Json<Value> {
    // Stub: capability service does not yet expose list_skills
    Json(serde_json::json!([]))
}

async fn match_agent(Query(query): Query<ProjectRootQuery>) -> Result<Json<Value>, ApiError> {
    let desc = query.task_description.as_deref().unwrap_or("");
    let agent = masday_service::CapabilityService::match_agent(desc).await?;
    Ok(Json(serde_json::json!({"agent": agent})))
}

async fn list_templates() -> Json<Value> {
    Json(serde_json::json!([]))
}

async fn scaffold_feature(Json(_payload): Json<Value>) -> Json<Value> {
    Json(serde_json::json!({"scaffolded": false, "message": "Not yet implemented"}))
}

async fn system_readiness(State(state): State<AppState>) -> Json<Value> {
    let pool_status = masday_db::pool::get_pool_status(&state.pool);
    Json(serde_json::json!({
        "ready": true,
        "pool": {
            "size": pool_status.size,
            "available": pool_status.available,
            "max_size": pool_status.max_size,
        }
    }))
}
