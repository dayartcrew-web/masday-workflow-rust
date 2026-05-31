//! Session MCP tools - HTTP client calls to API

use crate::client;
use serde_json::Value;

pub async fn session_init_context(
    args: Value,
) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    client::api_post("/api/sessions/init", args).await
}

pub async fn session_get_state(
    args: Value,
) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    let key = args
        .get("key")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "Missing key".to_string())?;
    client::api_get(&format!("/api/sessions/{}", key)).await
}

pub async fn session_patch_state(
    args: Value,
) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    let key = args
        .get("key")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "Missing key".to_string())?;
    client::api_patch(&format!("/api/sessions/{}", key), args).await
}
