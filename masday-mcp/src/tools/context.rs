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

#[cfg(test)]
mod tests {
    use serde_json::json;

    #[test]
    fn test_semantic_search_code_search_args() {
        let args = json!({ "query": "workflow execution" });
        let query = args.get("query").and_then(|v| v.as_str());
        assert!(query.is_some());
        assert_eq!(query.unwrap(), "workflow execution");

        let args = json!({});
        let query = args.get("query").and_then(|v| v.as_str());
        assert!(query.is_none());
    }

    #[test]
    fn test_function_url_building() {
        let workflow_id = "wf-123";
        let url = format!("/api/workflows/{}/execute", workflow_id);
        assert_eq!(url, "/api/workflows/wf-123/execute");

        let query = "test search";
        let url = format!("/api/context/search?query={}", query);
        assert_eq!(url, "/api/context/search?query=test search");
    }
}
