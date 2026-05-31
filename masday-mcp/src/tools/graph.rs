//! Graph MCP tools - HTTP client calls to API

use crate::client;
use serde_json::Value;

pub async fn memory_create_entities(
    args: Value,
) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    client::api_post("/api/graph/nodes", args).await
}

pub async fn memory_search_nodes(
    args: Value,
) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    client::api_post("/api/graph/search", args).await
}
