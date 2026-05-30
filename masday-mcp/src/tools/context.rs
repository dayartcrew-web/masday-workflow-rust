//! Context MCP tools - HTTP client calls to API

use reqwest::Client;

/// Build context pack via HTTP
pub async fn context_build_pack(workflow_id: &str) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let client = Client::new();
    let api_url = std::env::var("MASDAY_API_URL")
        .unwrap_or_else(|_| "http://localhost:3001".to_string());
    let api_key = std::env::var("MASDAY_API_KEY")
        .unwrap_or_else(|_| "dev-key".to_string());

    let response = client
        .get(&format!("{}/api/context/pack/{}", api_url, workflow_id))
        .header("Authorization", format!("Bearer {}", api_key))
        .send()
        .await?;

    let result: serde_json::Value = response.json().await?;
    Ok(result)
}

/// Compute fingerprint via HTTP
pub async fn context_compute_fingerprint(content: &str) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let client = Client::new();
    let api_url = std::env::var("MASDAY_API_URL")
        .unwrap_or_else(|_| "http://localhost:3001".to_string());
    let api_key = std::env::var("MASDAY_API_KEY")
        .unwrap_or_else(|_| "dev-key".to_string());

    let response = client
        .post(&format!("{}/api/context/fingerprint", api_url))
        .header("Authorization", format!("Bearer {}", api_key))
        .json(&serde_json::json!({"content": content}))
        .send()
        .await?;

    let result: serde_json::Value = response.json().await?;
    Ok(result)
}
