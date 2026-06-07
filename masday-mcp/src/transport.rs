//! JSON-RPC 2.0 transport over stdio

use crate::handler::{JsonRequest, McpHandler};
use crate::registry::ToolRegistry;
use std::io::{self, BufRead, BufReader, BufWriter, Write};
use tracing::debug;

/// JSON-RPC 2.0 server
pub struct JsonRpcServer {
    handler: McpHandler,
}

impl JsonRpcServer {
    /// Create a new JSON-RPC server
    pub fn new(registry: ToolRegistry) -> Self {
        Self {
            handler: McpHandler::new(registry),
        }
    }

    /// Run the server on stdin/stdout
    pub async fn run(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let stdin = io::stdin();
        let stdout = io::stdout();
        let reader = BufReader::new(stdin.lock());
        let mut writer = BufWriter::new(stdout.lock());

        tracing::info!("JSON-RPC server started on stdio");

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
            let response = self.handler.handle_request(request).await;

            // Send the response if not a notification
            if let Some(response) = response {
                let response_json = serde_json::to_string(&response)
                    .map_err(|e| format!("Failed to serialize response: {}", e))?;

                debug!("Sending response: {}", response_json);

                writeln!(writer, "{}", response_json)
                    .map_err(|e| format!("Failed to write response: {}", e))?;
                writer
                    .flush()
                    .map_err(|e| format!("Failed to flush response: {}", e))?;
            }
        }

        tracing::info!("JSON-RPC server ended");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::handler::JsonResponse;
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
}
