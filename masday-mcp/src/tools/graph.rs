//! Graph MCP tools - HTTP client calls to API

use reqwest::Client;
use serde_json::Value;

/// Add graph node via HTTP
pub async fn graph_add_node(args: Value) -> Result<Value, Box<dyn std::error::Error>> {
    let client = Client::new();
    let api_url = std::env::var("MASDAY_API_URL")
        .unwrap_or_else(|_| "http://localhost:3001".to_string());
    let api_key = std::env::var("MASDAY_API_KEY")
        .unwrap_or_else(|_| "PLACEHOLDER".to_string());

    let response = client
        .post(&format!("{}/api/graph/nodes", api_url))
        .header("Authorization", format!("Bearer {}", api_key))
        .json(&args)
        .send()
        .await?;

    let result: Value = response.json().await?;
    Ok(result)
}

/// Get graph node via HTTP
pub async fn graph_get_node(node_id: &str) -> Result<Value, Box<dyn std::error::Error>> {
    let client = Client::new();
    let api_url = std::env::var("MASDAY_API_URL")
        .unwrap_or_else(|_| "http://localhost:3001".to_string());
    let api_key = std::env::var("MASDAY_API_KEY")
        .unwrap_or_else(|_| "PLACEHOLDER".to_string());

    let response = client
        .get(&format!("{}/api/graph/nodes/{}", api_url, node_id))
        .header("Authorization", format!("Bearer {}", api_key))
        .send()
        .await?;

    let result: Value = response.json().await?;
    Ok(result)
}

/// Add graph edge via HTTP
pub async fn graph_add_edge(args: Value) -> Result<Value, Box<dyn std::error::Error>> {
    let client = Client::new();
    let api_url = std::env::var("MASDAY_API_URL")
        .unwrap_or_else(|_| "http://localhost:3001".to_string());
    let api_key = std::env::var("MASDAY_API_KEY")
        .unwrap_or_else(|_| "PLACEHOLDER".to_string());

    let response = client
        .post(&format!("{}/api/graph/edges", api_url))
        .header("Authorization", format!("Bearer {}", api_key))
        .json(&args)
        .send()
        .await?;

    let result: Value = response.json().await?;
    Ok(result)
}
