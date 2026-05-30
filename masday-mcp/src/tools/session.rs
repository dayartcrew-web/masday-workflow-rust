//! Session MCP tools - HTTP client calls to API

use reqwest::Client;
use serde_json::Value;

/// Get session via HTTP
pub async fn session_get(session_id: &str) -> Result<Value, Box<dyn std::error::Error>> {
    let client = Client::new();
    let api_url = std::env::var("MASDAY_API_URL")
        .unwrap_or_else(|_| "http://localhost:3001".to_string());
    let api_key = std::env::var("MASDAY_API_KEY")
        .unwrap_or_else(|_| "dev-key".to_string());

    let response = client
        .get(&format!("{}/api/sessions/{}", api_url, session_id))
        .header("Authorization", format!("Bearer {}", api_key))
        .send()
        .await?;

    let result: Value = response.json().await?;
    Ok(result)
}

/// Patch session state via HTTP
pub async fn session_patch(session_id: &str, state: Value) -> Result<Value, Box<dyn std::error::Error>> {
    let client = Client::new();
    let api_url = std::env::var("MASDAY_API_URL")
        .unwrap_or_else(|_| "http://localhost:3001".to_string());
    let api_key = std::env::var("MASDAY_API_KEY")
        .unwrap_or_else(|_| "dev-key".to_string());

    let response = client
        .patch(&format!("{}/api/sessions/{}", api_url, session_id))
        .header("Authorization", format!("Bearer {}", api_key))
        .json(&state)
        .send()
        .await?;

    let result: Value = response.json().await?;
    Ok(result)
}
