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
    // #7: this handler used to merge the entire payload into the `metadata`
    // jsonb blob, dropping every typed column (workflow_id, plan_id, task_id,
    // execution_mode, synthesis_ready, verification_ready, context_fingerprint,
    // ...). Delegate to `SessionRepo::patch_state`, which routes each known key
    // to its typed column on BOTH the insert and update paths (mirrors the
    // mark_*_ready / set_execution_mode handlers). Metadata is left untouched
    // when the patch carries no `metadata` key; callers wanting arbitrary data
    // persisted should send it under that key.
    let patch = normalize_session_patch(&payload);
    let repo = masday_db::repos::session_repo::SessionRepo::new(state.pool.clone());
    repo.patch_state(&id, patch).await.map_err(|e| {
        ApiError(masday_core::AppError::Internal(format!(
            "Failed to patch session state: {}",
            e
        )))
    })?;

    Ok(Json(
        serde_json::json!({"session_key": id, "updated": true}),
    ))
}

/// Normalize a session-patch payload: callers may wrap the real fields in a
/// `patch` key (legacy shape used by `session_patch_state` on the stdio path)
/// or send them at the top level. Returns the object to hand to
/// `SessionRepo::patch_state`. Pure so it's unit-testable without a DB.
fn normalize_session_patch(payload: &Value) -> Value {
    match payload.get("patch") {
        Some(v) if v.is_object() => v.clone(),
        _ => payload.clone(),
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// Regression (#7): the payload fields must reach `patch_state` regardless
    /// of whether the caller wraps them in a `patch` key. A wrapped payload
    /// unwraps to the inner object so the typed columns persist.
    #[test]
    fn normalize_unwraps_legacy_patch_key() {
        let payload = json!({
            "patch": {"workflow_id": "wf-1", "execution_mode": "sequential"}
        });
        let got = normalize_session_patch(&payload);
        assert_eq!(got["workflow_id"], "wf-1");
        assert_eq!(got["execution_mode"], "sequential");
        // The outer `patch` wrapper is gone — no double-nesting.
        assert!(got.get("patch").is_none());
    }

    /// Top-level fields (no wrapper) pass through unchanged — both shapes are
    /// accepted.
    #[test]
    fn normalize_passes_through_top_level_fields() {
        let payload = json!({
            "workflow_id": "wf-2",
            "synthesis_ready": true
        });
        let got = normalize_session_patch(&payload);
        assert_eq!(got["workflow_id"], "wf-2");
        assert_eq!(got["synthesis_ready"], true);
    }

    /// A non-object `patch` value (string/scalar) must NOT be trusted as the
    /// patch — fall back to the whole payload so the typed columns are still
    /// extracted, rather than handing a non-object to patch_state.
    #[test]
    fn normalize_ignores_non_object_patch_value() {
        let payload = json!({"patch": "not-an-object", "task_id": "t-3"});
        let got = normalize_session_patch(&payload);
        // Falls back to the whole payload; task_id survives.
        assert_eq!(got["task_id"], "t-3");
    }
}
