//! Context routes — wired to ContextService via ApiError

use axum::routing::{get, post};
use axum::{
    extract::{Path, Query, State},
    Json, Router,
};
use masday_db::repos::{CodeChunkRepo, RetrievalLogRepo};
use masday_db::schema::NewRetrievalLog;
use masday_service::embedding_service::EmbeddingService;
use serde::Deserialize;
use serde_json::{json, Value};
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
    /// Project path scoping the `code_chunks` index. Defaults to "." (whole project).
    /// Callers should forward the canonical absolute path (the stdio MCP resolver does).
    #[serde(default)]
    project_path: Option<String>,
    /// Max results. Defaults to 20, clamped to [1, 50].
    #[serde(default)]
    limit: Option<u64>,
}

/// Code search — pgvector semantic search over the shared PostgreSQL `code_chunks`
/// index. Falls back to an empty `pgvector_unavailable` body (HTTP 200) when no
/// embedding service is configured or embedding/vector search fails — semantic
/// search being unavailable is not a request error. Mirrors the embed→vector_search
/// pattern used by memory search (`routes/memory.rs`).
async fn code_search(
    State(state): State<AppState>,
    Query(params): Query<SearchQuery>,
) -> Result<Json<Value>, ApiError> {
    let project_path = params.project_path.as_deref().unwrap_or(".");
    // Clamp limit to a reasonable range; default 20.
    let limit = params.limit.unwrap_or(20).clamp(1, 50) as i64;

    let (body, source) = code_search_inner(&state.pool, &params.query, project_path, limit).await;

    // Log the retrieval (best-effort; never affects the response).
    let summary = masday_service::summarize_retrieval_results(&body);
    log_retrieval(
        &state.pool,
        params.workflow_id.clone(),
        params.task_id.clone(),
        params.agent_name.as_deref(),
        &params.query,
        source,
        Some(summary),
    )
    .await;

    Ok(Json(body))
}

/// Core of [`code_search`], factored out so the embed→vector_search flow is
/// distinct from the graceful-degradation path. Returns `(response_body, source_tag)`
/// where `source_tag` feeds both the JSON `source` field and retrieval logging.
async fn code_search_inner(
    pool: &masday_db::DbPool,
    query: &str,
    project_path: &str,
    limit: i64,
) -> (Value, &'static str) {
    // Canonicalize the same way the indexer does so the `WHERE project_path = $2`
    // lookup matches. On the API server the path usually isn't on the local fs, so
    // canonicalize fails and the (already-canonical) input is returned unchanged.
    let canonical = masday_db::repos::normalize_project_path(project_path);

    if let Some(service) = EmbeddingService::cached() {
        match service.embed(query).await {
            Ok(vec) if !vec.is_empty() => {
                let repo = CodeChunkRepo::new(pool.clone());
                match repo.vector_search(&vec, &canonical, limit).await {
                    Ok(results) => {
                        let mapped: Vec<Value> = results
                            .into_iter()
                            .map(|r| {
                                json!({
                                    "file_path": r.file_path,
                                    "language": r.language,
                                    "chunk_type": r.chunk_type,
                                    "name": r.name,
                                    "start_line": r.start_line,
                                    "end_line": r.end_line,
                                    "content": r.content,
                                    "similarity": r.similarity,
                                })
                            })
                            .collect();
                        let count = mapped.len();
                        let body = json!({
                            "query": query,
                            "project_path": canonical,
                            "results": mapped,
                            "count": count,
                            "source": "pgvector",
                        });
                        return (body, "pgvector");
                    }
                    Err(e) => warn!(
                        "code_chunks vector_search failed for project {}: {}",
                        canonical, e
                    ),
                }
            }
            Ok(_) => warn!("code_search: embedding service returned an empty vector"),
            Err(e) => warn!("code_search: query embedding failed: {}", e),
        }
    }

    // Degradation: semantic search unavailable. HTTP 200, not an error.
    (degradation_body(query, &canonical), "pgvector_unavailable")
}

/// Graceful-degradation response body. Pure (no I/O) so it is unit-testable
/// without a database pool or a configured embedding service.
fn degradation_body(query: &str, canonical_project_path: &str) -> Value {
    let reason = if EmbeddingService::cached().is_none() {
        "embedding service not configured"
    } else {
        "embedding generation or vector search failed (see logs)"
    };
    json!({
        "query": query,
        "project_path": canonical_project_path,
        "results": [],
        "count": 0,
        "source": "pgvector_unavailable",
        "reason": reason,
    })
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

#[cfg(test)]
mod tests {
    use super::*;
    use masday_service::embedding_service::EmbeddingService;

    /// When no embedding service is configured (the usual unit-test state —
    /// `EMBEDDING_PROVIDER` unset → `EmbeddingService::cached()` is None),
    /// `degradation_body` must report `pgvector_unavailable` with empty results
    /// and the "not configured" reason. Pure helper — no DB pool needed.
    #[test]
    fn degradation_body_when_no_embedding_service() {
        if EmbeddingService::cached().is_some() {
            // A provider is configured in this environment; the "not configured"
            // branch can't be exercised — skip rather than spuriously fail.
            eprintln!("skipped: embedding service configured in this env");
            return;
        }
        let body = degradation_body("async runtime", "/abs/project");
        assert_eq!(body["source"], "pgvector_unavailable");
        assert_eq!(body["count"], 0);
        assert_eq!(body["query"], "async runtime");
        assert_eq!(body["project_path"], "/abs/project");
        assert_eq!(
            body["reason"], "embedding service not configured",
            "reason must reflect the no-service case"
        );
        assert_eq!(body["results"].as_array().unwrap().len(), 0);
    }

    /// `degradation_body` shape is consistent regardless of why search degraded:
    /// source/count/results are identical; only the reason wording differs.
    #[test]
    fn degradation_body_shape_is_consistent() {
        let body = degradation_body("q", "/p");
        assert_eq!(body["source"], "pgvector_unavailable");
        assert_eq!(body["count"], 0);
        assert!(body["results"].as_array().unwrap().is_empty());
        assert!(body["reason"].as_str().is_some());
    }
}
