//! Tool registry - stores tool definitions and dispatches calls

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;

/// Tool definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub input_schema: Value,
}

/// Tool handler function type (boxed async function)
pub type ToolHandler = Box<
    dyn Fn(
            Value,
        ) -> Pin<
            Box<
                dyn Future<Output = Result<Value, Box<dyn std::error::Error + Send + Sync>>> + Send,
            >,
        > + Send
        + Sync,
>;

/// Tool registry
pub struct ToolRegistry {
    tools: HashMap<String, (ToolDefinition, ToolHandler)>,
}

impl ToolRegistry {
    /// Create a new empty registry
    pub fn new() -> Self {
        Self {
            tools: HashMap::new(),
        }
    }

    /// Register a tool
    pub fn register(&mut self, definition: ToolDefinition, handler: ToolHandler) {
        self.tools
            .insert(definition.name.clone(), (definition, handler));
    }

    /// List all registered tools (sorted by name for deterministic prompt cache)
    pub fn list_tools(&self) -> Vec<ToolDefinition> {
        let mut tools: Vec<_> = self.tools.values().map(|(def, _)| def.clone()).collect();
        tools.sort_by(|a, b| a.name.cmp(&b.name));
        tools
    }

    /// Call a tool by name
    pub async fn call_tool(
        &self,
        name: &str,
        args: Value,
    ) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
        match self.tools.get(name) {
            Some((_, handler)) => handler(args).await,
            None => Err(format!("Tool not found: {}", name).into()),
        }
    }

    /// Check if a tool exists
    pub fn has_tool(&self, name: &str) -> bool {
        self.tools.contains_key(name)
    }

    /// Get the number of registered tools
    pub fn count(&self) -> usize {
        self.tools.len()
    }
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Macro to wrap async functions into ToolHandler
#[macro_export]
macro_rules! async_tool_handler {
    ($handler:expr) => {{
        Box::new(move |args: serde_json::Value| {
            Box::pin(async move { $handler(args).await })
                as std::pin::Pin<
                    Box<
                        dyn std::future::Future<
                                Output = Result<
                                    serde_json::Value,
                                    Box<dyn std::error::Error + Send + Sync>,
                                >,
                            > + Send,
                    >,
                >
        })
    }};
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    async fn dummy_handler(args: Value) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
        Ok(args)
    }

    #[test]
    fn test_registry_new() {
        let registry = ToolRegistry::new();
        assert_eq!(registry.count(), 0);
    }

    #[test]
    fn test_registry_register() {
        let mut registry = ToolRegistry::new();
        let definition = ToolDefinition {
            name: "test_tool".to_string(),
            description: "A test tool".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {}
            }),
        };

        let handler = async_tool_handler!(dummy_handler);
        registry.register(definition, handler);
        assert_eq!(registry.count(), 1);
        assert!(registry.has_tool("test_tool"));
    }

    #[test]
    fn test_registry_list_tools() {
        let mut registry = ToolRegistry::new();
        let definition = ToolDefinition {
            name: "test_tool".to_string(),
            description: "A test tool".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {}
            }),
        };

        let handler = async_tool_handler!(dummy_handler);
        registry.register(definition, handler);
        let tools = registry.list_tools();
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0].name, "test_tool");
    }

    #[tokio::test]
    async fn test_registry_call_tool() {
        let mut registry = ToolRegistry::new();
        let definition = ToolDefinition {
            name: "test_tool".to_string(),
            description: "A test tool".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {}
            }),
        };

        let handler = async_tool_handler!(dummy_handler);
        registry.register(definition, handler);
        let args = serde_json::json!({"test": "value"});
        let result = registry.call_tool("test_tool", args.clone()).await.unwrap();
        assert_eq!(result, args);
    }

    #[tokio::test]
    async fn test_registry_call_tool_not_found() {
        let registry = ToolRegistry::new();
        let result = registry
            .call_tool("nonexistent", serde_json::json!({}))
            .await;
        assert!(result.is_err());
    }
}
