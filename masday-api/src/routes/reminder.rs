//! Reminder routes — wired to ReminderService via ApiError

use axum::routing::{get, post};
use axum::{
    extract::{Path, Query, State},
    Json, Router,
};
use serde::Deserialize;
use serde_json::Value;

use crate::middleware::error_handler::ApiError;
use crate::AppState;

/// Optional reminder query params. `stuck_task_minutes` carries the advertised
/// `stuckTaskMinutes` MCP param through to the service's stuck-task pass;
/// absent (or `< 1`) falls back to the 60-minute default via
/// [`masday_service::reminder_service::resolve_stuck_task_threshold`], so a
/// request with no query string behaves exactly as before.
#[derive(Debug, Default, Deserialize)]
struct ReminderQuery {
    stuck_task_minutes: Option<i64>,
    /// Advertised as the `includeFailed` MCP param — when true, FAILED workflows
    /// are also checked against the FAILED-staleness threshold. Defaults to
    /// false (legacy behavior: FAILED workflows are excluded).
    include_failed: Option<bool>,
}

pub fn reminder_routes() -> Router<AppState> {
    Router::new()
        .route("/reminders", get(list_reminders))
        .route("/reminders/check", get(check_reminders))
        .route("/reminders/stale", get(get_stale))
        .route("/reminders/stuck", get(get_stuck))
        .route("/reminders/{id}/acknowledge", post(acknowledge_reminder))
}

async fn check_reminders(
    State(state): State<AppState>,
    Query(q): Query<ReminderQuery>,
) -> Result<Json<Value>, ApiError> {
    let threshold =
        masday_service::reminder_service::resolve_stuck_task_threshold(q.stuck_task_minutes);
    let include_failed = q.include_failed.unwrap_or(false);
    let reminders = masday_service::ReminderService::check_reminders_with_options(
        &state.pool,
        threshold,
        include_failed,
    )
    .await?;
    Ok(Json(serde_json::json!(reminders)))
}

async fn get_stale(
    State(state): State<AppState>,
    Query(q): Query<ReminderQuery>,
) -> Result<Json<Value>, ApiError> {
    let threshold =
        masday_service::reminder_service::resolve_stuck_task_threshold(q.stuck_task_minutes);
    let include_failed = q.include_failed.unwrap_or(false);
    let reminders = masday_service::ReminderService::check_reminders_with_options(
        &state.pool,
        threshold,
        include_failed,
    )
    .await?;
    Ok(Json(serde_json::json!(reminders)))
}

async fn get_stuck(
    State(state): State<AppState>,
    Query(q): Query<ReminderQuery>,
) -> Result<Json<Value>, ApiError> {
    let threshold =
        masday_service::reminder_service::resolve_stuck_task_threshold(q.stuck_task_minutes);
    let include_failed = q.include_failed.unwrap_or(false);
    let reminders = masday_service::ReminderService::check_reminders_with_options(
        &state.pool,
        threshold,
        include_failed,
    )
    .await?;
    Ok(Json(serde_json::json!(reminders)))
}

async fn acknowledge_reminder(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Value>, ApiError> {
    masday_service::ReminderService::acknowledge_reminder(&state.pool, &id).await?;
    Ok(Json(serde_json::json!({"acknowledged": id})))
}

async fn list_reminders(
    State(state): State<AppState>,
    Query(q): Query<ReminderQuery>,
) -> Result<Json<Value>, ApiError> {
    let threshold =
        masday_service::reminder_service::resolve_stuck_task_threshold(q.stuck_task_minutes);
    let include_failed = q.include_failed.unwrap_or(false);
    let reminders = masday_service::ReminderService::check_reminders_with_options(
        &state.pool,
        threshold,
        include_failed,
    )
    .await?;
    Ok(Json(serde_json::json!(reminders)))
}
