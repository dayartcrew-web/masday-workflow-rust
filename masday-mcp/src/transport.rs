//! JSON-RPC 2.0 transport over stdio

use crate::registry::ToolRegistry;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::io::{self, BufRead, BufReader, BufWriter, Write};
use tracing::{debug, error, info, warn};

/// JSON-RPC 2.0 request
#[derive(Debug, Deserialize)]
struct JsonRequest {
    #[allow(dead_code)]
    jsonrpc: Option<String>,
    id: Option<Value>,
    method: String,
    params: Option<Value>,
}

/// JSON-RPC 2.0 response
#[derive(Debug, Serialize)]
struct JsonResponse {
    jsonrpc: &'static str,
    id: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<JsonError>,
}

/// JSON-RPC 2.0 error
#[derive(Debug, Serialize)]
struct JsonError {
    code: i64,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<Value>,
}

/// MCP tool call response content
#[derive(Debug, Serialize)]
struct ToolContent {
    #[serde(rename = "type")]
    content_type: String,
    text: String,
}

/// MCP tool call result
#[derive(Debug, Serialize)]
struct ToolResult {
    content: Vec<ToolContent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "isError")]
    #[allow(non_snake_case)]
    is_error: Option<bool>,
}

/// Error codes
#[allow(dead_code)]
const ERROR_INVALID_REQUEST: i64 = -32600;
const ERROR_METHOD_NOT_FOUND: i64 = -32601;
const ERROR_INVALID_PARAMS: i64 = -32602;
#[allow(dead_code)]
const ERROR_INTERNAL_ERROR: i64 = -32603;

/// JSON-RPC 2.0 server
pub struct JsonRpcServer {
    registry: ToolRegistry,
    initialized: bool,
}

impl JsonRpcServer {
    /// Create a new JSON-RPC server
    pub fn new(registry: ToolRegistry) -> Self {
        Self {
            registry,
            initialized: false,
        }
    }

    /// Run the server on stdin/stdout
    pub async fn run(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let stdin = io::stdin();
        let stdout = io::stdout();
        let reader = BufReader::new(stdin.lock());
        let mut writer = BufWriter::new(stdout.lock());

        info!("JSON-RPC server started on stdio");

        for line in reader.lines() {
            let line = line.map_err(|e| format!("Failed to read line: {}", e))?;

            // Skip empty lines
            if line.trim().is_empty() {
                continue;
            }

            debug!("Received request: {}", line);

            // Parse the request
            let request: JsonRequest = serde_json::from_str(&line)
                .map_err(|e| format!("Failed to parse JSON-RPC request: {}", e))?;

            // Handle the request
            let response = self.handle_request(request).await;

            // Send the response
            let response_json = serde_json::to_string(&response)
                .map_err(|e| format!("Failed to serialize response: {}", e))?;

            debug!("Sending response: {}", response_json);

            writeln!(writer, "{}", response_json)
                .map_err(|e| format!("Failed to write response: {}", e))?;
            writer
                .flush()
                .map_err(|e| format!("Failed to flush response: {}", e))?;
        }

        info!("JSON-RPC server ended");
        Ok(())
    }

    /// Handle a single JSON-RPC request
    async fn handle_request(&mut self, request: JsonRequest) -> JsonResponse {
        let id = request.id;

        // Handle initialize
        if request.method == "initialize" {
            return self.handle_initialize(id).await;
        }

        // Handle initialized notification
        if request.method == "notifications/initialized" {
            self.initialized = true;
            info!("MCP initialized");
            return JsonResponse {
                jsonrpc: "2.0",
                id: None, // Notification - no response
                result: None,
                error: None,
            };
        }

        // Handle ping
        if request.method == "ping" {
            return JsonResponse {
                jsonrpc: "2.0",
                id,
                result: Some(serde_json::json!({})),
                error: None,
            };
        }

        // Handle tools/list
        if request.method == "tools/list" {
            return self.handle_list_tools(id).await;
        }

        // Handle tools/call
        if request.method == "tools/call" {
            return self.handle_call_tool(id, request.params).await;
        }

        // Method not found
        warn!("Unknown method: {}", request.method);
        JsonResponse {
            jsonrpc: "2.0",
            id,
            result: None,
            error: Some(JsonError {
                code: ERROR_METHOD_NOT_FOUND,
                message: format!("Method not found: {}", request.method),
                data: None,
            }),
        }
    }

    /// Handle initialize request
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

    /// Handle tools/list request
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

    /// Handle tools/call request
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

        // Extract tool name and arguments
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

        // Call the tool
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
    use serde_json::Value;

    #[test]
    fn test_jsonrpc_request_deserialize() {
        let json = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}"#;
        let request: JsonRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.method, "initialize");
        assert_eq!(request.id, Some(Value::Number(1.into())));
    }

    #[test]
    fn test_jsonrpc_response_serialize() {
        let response = JsonResponse {
            jsonrpc: "2.0",
            id: Some(Value::Number(1.into())),
            result: Some(serde_json::json!({"test": "value"})),
            error: None,
        };
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("jsonrpc"));
        assert!(json.contains("result"));
    }

    #[test]
    fn test_tool_content_serialize() {
        let content = ToolContent {
            content_type: "text".to_string(),
            text: "Hello".to_string(),
        };
        let json = serde_json::to_string(&content).unwrap();
        assert!(json.contains("text"));
        assert!(json.contains("Hello"));
    }
}
