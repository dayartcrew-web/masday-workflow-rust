//! Session routes — wired to PostgreSQL via ApiError

use axum::routing::{get, post};
use axum::{
    extract::{Path, State},
    Json, Router,
};
use serde_json::Value;

use crate::middleware::error_handler::ApiError;
use crate::AppState;

pub fn session_routes() -> Router<AppState> {
    Router::new()
        .route("/sessions/{id}", get(get_session).patch(update_session))
        .route("/sessions/{id}/init", post(init_session))
}

async fn get_session(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Value>, ApiError> {
    let client = state
        .pool
        .get()
        .await
        .map_err(|e| masday_core::AppError::database(e.to_string()))?;
    let result = client
        .query_opt(
            "SELECT session_key, metadata, execution_mode, workflow_id, plan_id, task_id FROM session_states WHERE session_key = $1",
            &[&id],
        )
        .await
        .map_err(|e| masday_core::AppError::database(e.to_string()))?;

    match result {
        Some(row) => {
            let meta: Option<serde_json::Value> = row.get("metadata");
            Ok(Json(serde_json::json!({
                "session_key": id,
                "state": meta.unwrap_or(serde_json::json!({})),
                "workflow_id": row.get::<_, Option<String>>("workflow_id"),
                "plan_id": row.get::<_, Option<String>>("plan_id"),
                "task_id": row.get::<_, Option<String>>("task_id"),
            })))
        }
        None => Ok(Json(serde_json::json!({"session_key": id, "state": null}))),
    }
}

async fn update_session(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(payload): Json<Value>,
) -> Result<Json<Value>, ApiError> {
    let client = state
        .pool
        .get()
        .await
        .map_err(|e| masday_core::AppError::database(e.to_string()))?;

    // Extract patch string or use entire payload as metadata
    let patch = payload
        .get("patch")
        .cloned()
        .unwrap_or_else(|| payload.clone());

    client.execute(
        "INSERT INTO session_states (id, session_key, metadata, updated_at, created_at) VALUES ($1, $2, $3, NOW(), NOW()) ON CONFLICT (session_key) DO UPDATE SET metadata = COALESCE(session_states.metadata, '{}'::jsonb) || $3, updated_at = NOW()",
        &[&id, &id, &patch],
    ).await.map_err(|e| masday_core::AppError::database(e.to_string()))?;

    Ok(Json(
        serde_json::json!({"session_key": id, "updated": true}),
    ))
}

async fn init_session(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Value>, ApiError> {
    let client = state
        .pool
        .get()
        .await
        .map_err(|e| masday_core::AppError::database(e.to_string()))?;
    client.execute(
        "INSERT INTO session_states (id, session_key, metadata, updated_at, created_at) VALUES ($1, $2, '{}'::jsonb, NOW(), NOW()) ON CONFLICT (session_key) DO NOTHING",
        &[&id, &id],
    ).await.map_err(|e| masday_core::AppError::database(e.to_string()))?;

    Ok(Json(
        serde_json::json!({"session_key": id, "initialized": true}),
    ))
}
