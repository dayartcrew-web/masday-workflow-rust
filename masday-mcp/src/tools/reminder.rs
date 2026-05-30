//! Reminder MCP tools - HTTP client calls to API

use reqwest::Client;

/// Check stale workflows via HTTP
pub async fn reminder_check_stale() -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let client = Client::new();
    let api_url = std::env::var("MASDAY_API_URL")
        .unwrap_or_else(|_| "http://localhost:3001".to_string());
    let api_key = std::env::var("MASDAY_API_KEY")
        .unwrap_or_else(|_| "dev-key".to_string());

    let response = client
        .get(&format!("{}/api/reminders/stale", api_url))
        .header("Authorization", format!("Bearer {}", api_key))
        .send()
        .await?;

    let result: serde_json::Value = response.json().await?;
    Ok(result)
}

/// Check stuck tasks via HTTP
pub async fn reminder_check_stuck() -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let client = Client::new();
    let api_url = std::env::var("MASDAY_API_URL")
        .unwrap_or_else(|_| "http://localhost:3001".to_string());
    let api_key = std::env::var("MASDAY_API_KEY")
        .unwrap_or_else(|_| "dev-key".to_string());

    let response = client
        .get(&format!("{}/api/reminders/stuck", api_url))
        .header("Authorization", format!("Bearer {}", api_key))
        .send()
        .await?;

    let result: serde_json::Value = response.json().await?;
    Ok(result)
}
