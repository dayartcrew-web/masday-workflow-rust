//! Review routes — wired to ReviewService via ApiError

use axum::routing::{get, post};
use axum::{
    extract::{Path, Query, State},
    Json, Router,
};
use masday_core::AppError;
use serde_json::Value;

use crate::middleware::error_handler::ApiError;
use crate::AppState;

#[derive(serde::Deserialize)]
struct LatestReviewQuery {
    workflow_id: Option<String>,
    task_id: Option<String>,
}

pub fn review_routes() -> Router<AppState> {
    Router::new()
        .route("/reviews", post(submit_review))
        .route("/reviews/latest", get(get_latest_review))
        .route("/reviews/task/{task_id}", get(get_review_by_task))
}

async fn submit_review(
    State(state): State<AppState>,
    Json(payload): Json<Value>,
) -> Result<Json<Value>, ApiError> {
    let workflow_id = payload
        .get("workflow_id")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let task_id = payload
        .get("task_id")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let decision = parse_decision(payload.get("decision"))?;
    let reviewer = payload
        .get("reviewer_agent")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown")
        .to_string();
    let notes = payload
        .get("notes")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let gaps = payload.get("gaps").cloned();

    let review = masday_service::ReviewService::submit_review(
        &state.pool,
        workflow_id,
        task_id,
        reviewer,
        decision,
        notes,
        gaps,
    )
    .await?;
    Ok(Json(serde_json::json!(review)))
}

async fn get_review_by_task(
    State(state): State<AppState>,
    Path(task_id): Path<String>,
    Query(params): Query<LatestReviewQuery>,
) -> Result<Json<Value>, ApiError> {
    let workflow_id = params.workflow_id.as_deref().unwrap_or("");

    if workflow_id.is_empty() {
        return Ok(Json(serde_json::json!({
            "task_id": task_id,
            "approved": false,
            "message": "workflow_id query parameter is required"
        })));
    }

    let approved =
        masday_service::ReviewService::is_approved(&state.pool, workflow_id, &task_id).await?;
    Ok(Json(
        serde_json::json!({"task_id": task_id, "approved": approved}),
    ))
}

/// GET /reviews/latest?workflow_id=X&task_id=Y — get latest review decision
async fn get_latest_review(
    State(state): State<AppState>,
    Query(params): Query<LatestReviewQuery>,
) -> Result<Json<Value>, ApiError> {
    let task_id = params.task_id.as_deref().unwrap_or("");
    let workflow_id = params.workflow_id.as_deref().unwrap_or("");

    if task_id.is_empty() || workflow_id.is_empty() {
        // #10: bad input (missing required query params) must be a 400, not a
        // 200 with a "message" body — a 200 hides the error from clients that
        // branch on status code.
        return Err(ApiError(masday_core::AppError::Validation(
            "workflow_id and task_id query parameters are required".into(),
        )));
    }

    let repo = masday_db::repos::ReviewRepo::new(state.pool.clone());
    match repo.get_latest(workflow_id, task_id).await? {
        Some(review) => Ok(Json(serde_json::json!(review))),
        None => Ok(Json(serde_json::json!({
            "review": null,
            "workflow_id": workflow_id,
            "task_id": task_id
        }))),
    }
}

/// Extract the review decision from the request payload.
///
/// A reviewer who omits `decision` must NOT silently auto-approve — the
/// completion gate relies on an explicit decision. Missing/empty/non-string
/// values are rejected with a 400 (AppError::Validation). Enum membership is
/// still enforced downstream by `ReviewService::submit_review`.
fn parse_decision(raw: Option<&Value>) -> Result<String, ApiError> {
    let decision = raw
        .and_then(|v| v.as_str())
        .ok_or_else(|| ApiError(AppError::Validation("decision is required".into())))?;
    let decision = decision.trim();
    if decision.is_empty() {
        return Err(ApiError(AppError::Validation(
            "decision is required".into(),
        )));
    }
    Ok(decision.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// Regression: omitting `decision` used to default to "APPROVED", silently
    /// auto-approving a review and defeating the completion gate. It must now
    /// return a 400 validation error.
    #[test]
    fn missing_decision_is_rejected() {
        let err = parse_decision(None).unwrap_err();
        assert!(
            matches!(err.0, AppError::Validation(ref m) if m.contains("decision is required")),
            "missing decision must be a Validation error, got {:?}",
            err.0
        );
    }

    /// A present-but-empty decision string must also be rejected — a reviewer
    /// who sends `"decision": ""` should not sneak through either.
    #[test]
    fn empty_decision_is_rejected() {
        let err = parse_decision(Some(&json!(""))).unwrap_err();
        assert!(matches!(err.0, AppError::Validation(_)));
        // Whitespace-only is also empty once trimmed.
        let err = parse_decision(Some(&json!("   "))).unwrap_err();
        assert!(matches!(err.0, AppError::Validation(_)));
    }

    /// A non-string `decision` (number/null/array) must be rejected rather than
    /// coerced.
    #[test]
    fn non_string_decision_is_rejected() {
        let err = parse_decision(Some(&json!(42))).unwrap_err();
        assert!(matches!(err.0, AppError::Validation(_)));
        let err = parse_decision(Some(&json!(null))).unwrap_err();
        assert!(matches!(err.0, AppError::Validation(_)));
    }

    /// A valid decision is returned unchanged (whitespace trimmed), and is not
    /// itself validated for enum membership here — that stays with ReviewService.
    #[test]
    fn valid_decision_passes_through() {
        let got = parse_decision(Some(&json!("REWORK_REQUIRED"))).unwrap();
        assert_eq!(got, "REWORK_REQUIRED");
        // Surrounding whitespace is trimmed.
        let got = parse_decision(Some(&json!("  APPROVED  "))).unwrap();
        assert_eq!(got, "APPROVED");
    }
}
