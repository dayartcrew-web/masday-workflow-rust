//! Policy MCP tools - HTTP client calls to API

use reqwest::Client;
use serde_json::Value;

/// Validate completion via HTTP
pub async fn policy_validate_completion(workflow_id: &str) -> Result<Value, Box<dyn std::error::Error>> {
    let client = Client::new();
    let api_url = std::env::var("MASDAY_API_URL")
        .unwrap_or_else(|_| "http://localhost:3001".to_string());
    let api_key = std::env::var("MASDAY_API_KEY")
        .unwrap_or_else(|_| "PLACEHOLDER".to_string());

    let response = client
        .post(&format!("{}/api/policy/validate", api_url))
        .header("Authorization", format!("Bearer {}", api_key))
        .json(&serde_json::json!({"workflow_id": workflow_id}))
        .send()
        .await?;

    let result: Value = response.json().await?;
    Ok(result)
}

/// Detect drift via HTTP
pub async fn policy_detect_drift(workflow_id: &str) -> Result<Value, Box<dyn std::error::Error>> {
    let client = Client::new();
    let api_url = std::env::var("MASDAY_API_URL")
        .unwrap_or_else(|_| "http://localhost:3001".to_string());
    let api_key = std::env::var("MASDAY_API_KEY")
        .unwrap_or_else(|_| "PLACEHOLDER".to_string());

    let response = client
        .get(&format!("{}/api/policy/drift/{}", api_url, workflow_id))
        .header("Authorization", format!("Bearer {}", api_key))
        .send()
        .await?;

    let result: Value = response.json().await?;
    Ok(result)
}
