//! Review MCP tools - HTTP client calls to API

use reqwest::Client;
use serde_json::Value;

/// Submit review via HTTP
pub async fn review_submit(args: Value) -> Result<Value, Box<dyn std::error::Error>> {
    let client = Client::new();
    let api_url = std::env::var("MASDAY_API_URL")
        .unwrap_or_else(|_| "http://localhost:3001".to_string());
    let api_key = std::env::var("MASDAY_API_KEY")
        .unwrap_or_else(|_| "PLACEHOLDER".to_string());

    let response = client
        .post(&format!("{}/api/reviews", api_url))
        .header("Authorization", format!("Bearer {}", api_key))
        .json(&args)
        .send()
        .await?;

    let result: Value = response.json().await?;
    Ok(result)
}

/// Get review by task via HTTP
pub async fn review_get_by_task(task_id: &str) -> Result<Value, Box<dyn std::error::Error>> {
    let client = Client::new();
    let api_url = std::env::var("MASDAY_API_URL")
        .unwrap_or_else(|_| "http://localhost:3001".to_string());
    let api_key = std::env::var("MASDAY_API_KEY")
        .unwrap_or_else(|_| "PLACEHOLDER".to_string());

    let response = client
        .get(&format!("{}/api/reviews/task/{}", api_url, task_id))
        .header("Authorization", format!("Bearer {}", api_key))
        .send()
        .await?;

    let result: Value = response.json().await?;
    Ok(result)
}
