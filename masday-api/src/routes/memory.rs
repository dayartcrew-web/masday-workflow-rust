//! Memory routes — wired to MemoryRepo via ApiError

use axum::routing::{delete, get, post};
use axum::{
    extract::{Path, Query, State},
    Json, Router,
};
use serde::Deserialize;
use serde_json::Value;

use crate::extractors::pagination::Pagination;
use crate::middleware::error_handler::ApiError;
use crate::AppState;
use masday_db::repos::MemoryRepo;

#[derive(Deserialize)]
struct ListMemoriesQuery {
    #[serde(default = "default_limit")]
    limit: i64,
    #[serde(default)]
    workflow_id: Option<String>,
}

pub fn memory_routes() -> Router<AppState> {
    Router::new()
        .route("/memories", post(store_memory).get(list_memories))
        .route("/memories/search", post(search_memories))
        .route("/memories/research", post(store_research))
        .route("/memories/recent", get(recall_recent))
        .route("/memories/by-type", get(recall_by_type))
        .route("/memories/by-task/{task_id}", get(recall_by_task))
        .route(
            "/memories/workflow/{workflow_id}",
            delete(delete_by_workflow),
        )
        .route("/memories/stats", get(memory_stats))
        .route(
            "/memories/{id}",
            get(get_memory).patch(update_memory).delete(delete_memory),
        )
}

#[derive(Deserialize)]
struct RecallQuery {
    #[serde(default = "default_limit")]
    limit: i64,
    #[serde(default)]
    memory_type: Option<String>,
}

fn default_limit() -> i64 {
    20
}

/// GET /memories — List memories, optionally filtered by workflow_id
async fn list_memories(
    State(state): State<AppState>,
    Query(params): Query<ListMemoriesQuery>,
) -> Result<Json<Value>, ApiError> {
    let repo = MemoryRepo::new(state.pool.clone());
    let memories = if let Some(wid) = &params.workflow_id {
        repo.recall_by_workflow(wid, params.limit).await?
    } else {
        repo.recall_recent(params.limit).await?
    };
    Ok(Json(serde_json::json!(memories)))
}

async fn store_memory(
    State(state): State<AppState>,
    Json(payload): Json<Value>,
) -> Result<Json<Value>, ApiError> {
    let repo = MemoryRepo::new(state.pool.clone());
    let new_mem = masday_db::schema::NewMemory {
        workflow_id: payload
            .get("workflow_id")
            .and_then(|v| v.as_str())
            .map(String::from),
        task_id: payload
            .get("task_id")
            .and_then(|v| v.as_str())
            .map(String::from),
        memory_type: payload
            .get("memory_type")
            .and_then(|v| v.as_str())
            .unwrap_or("fact")
            .to_string(),
        summary: payload
            .get("summary")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        content: payload
            .get("content")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        importance_score: payload
            .get("importance_score")
            .and_then(|v| v.as_f64())
            .or_else(|| {
                payload
                    .get("importance_score")
                    .and_then(|v| v.as_str())
                    .and_then(|s| s.parse::<f64>().ok())
            }),
        created_by_agent: payload
            .get("created_by_agent")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string(),
        tags: Some(
            payload
                .get("tags")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(String::from))
                        .collect()
                })
                .unwrap_or_default(),
        ),
        source: payload
            .get("source")
            .and_then(|v| v.as_str())
            .map(String::from),
        embedding: None,
    };
    let mem = repo.store(&new_mem).await?;
    Ok(Json(serde_json::json!(mem)))
}

async fn search_memories(
    State(state): State<AppState>,
    Json(payload): Json<Value>,
) -> Result<Json<Value>, ApiError> {
    let repo = MemoryRepo::new(state.pool.clone());
    let query = payload.get("query").and_then(|v| v.as_str()).unwrap_or("");
    let limit = payload.get("limit").and_then(|v| v.as_i64()).unwrap_or(20);
    let memories = repo.search(query, limit).await?;
    Ok(Json(serde_json::json!(memories)))
}

/// POST /memories/research — store research findings (reuses memory storage)
async fn store_research(
    State(state): State<AppState>,
    Json(payload): Json<Value>,
) -> Result<Json<Value>, ApiError> {
    let repo = MemoryRepo::new(state.pool.clone());
    let new_mem = masday_db::schema::NewMemory {
        workflow_id: payload
            .get("workflow_id")
            .and_then(|v| v.as_str())
            .map(String::from),
        task_id: None,
        memory_type: "research".to_string(),
        summary: payload
            .get("summary")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        content: payload
            .get("content")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        importance_score: payload.get("importance_score").and_then(|v| v.as_f64()),
        created_by_agent: payload
            .get("created_by_agent")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string(),
        tags: Some(vec!["research".to_string()]),
        source: None,
        embedding: None,
    };
    let mem = repo.store(&new_mem).await?;
    Ok(Json(serde_json::json!(mem)))
}

async fn recall_recent(
    State(state): State<AppState>,
    pagination: Pagination,
) -> Result<Json<Value>, ApiError> {
    let repo = MemoryRepo::new(state.pool.clone());
    let memories = repo.recall_recent(pagination.limit() as i64).await?;
    Ok(Json(serde_json::json!(memories)))
}

async fn recall_by_type(
    State(state): State<AppState>,
    Query(params): Query<RecallQuery>,
) -> Result<Json<Value>, ApiError> {
    let repo = MemoryRepo::new(state.pool.clone());
    let mem_type = params.memory_type.as_deref().unwrap_or("fact");
    let memories = repo.recall_by_type(mem_type, params.limit).await?;
    Ok(Json(serde_json::json!(memories)))
}

async fn recall_by_task(
    State(state): State<AppState>,
    Path(task_id): Path<String>,
) -> Result<Json<Value>, ApiError> {
    let repo = MemoryRepo::new(state.pool.clone());
    let memories = repo.recall_by_task(&task_id).await?;
    Ok(Json(serde_json::json!(memories)))
}

async fn get_memory(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Value>, ApiError> {
    let repo = MemoryRepo::new(state.pool.clone());
    let mem = repo.get_by_id(&id).await?;
    Ok(Json(serde_json::json!(mem)))
}

async fn delete_memory(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Value>, ApiError> {
    let repo = MemoryRepo::new(state.pool.clone());
    repo.delete(&id).await?;
    Ok(Json(serde_json::json!({"deleted": id})))
}

/// PATCH /memories/{id} — update a memory
async fn update_memory(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(payload): Json<Value>,
) -> Result<Json<Value>, ApiError> {
    let repo = MemoryRepo::new(state.pool.clone());
    // Verify memory exists first
    let _mem = repo.get_by_id(&id).await?;
    // Update fields — content and importance
    if let Some(content) = payload.get("content").and_then(|v| v.as_str()) {
        let client = state
            .pool
            .get()
            .await
            .map_err(|e| masday_core::AppError::database(e.to_string()))?;
        client
            .execute(
                r#"UPDATE "Memory" SET content = $1, "updatedAt" = NOW() WHERE id = $2"#,
                &[&content, &id],
            )
            .await
            .map_err(|e| masday_core::AppError::database(e.to_string()))?;
    }
    if let Some(importance) = payload.get("importance").and_then(|v| v.as_f64()) {
        let client = state
            .pool
            .get()
            .await
            .map_err(|e| masday_core::AppError::database(e.to_string()))?;
        client
            .execute(
                r#"UPDATE "Memory" SET "importanceScore" = $1, "updatedAt" = NOW() WHERE id = $2"#,
                &[&importance, &id],
            )
            .await
            .map_err(|e| masday_core::AppError::database(e.to_string()))?;
    }
    let updated = repo.get_by_id(&id).await?;
    Ok(Json(serde_json::json!(updated)))
}

/// DELETE /memories/workflow/{workflow_id} — delete all memories for a workflow
async fn delete_by_workflow(
    State(state): State<AppState>,
    Path(workflow_id): Path<String>,
) -> Result<Json<Value>, ApiError> {
    let client = state
        .pool
        .get()
        .await
        .map_err(|e| masday_core::AppError::database(e.to_string()))?;
    let result = client
        .execute(
            r#"DELETE FROM "Memory" WHERE "workflowId" = $1"#,
            &[&workflow_id],
        )
        .await
        .map_err(|e| masday_core::AppError::database(e.to_string()))?;
    Ok(Json(serde_json::json!({
        "deleted_count": result,
        "workflow_id": workflow_id
    })))
}

async fn memory_stats(State(state): State<AppState>) -> Result<Json<Value>, ApiError> {
    let repo = MemoryRepo::new(state.pool.clone());
    let stats = repo.stats().await?;
    Ok(Json(serde_json::json!({
        "total_count": stats.total_count,
        "by_type": stats.by_type
    })))
}
