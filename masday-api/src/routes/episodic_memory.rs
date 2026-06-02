//! Episodic memory routes — CRUD for EpisodicMemory

use axum::routing::{delete, get, post};
use axum::{
    extract::{Path, State},
    Json, Router,
};
use serde_json::Value;

use crate::middleware::error_handler::ApiError;
use crate::AppState;
use masday_db::repos::EpisodicMemoryRepo;

pub fn episodic_memory_routes() -> Router<AppState> {
    Router::new()
        .route(
            "/episodic-memory",
            post(create_episodic_memory).get(list_episodic_memory),
        )
        .route(
            "/episodic-memory/{id}",
            get(get_episodic_memory).delete(delete_episodic_memory),
        )
        .route(
            "/episodic-memory/session/{session_id}",
            get(list_by_session),
        )
        .route(
            "/episodic-memory/session/{session_id}/clear",
            delete(delete_by_session),
        )
}

/// POST /episodic-memory — Create a new episodic memory entry
async fn create_episodic_memory(
    State(state): State<AppState>,
    Json(payload): Json<Value>,
) -> Result<Json<Value>, ApiError> {
    let repo = EpisodicMemoryRepo::new(state.pool.clone());
    let new_memory = masday_db::schema::NewEpisodicMemory {
        session_id: payload
            .get("session_id")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        role: payload
            .get("role")
            .and_then(|v| v.as_str())
            .unwrap_or("user")
            .to_string(),
        content: payload
            .get("content")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        sequence_order: payload
            .get("sequence_order")
            .and_then(|v| v.as_i64())
            .unwrap_or(0) as i32,
    };
    let memory = repo.create(&new_memory).await?;
    Ok(Json(serde_json::json!(memory)))
}

/// GET /episodic-memory — List all episodic memories
async fn list_episodic_memory(State(state): State<AppState>) -> Result<Json<Value>, ApiError> {
    let repo = EpisodicMemoryRepo::new(state.pool.clone());
    let memories = repo.list_all(Some(100)).await?;
    Ok(Json(serde_json::json!(memories)))
}

/// GET /episodic-memory/{id} — Get an episodic memory by ID
async fn get_episodic_memory(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Value>, ApiError> {
    let repo = EpisodicMemoryRepo::new(state.pool.clone());
    let memory = repo.get_by_id(&id).await?;
    Ok(Json(serde_json::json!(memory)))
}

/// GET /episodic-memory/session/{session_id} — List episodic memories for a session
async fn list_by_session(
    State(state): State<AppState>,
    Path(session_id): Path<String>,
) -> Result<Json<Value>, ApiError> {
    let repo = EpisodicMemoryRepo::new(state.pool.clone());
    let memories = repo.list_by_session(&session_id).await?;
    Ok(Json(serde_json::json!(memories)))
}

/// DELETE /episodic-memory/{id} — Delete an episodic memory entry
async fn delete_episodic_memory(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Value>, ApiError> {
    let repo = EpisodicMemoryRepo::new(state.pool.clone());
    let deleted = repo.delete(&id).await?;
    Ok(Json(serde_json::json!({"deleted": deleted, "id": id})))
}

/// DELETE /episodic-memory/session/{session_id}/clear — Delete all episodic memories for a session
async fn delete_by_session(
    State(state): State<AppState>,
    Path(session_id): Path<String>,
) -> Result<Json<Value>, ApiError> {
    let repo = EpisodicMemoryRepo::new(state.pool.clone());
    let count = repo.delete_by_session(&session_id).await?;
    Ok(Json(serde_json::json!({
        "deleted_count": count,
        "session_id": session_id
    })))
}
