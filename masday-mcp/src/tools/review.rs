//! Review MCP tools - HTTP client calls to API

use crate::client;
use serde_json::Value;

pub async fn review_submit(args: Value) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    client::api_post("/api/reviews", args).await
}

pub async fn review_get_latest(
    args: Value,
) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    let task_id = args
        .get("task_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "Missing task_id".to_string())?;
    client::api_get(&format!("/api/reviews/task/{}", task_id)).await
}
