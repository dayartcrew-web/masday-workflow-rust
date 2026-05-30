//! Capability MCP tools - HTTP client calls to API

use reqwest::Client;

/// List agents via HTTP
pub async fn capability_list_agents() -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let client = Client::new();
    let api_url = std::env::var("MASDAY_API_URL")
        .unwrap_or_else(|_| "http://localhost:3001".to_string());
    let api_key = std::env::var("MASDAY_API_KEY")
        .unwrap_or_else(|_| "dev-key".to_string());

    let response = client
        .get(&format!("{}/api/capabilities/agents", api_url))
        .header("Authorization", format!("Bearer {}", api_key))
        .send()
        .await?;

    let result: serde_json::Value = response.json().await?;
    Ok(result)
}

/// List skills via HTTP
pub async fn capability_list_skills() -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let client = Client::new();
    let api_url = std::env::var("MASDAY_API_URL")
        .unwrap_or_else(|_| "http://localhost:3001".to_string());
    let api_key = std::env::var("MASDAY_API_KEY")
        .unwrap_or_else(|_| "dev-key".to_string());

    let response = client
        .get(&format!("{}/api/capabilities/skills", api_url))
        .header("Authorization", format!("Bearer {}", api_key))
        .send()
        .await?;

    let result: serde_json::Value = response.json().await?;
    Ok(result)
}

/// Match agent via HTTP
pub async fn capability_match_agent(task: &str) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let client = Client::new();
    let api_url = std::env::var("MASDAY_API_URL")
        .unwrap_or_else(|_| "http://localhost:3001".to_string());
    let api_key = std::env::var("MASDAY_API_KEY")
        .unwrap_or_else(|_| "dev-key".to_string());

    let response = client
        .get(&format!("{}/api/capabilities/match?task={}", api_url, task))
        .header("Authorization", format!("Bearer {}", api_key))
        .send()
        .await?;

    let result: serde_json::Value = response.json().await?;
    Ok(result)
}
