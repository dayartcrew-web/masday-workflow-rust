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
    client::api_post("/api/memories/recall", args).await
}

pub async fn memory_recall_document_by_type(
    args: Value,
) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    let memory_type = args
        .get("memory_type")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "Missing memory_type".to_string())?;
    client::api_post(&format!("/api/memories/by-type/{}", memory_type), args).await
}

pub async fn memory_recall_by_task(
    args: Value,
) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    client::api_post("/api/memories/by-task", args).await
}

pub async fn memory_recall_recent(
    args: Value,
) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    let limit = args.get("limit").and_then(|v| v.as_u64()).unwrap_or(10);
    client::api_get(&format!("/api/memories/recent?limit={}", limit)).await
}

pub async fn memory_update(args: Value) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    let memory_id = args
        .get("memory_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "Missing memory_id".to_string())?;
    client::api_patch(&format!("/api/memories/{}", memory_id), args).await
}

pub async fn memory_delete(args: Value) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    let memory_id = args
        .get("memory_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "Missing memory_id".to_string())?;
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
