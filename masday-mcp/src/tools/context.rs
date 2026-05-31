//! Context (Semantic Search) MCP tools - HTTP client calls to API

use crate::client;
use serde_json::Value;

pub async fn semantic_search_search_hybrid_context_pack(
    args: Value,
) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    client::api_post("/api/context/hybrid-search", args).await
}

pub async fn semantic_search_search_context_fingerprint(
    args: Value,
) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    client::api_post("/api/context/fingerprint-search", args).await
}

pub async fn semantic_search_code_search(
    args: Value,
) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    let query = args
        .get("query")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "Missing query".to_string())?;
    client::api_get(&format!("/api/context/search?query={}", query)).await
}

pub async fn semantic_search_make_fingerprint(
    args: Value,
) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    client::api_post("/api/context/fingerprint", args).await
}
