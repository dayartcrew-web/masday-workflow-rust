//! Memory MCP tools - HTTP client calls to API

use crate::client;
use serde_json::Value;

pub async fn memory_store(args: Value) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    client::api_post("/api/memories", args).await
}

pub async fn memory_store_research(
    args: Value,
) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    client::api_post("/api/memories/research", args).await
}

pub async fn memory_search(args: Value) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    client::api_post("/api/memories/search", args).await
}

pub async fn memory_recall_documents(
    args: Value,
) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    // API: GET /api/memories?workflow_id=X&limit=N
    let workflow_id = args
        .get("workflow_id")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let limit = args.get("limit").and_then(|v| v.as_u64()).unwrap_or(20);
    client::api_get(&format!(
        "/api/memories?workflow_id={}&limit={}",
        workflow_id, limit
    ))
    .await
}

pub async fn memory_recall_document_by_type(
    args: Value,
) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    // API: GET /api/memories/by-type?memory_type=X&limit=N
    let source_type = args
        .get("source_type")
        .or_else(|| args.get("memory_type"))
        .and_then(|v| v.as_str())
        .unwrap_or("fact");
    let limit = args.get("limit").and_then(|v| v.as_u64()).unwrap_or(20);
    client::api_get(&format!(
        "/api/memories/by-type?memory_type={}&limit={}",
        source_type, limit
    ))
    .await
}

pub async fn memory_recall_by_task(
    args: Value,
) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    // API: GET /api/memories/by-task/{task_id}
    let task_id = args
        .get("task_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "Missing task_id".to_string())?;
    client::api_get(&format!("/api/memories/by-task/{}", task_id)).await
}

pub async fn memory_recall_recent(
    args: Value,
) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    let limit = args.get("limit").and_then(|v| v.as_u64()).unwrap_or(10);
    let memory_type = args
        .get("type")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    if memory_type.is_empty() {
        client::api_get(&format!("/api/memories/recent?limit={}", limit)).await
    } else {
        client::api_get(&format!(
            "/api/memories/recent?limit={}&memory_type={}",
            limit, memory_type
        ))
        .await
    }
}

pub async fn memory_update(args: Value) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    let memory_id = args
        .get("id")
        .or_else(|| args.get("memory_id"))
        .and_then(|v| v.as_str())
        .ok_or_else(|| "Missing id or memory_id".to_string())?;
    client::api_patch(&format!("/api/memories/{}", memory_id), args).await
}

pub async fn memory_delete(args: Value) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    let memory_id = args
        .get("id")
        .or_else(|| args.get("memory_id"))
        .and_then(|v| v.as_str())
        .ok_or_else(|| "Missing id or memory_id".to_string())?;
    client::api_delete(&format!("/api/memories/{}", memory_id)).await
}

pub async fn memory_delete_by_workflow(
    args: Value,
) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    let workflow_id = args
        .get("workflow_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "Missing workflow_id".to_string())?;
    client::api_delete(&format!("/api/memories/workflow/{}", workflow_id)).await
}

pub async fn memory_stats(_args: Value) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    client::api_get("/api/memories/stats").await
}
