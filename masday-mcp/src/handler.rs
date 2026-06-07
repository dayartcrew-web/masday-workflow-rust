//! Transport-agnostic MCP JSON-RPC handler
//!
//! This module extracts all JSON-RPC protocol handling from transport.rs
//! into a Clone + Send + Sync struct that can be used by both stdio and HTTP transports.

use crate::registry::ToolRegistry;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tracing::{debug, error, info, warn};

/// JSON-RPC 2.0 request
#[derive(Debug, Deserialize)]
pub struct JsonRequest {
    #[allow(dead_code)]
    pub jsonrpc: Option<String>,
    pub id: Option<Value>,
    pub method: String,
    pub params: Option<Value>,
}

/// JSON-RPC 2.0 response
#[derive(Debug, Serialize)]
pub struct JsonResponse {
    pub jsonrpc: &'static str,
    pub id: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonError>,
}

/// JSON-RPC 2.0 error
#[derive(Debug, Serialize)]
pub struct JsonError {
    pub code: i64,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

/// MCP tool call response content
#[derive(Debug, Serialize)]
pub struct ToolContent {
    #[serde(rename = "type")]
    pub content_type: String,
    pub text: String,
}

/// MCP tool call result
#[derive(Debug, Serialize)]
pub struct ToolResult {
    pub content: Vec<ToolContent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "isError")]
    pub is_error: Option<bool>,
}

/// Error codes
pub const ERROR_INVALID_REQUEST: i64 = -32600;
pub const ERROR_METHOD_NOT_FOUND: i64 = -32601;
pub const ERROR_INVALID_PARAMS: i64 = -32602;
pub const ERROR_INTERNAL_ERROR: i64 = -32603;

/// Transport-agnostic MCP JSON-RPC handler
#[derive(Clone)]
pub struct McpHandler {
    registry: Arc<ToolRegistry>,
    initialized: Arc<AtomicBool>,
}

impl McpHandler {
    pub fn new(registry: ToolRegistry) -> Self {
        Self {
            registry: Arc::new(registry),
            initialized: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Handle a parsed JSON-RPC request. Returns None for notifications.
    pub async fn handle_request(&self, request: JsonRequest) -> Option<JsonResponse> {
        let id = request.id;

        if request.method == "initialize" {
            return Some(self.handle_initialize(id).await);
        }

        if request.method == "notifications/initialized" {
            self.initialized.store(true, Ordering::SeqCst);
            info!("MCP initialized");
            return None; // Notification - no response
        }

        if request.method == "ping" {
            return Some(JsonResponse {
                jsonrpc: "2.0",
                id,
                result: Some(serde_json::json!({})),
                error: None,
            });
        }

        if request.method == "tools/list" {
            return Some(self.handle_list_tools(id).await);
        }

        if request.method == "tools/call" {
            return Some(self.handle_call_tool(id, request.params).await);
        }

        warn!("Unknown method: {}", request.method);
        Some(JsonResponse {
            jsonrpc: "2.0",
            id,
            result: None,
            error: Some(JsonError {
                code: ERROR_METHOD_NOT_FOUND,
                message: format!("Method not found: {}", request.method),
                data: None,
            }),
        })
    }

    /// Parse raw JSON string, handle request, serialize response.
    /// Returns None for notifications (no response needed).
    pub async fn handle_raw_json(&self, json: &str) -> Option<String> {
        let request: JsonRequest = match serde_json::from_str(json) {
            Ok(r) => r,
            Err(e) => {
                let response = JsonResponse {
                    jsonrpc: "2.0",
                    id: None,
                    result: None,
                    error: Some(JsonError {
                        code: ERROR_INVALID_REQUEST,
                        message: format!("Failed to parse JSON-RPC request: {}", e),
                        data: None,
                    }),
                };
                return Some(serde_json::to_string(&response).unwrap_or_default());
            }
        };

        match self.handle_request(request).await {
            Some(response) => serde_json::to_string(&response).ok(),
            None => None,
        }
    }

    async fn handle_initialize(&self, id: Option<Value>) -> JsonResponse {
        info!("Initialize request");
        let result = serde_json::json!({
            "protocolVersion": "2024-11-05",
            "capabilities": {
                "tools": {
                    "listChanged": false
                }
            },
            "serverInfo": {
                "name": "masday-mcp",
                "version": "0.1.0"
            }
        });

        JsonResponse {
            jsonrpc: "2.0",
            id,
            result: Some(result),
            error: None,
        }
    }

    async fn handle_list_tools(&self, id: Option<Value>) -> JsonResponse {
        let tools = self.registry.list_tools();
        let tool_definitions: Vec<Value> = tools
            .into_iter()
            .map(|t| {
                serde_json::json!({
                    "name": t.name,
                    "description": t.description,
                    "inputSchema": t.input_schema
                })
            })
            .collect();

        let result = serde_json::json!({ "tools": tool_definitions });
        debug!("Listed {} tools", tool_definitions.len());

        JsonResponse {
            jsonrpc: "2.0",
            id,
            result: Some(result),
            error: None,
        }
    }

    async fn handle_call_tool(&self, id: Option<Value>, params: Option<Value>) -> JsonResponse {
        let params = match params {
            Some(p) => p,
            None => {
                return JsonResponse {
                    jsonrpc: "2.0",
                    id,
                    result: None,
                    error: Some(JsonError {
                        code: ERROR_INVALID_PARAMS,
                        message: "Missing params".to_string(),
                        data: None,
                    }),
                }
            }
        };

        let tool_name = match params.get("name").and_then(|v| v.as_str()) {
            Some(name) => name,
            None => {
                return JsonResponse {
                    jsonrpc: "2.0",
                    id,
                    result: None,
                    error: Some(JsonError {
                        code: ERROR_INVALID_PARAMS,
                        message: "Missing tool name".to_string(),
                        data: None,
                    }),
                }
            }
        };

        let args = params
            .get("arguments")
            .cloned()
            .unwrap_or_else(|| serde_json::json!({}));
        debug!("Calling tool: {} with args: {}", tool_name, args);

        match self.registry.call_tool(tool_name, args).await {
            Ok(result) => {
                let tool_result = ToolResult {
                    content: vec![ToolContent {
                        content_type: "text".to_string(),
                        text: serde_json::to_string(&result)
                            .unwrap_or_else(|_| "Failed to serialize result".to_string()),
                    }],
                    is_error: None,
                };

                JsonResponse {
                    jsonrpc: "2.0",
                    id,
                    result: Some(serde_json::to_value(tool_result).unwrap()),
                    error: None,
                }
            }
            Err(e) => {
                error!("Tool call failed: {}", e);
                let tool_result = ToolResult {
                    content: vec![ToolContent {
                        content_type: "text".to_string(),
                        text: format!("Error: {}", e),
                    }],
                    is_error: Some(true),
                };

                JsonResponse {
                    jsonrpc: "2.0",
                    id,
                    result: Some(serde_json::to_value(tool_result).unwrap()),
                    error: None,
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry::ToolDefinition;

    fn test_registry() -> ToolRegistry {
        let mut r = ToolRegistry::new();
        r.register(
            ToolDefinition {
                name: "test_tool".to_string(),
                description: "A test tool".to_string(),
                input_schema: serde_json::json!({"type":"object","properties":{"msg":{"type":"string"}},"required":["msg"]}),
            },
            Box::new(|args: Value| {
                Box::pin(async move {
                    let msg = args.get("msg").and_then(|v| v.as_str()).unwrap_or("no msg");
                    Ok(serde_json::json!({"echo": msg}))
                })
            }),
        );
        r
    }

    #[tokio::test]
    async fn test_handle_initialize() {
        let handler = McpHandler::new(test_registry());
        let req = JsonRequest {
            jsonrpc: Some("2.0".to_string()),
            id: Some(serde_json::json!(1)),
            method: "initialize".to_string(),
            params: None,
        };
        let resp = handler.handle_request(req).await.unwrap();
        assert_eq!(resp.jsonrpc, "2.0");
        assert!(resp.result.is_some());
        assert!(resp.error.is_none());
        let result = resp.result.unwrap();
        assert_eq!(result["protocolVersion"], "2024-11-05");
    }

    #[tokio::test]
    async fn test_handle_ping() {
        let handler = McpHandler::new(test_registry());
        let req = JsonRequest {
            jsonrpc: Some("2.0".to_string()),
            id: Some(serde_json::json!(2)),
            method: "ping".to_string(),
            params: None,
        };
        let resp = handler.handle_request(req).await.unwrap();
        assert!(resp.result.is_some());
    }

    #[tokio::test]
    async fn test_handle_notification_returns_none() {
        let handler = McpHandler::new(test_registry());
        let req = JsonRequest {
            jsonrpc: Some("2.0".to_string()),
            id: None,
            method: "notifications/initialized".to_string(),
            params: None,
        };
        let resp = handler.handle_request(req).await;
        assert!(resp.is_none());
        assert!(handler
            .initialized
            .load(std::sync::atomic::Ordering::SeqCst));
    }

    #[tokio::test]
    async fn test_handle_tools_list() {
        let handler = McpHandler::new(test_registry());
        let req = JsonRequest {
            jsonrpc: Some("2.0".to_string()),
            id: Some(serde_json::json!(3)),
            method: "tools/list".to_string(),
            params: None,
        };
        let resp = handler.handle_request(req).await.unwrap();
        let result = resp.result.unwrap();
        let tools = result["tools"].as_array().unwrap();
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0]["name"], "test_tool");
    }

    #[tokio::test]
    async fn test_handle_tools_call() {
        let handler = McpHandler::new(test_registry());
        let req = JsonRequest {
            jsonrpc: Some("2.0".to_string()),
            id: Some(serde_json::json!(4)),
            method: "tools/call".to_string(),
            params: Some(serde_json::json!({"name": "test_tool", "arguments": {"msg": "hello"}})),
        };
        let resp = handler.handle_request(req).await.unwrap();
        let result = resp.result.unwrap();
        let content = &result["content"][0];
        let text = content["text"].as_str().unwrap();
        assert!(text.contains("hello"));
    }

    #[tokio::test]
    async fn test_handle_raw_json() {
        let handler = McpHandler::new(test_registry());
        let json = r#"{"jsonrpc":"2.0","id":1,"method":"ping"}"#;
        let result = handler.handle_raw_json(json).await.unwrap();
        assert!(result.contains("2.0"));
    }

    #[tokio::test]
    async fn test_unknown_method() {
        let handler = McpHandler::new(test_registry());
        let req = JsonRequest {
            jsonrpc: Some("2.0".to_string()),
            id: Some(serde_json::json!(99)),
            method: "nonexistent".to_string(),
            params: None,
        };
        let resp = handler.handle_request(req).await.unwrap();
        assert!(resp.error.is_some());
        assert_eq!(resp.error.unwrap().code, ERROR_METHOD_NOT_FOUND);
    }
}
