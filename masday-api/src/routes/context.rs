//! Context routes — wired to ContextService via ApiError

use axum::routing::{get, post};
use axum::{
    extract::{Path, State},
    Json, Router,
};
use serde::Deserialize;
use serde_json::Value;

use crate::middleware::error_handler::ApiError;
use crate::AppState;

pub fn context_routes() -> Router<AppState> {
    Router::new()
        .route("/context/pack/:workflow_id", get(build_context_pack))
        .route("/context/fingerprint", post(compute_fingerprint))
        .route(
            "/context/pack/:workflow_id/:plan_id/:task_id",
            get(build_context_pack_full),
        )
}

async fn build_context_pack(
    State(state): State<AppState>,
    Path(workflow_id): Path<String>,
) -> Result<Json<Value>, ApiError> {
    let pack = masday_service::ContextService::build_context_pack(
        &state.pool,
        &workflow_id,
        &workflow_id,
        &workflow_id,
    )
    .await?;
    Ok(Json(pack))
}

async fn build_context_pack_full(
    State(state): State<AppState>,
    Path((workflow_id, plan_id, task_id)): Path<(String, String, String)>,
) -> Result<Json<Value>, ApiError> {
    let pack = masday_service::ContextService::build_context_pack(
        &state.pool,
        &workflow_id,
        &plan_id,
        &task_id,
    )
    .await?;
    Ok(Json(pack))
}

#[derive(Deserialize)]
struct FingerprintInput {
    workflow_id: String,
    plan_id: String,
    task_id: String,
}

async fn compute_fingerprint(Json(input): Json<FingerprintInput>) -> Json<Value> {
    let fp = masday_service::ContextService::compute_fingerprint(
        &input.workflow_id,
        &input.plan_id,
        &input.task_id,
    );
    Json(serde_json::json!({"fingerprint": fp}))
}
