//! Context routes — wired to ContextService and SearchService via ApiError

use axum::routing::{get, post};
use axum::{
    extract::{Path, Query, State},
    Json, Router,
};
use masday_db::repos::RetrievalLogRepo;
use masday_db::schema::NewRetrievalLog;
use serde::Deserialize;
use serde_json::Value;
use tracing::warn;

use crate::middleware::error_handler::ApiError;
use crate::AppState;

/// Best-effort write of a retrieval-log row. Never fails the request: a database
/// error is logged via `tracing::warn` and swallowed. Retrieval logging is a
/// side-effect of search — the caller asked for search results, not a log write,
/// so a logging failure must never alter or block the response.
async fn log_retrieval(
    pool: &masday_db::DbPool,
    workflow_id: Option<String>,
    task_id: Option<String>,
    agent_name: Option<&str>,
    query: &str,
    source: &str,
    results: Option<Value>,
) {
    let log = NewRetrievalLog {
        workflow_id,
        task_id,
        agent_name: agent_name.unwrap_or("api").to_string(),
        query: query.to_string(),
        source: source.to_string(),
        results,
    };
    if let Err(e) = RetrievalLogRepo::new(pool.clone()).create(&log).await {
        warn!("retrieval_log write failed ({}): {}", source, e);
    }
}

pub fn context_routes() -> Router<AppState> {
    Router::new()
        .route("/context/pack/{workflow_id}", get(build_context_pack))
        .route("/context/fingerprint", post(compute_fingerprint))
        .route(
            "/context/pack/{workflow_id}/{plan_id}/{task_id}",
            get(build_context_pack_full),
        )
        // Semantic search routes
        .route("/context/search", get(code_search))
        .route("/context/hybrid-search", post(hybrid_search))
        .route("/context/fingerprint-search", post(fingerprint_search))
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

#[derive(Deserialize)]
struct SearchQuery {
    query: String,
    /// Optional attribution so a caller can tie the retrieval to a workflow/task.
    #[serde(default)]
    workflow_id: Option<String>,
    #[serde(default)]
    task_id: Option<String>,
    #[serde(default)]
    agent_name: Option<String>,
}

/// Code search — uses filesystem + BM25 search
async fn code_search(
    State(state): State<AppState>,
    Query(params): Query<SearchQuery>,
) -> Result<Json<Value>, ApiError> {
    let results = masday_service::SearchService::code_search(&params.query, ".")
        .await
        .map_err(|e| masday_core::AppError::internal(e.to_string()))?;
    let count = results
        .get("results")
        .and_then(|v| v.as_array())
        .map(|a| a.len())
        .unwrap_or(0);

    // Log the retrieval (best-effort; never affects the response).
    let summary = masday_service::summarize_retrieval_results(&results);
    log_retrieval(
        &state.pool,
        params.workflow_id.clone(),
        params.task_id.clone(),
        params.agent_name.as_deref(),
        &params.query,
        "code_search",
        Some(summary),
    )
    .await;

    Ok(Json(serde_json::json!({
        "query": params.query,
        "results": results.get("results").cloned().unwrap_or(serde_json::json!([])),
        "count": count
    })))
}

#[derive(Deserialize)]
struct HybridSearchInput {
    workflow_id: String,
    plan_id: String,
    task_id: String,
    /// Optional agent attribution for the retrieval log.
    #[serde(default)]
    agent_name: Option<String>,
}

/// Hybrid context pack — combines semantic search + fingerprinting
async fn hybrid_search(
    State(state): State<AppState>,
    Json(input): Json<HybridSearchInput>,
) -> Result<Json<Value>, ApiError> {
    // Not-found is expected for new workflows with no plan yet — return empty pack
    // Real errors (database failures) propagate as 500
    let pack = match masday_service::ContextService::build_context_pack(
        &state.pool,
        &input.workflow_id,
        &input.plan_id,
        &input.task_id,
    )
    .await
    {
        Ok(p) => p,
        Err(e) => {
            // NotFound → empty pack (workflow exists but no plan/task yet)
            // All other errors propagate normally
            if matches!(e, masday_core::AppError::NotFound(_)) {
                serde_json::json!({"tasks": [], "plan": null})
            } else {
                return Err(ApiError::from(e));
            }
        }
    };

    // Compute fingerprint for change detection
    let fingerprint = masday_service::ContextService::compute_fingerprint(
        &input.workflow_id,
        &input.plan_id,
        &input.task_id,
    );

    // Log the context-pack retrieval (best-effort; never affects the response).
    let summary = serde_json::json!({
        "fingerprint": fingerprint,
        "task_count": pack.get("tasks").and_then(|v| v.as_array()).map(|a| a.len()).unwrap_or(0),
    });
    let task_id_log = if input.task_id.is_empty() {
        None
    } else {
        Some(input.task_id.clone())
    };
    let query = if !input.task_id.is_empty() {
        input.task_id.as_str()
    } else {
        input.workflow_id.as_str()
    };
    log_retrieval(
        &state.pool,
        Some(input.workflow_id.clone()),
        task_id_log,
        input.agent_name.as_deref(),
        query,
        "hybrid_context_pack",
        Some(summary),
    )
    .await;

    Ok(Json(serde_json::json!({
        "contextPack": pack,
        "fingerprint": fingerprint,
        "workflowId": input.workflow_id,
        "planId": input.plan_id,
        "taskId": input.task_id
    })))
}

#[derive(Deserialize)]
struct FingerprintSearchInput {
    workflow_id: String,
    plan_id: String,
    task_id: String,
    /// Optional agent attribution for the retrieval log.
    #[serde(default)]
    agent_name: Option<String>,
}

/// Fingerprint search — compute and compare context fingerprints
async fn fingerprint_search(
    State(state): State<AppState>,
    Json(input): Json<FingerprintSearchInput>,
) -> Result<Json<Value>, ApiError> {
    let fingerprint = masday_service::ContextService::compute_fingerprint(
        &input.workflow_id,
        &input.plan_id,
        &input.task_id,
    );

    // Log the fingerprint retrieval (best-effort; never affects the response).
    let summary = serde_json::json!({ "fingerprint": fingerprint });
    let task_id_log = if input.task_id.is_empty() {
        None
    } else {
        Some(input.task_id.clone())
    };
    let query = if !input.task_id.is_empty() {
        input.task_id.as_str()
    } else {
        input.workflow_id.as_str()
    };
    log_retrieval(
        &state.pool,
        Some(input.workflow_id.clone()),
        task_id_log,
        input.agent_name.as_deref(),
        query,
        "context_fingerprint",
        Some(summary),
    )
    .await;

    Ok(Json(serde_json::json!({
        "fingerprint": fingerprint,
        "workflowId": input.workflow_id,
        "planId": input.plan_id,
        "taskId": input.task_id
    })))
}
