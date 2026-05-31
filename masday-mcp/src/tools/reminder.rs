//! Reminder MCP tools - HTTP client calls to API

use crate::client;
use serde_json::Value;

pub async fn reminder_check(
    _args: Value,
) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    let stale = client::api_get("/api/reminders/stale").await?;
    let stuck = client::api_get("/api/reminders/stuck").await?;
    Ok(serde_json::json!({ "stale": stale, "stuck": stuck }))
}

pub async fn reminder_list(
    _args: Value,
) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    client::api_get("/api/reminders/list").await
}

pub async fn reminder_acknowledge(
    args: Value,
) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    let reminder_id = args
        .get("reminder_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "Missing reminder_id".to_string())?;
    client::api_post(&format!("/api/reminders/{}/acknowledge", reminder_id), args).await
}
