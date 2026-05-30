//! Workflow MCP tools - HTTP client calls to API

use reqwest::Client;
use serde_json::Value;

/// Create workflow via HTTP
pub async fn workflow_create(args: Value) -> Result<Value, Box<dyn std::error::Error>> {
    let client = Client::new();
    let api_url = std::env::var("MASDAY_API_URL")
        .unwrap_or_else(|_| "http://localhost:3001".to_string());
    let api_key = std::env::var("MASDAY_API_KEY")
        .unwrap_or_else(|_| "dev-key".to_string());

    let response = client
        .post(&format!("{}/api/workflows", api_url))
        .header("Authorization", format!("Bearer {}", api_key))
        .json(&args)
        .send()
        .await?;

    let result: Value = response.json().await?;
    Ok(result)
}

/// Get workflow status via HTTP
pub async fn workflow_get_status(workflow_id: &str) -> Result<Value, Box<dyn std::error::Error>> {
    let client = Client::new();
    let api_url = std::env::var("MASDAY_API_URL")
        .unwrap_or_else(|_| "http://localhost:3001".to_string());
    let api_key = std::env::var("MASDAY_API_KEY")
        .unwrap_or_else(|_| "dev-key".to_string());

    let response = client
        .get(&format!("{}/api/workflows/{}/status", api_url, workflow_id))
        .header("Authorization", format!("Bearer {}", api_key))
        .send()
        .await?;

    let result: Value = response.json().await?;
    Ok(result)
}

/// Execute workflow via HTTP
pub async fn workflow_execute(workflow_id: &str) -> Result<Value, Box<dyn std::error::Error>> {
    let client = Client::new();
    let api_url = std::env::var("MASDAY_API_URL")
        .unwrap_or_else(|_| "http://localhost:3001".to_string());
    let api_key = std::env::var("MASDAY_API_KEY")
        .unwrap_or_else(|_| "dev-key".to_string());

    let response = client
        .post(&format!("{}/api/workflows/{}/execute", api_url, workflow_id))
        .header("Authorization", format!("Bearer {}", api_key))
        .send()
        .await?;

    let result: Value = response.json().await?;
    Ok(result)
}
