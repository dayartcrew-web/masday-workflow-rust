//! Memory MCP tools - HTTP client calls to API

use reqwest::Client;
use serde_json::Value;

/// Store memory via HTTP
pub async fn memory_store(args: Value) -> Result<Value, Box<dyn std::error::Error>> {
    let client = Client::new();
    let api_url = std::env::var("MASDAY_API_URL")
        .unwrap_or_else(|_| "http://localhost:3001".to_string());
    let api_key = std::env::var("MASDAY_API_KEY")
        .unwrap_or_else(|_| "PLACEHOLDER".to_string());

    let response = client
        .post(&format!("{}/api/memories", api_url))
        .header("Authorization", format!("Bearer {}", api_key))
        .json(&args)
        .send()
        .await?;

    let result: Value = response.json().await?;
    Ok(result)
}

/// Search memories via HTTP
pub async fn memory_search(query: &str) -> Result<Value, Box<dyn std::error::Error>> {
    let client = Client::new();
    let api_url = std::env::var("MASDAY_API_URL")
        .unwrap_or_else(|_| "http://localhost:3001".to_string());
    let api_key = std::env::var("MASDAY_API_KEY")
        .unwrap_or_else(|_| "PLACEHOLDER".to_string());

    let response = client
        .post(&format!("{}/api/memories/search", api_url))
        .header("Authorization", format!("Bearer {}", api_key))
        .json(&serde_json::json!({"query": query}))
        .send()
        .await?;

    let result: Value = response.json().await?;
    Ok(result)
}

/// Recall recent memories via HTTP
pub async fn memory_recall_recent(limit: usize) -> Result<Value, Box<dyn std::error::Error>> {
    let client = Client::new();
    let api_url = std::env::var("MASDAY_API_URL")
        .unwrap_or_else(|_| "http://localhost:3001".to_string());
    let api_key = std::env::var("MASDAY_API_KEY")
        .unwrap_or_else(|_| "PLACEHOLDER".to_string());

    let response = client
        .get(&format!("{}/api/memories/recent?limit={}", api_url, limit))
        .header("Authorization", format!("Bearer {}", api_key))
        .send()
        .await?;

    let result: Value = response.json().await?;
    Ok(result)
}
