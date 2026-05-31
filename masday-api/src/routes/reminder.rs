//! Reminder routes — wired to ReminderService via ApiError

use axum::routing::{get, post};
use axum::{
    extract::{Path, State},
    Json, Router,
};
use serde_json::Value;

use crate::middleware::error_handler::ApiError;
use crate::AppState;

pub fn reminder_routes() -> Router<AppState> {
    Router::new()
        .route("/reminders/check", get(check_reminders))
        .route("/reminders/:id/acknowledge", post(acknowledge_reminder))
        .route("/reminders", get(list_reminders))
}

async fn check_reminders(State(state): State<AppState>) -> Result<Json<Value>, ApiError> {
    let reminders = masday_service::ReminderService::check_reminders(&state.pool).await?;
    Ok(Json(serde_json::json!(reminders)))
}

async fn acknowledge_reminder(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Value>, ApiError> {
    masday_service::ReminderService::acknowledge_reminder(&state.pool, &id).await?;
    Ok(Json(serde_json::json!({"acknowledged": id})))
}

async fn list_reminders(State(state): State<AppState>) -> Result<Json<Value>, ApiError> {
    let reminders = masday_service::ReminderService::check_reminders(&state.pool).await?;
    Ok(Json(serde_json::json!(reminders)))
}
