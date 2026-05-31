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
            "SELECT state FROM session_states WHERE session_key = $1",
            &[&id],
        )
        .await
        .map_err(|e| masday_core::AppError::database(e.to_string()))?;

    match result {
        Some(row) => {
            let state_val: serde_json::Value = row.get("state");
            Ok(Json(
                serde_json::json!({"session_key": id, "state": state_val}),
            ))
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
    client.execute(
        "INSERT INTO session_states (session_key, state, updated_at) VALUES ($1, $2, NOW()) ON CONFLICT (session_key) DO UPDATE SET state = $2, updated_at = NOW()",
        &[&id, &payload],
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
        "INSERT INTO session_states (session_key, state, updated_at) VALUES ($1, '{}'::jsonb, NOW()) ON CONFLICT (session_key) DO NOTHING",
        &[&id],
    ).await.map_err(|e| masday_core::AppError::database(e.to_string()))?;

    Ok(Json(
        serde_json::json!({"session_key": id, "initialized": true}),
    ))
}
