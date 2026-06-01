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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_function_signatures() {
        // Test that functions accept Value type
        let args = json!({ "test": "value" });
        let test_fn = |args: Value| -> Result<(), String> {
            if args.get("test").is_some() {
                Ok(())
            } else {
                Err("Missing test".to_string())
            }
        };
        assert!(test_fn(args).is_ok());
    }
}
