//! Plan routes — wired to PlanService via ApiError

use axum::routing::{get, patch};
use axum::{
    extract::{Path, State},
    Json, Router,
};
use serde_json::Value;

use crate::middleware::error_handler::ApiError;
use crate::AppState;

pub fn plan_routes() -> Router<AppState> {
    Router::new()
        .route("/plans/{workflow_id}", get(get_plan))
        .route("/plans/{workflow_id}/status", patch(update_plan_status))
}

async fn get_plan(
    State(state): State<AppState>,
    Path(workflow_id): Path<String>,
) -> Result<Json<Value>, ApiError> {
    let plan = masday_service::PlanService::get_plan(&state.pool, &workflow_id).await?;
    Ok(Json(serde_json::json!(plan)))
}

async fn update_plan_status(
    State(state): State<AppState>,
    Path(workflow_id): Path<String>,
    Json(payload): Json<Value>,
) -> Result<Json<Value>, ApiError> {
    let status = payload
        .get("status")
        .and_then(|v| v.as_str())
        .unwrap_or("ACTIVE")
        .to_string();
    let plan =
        masday_service::PlanService::update_plan_status(&state.pool, &workflow_id, status).await?;
    Ok(Json(serde_json::json!(plan)))
}
