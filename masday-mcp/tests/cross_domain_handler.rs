//! Cross-domain handler tests
//!
//! Tests multi-tool workflows through the McpHandler directly.
//! No infrastructure needed — these are unit tests that verify tool routing.

use masday_mcp::{build_registry, JsonRequest, McpHandler, ToolDefinition, ToolRegistry};
use serde_json::Value;
use std::sync::Arc;

/// Helper: Create a mock tool that returns a canned response.
/// Uses Arc<Value> so the closure implements Fn (not just FnOnce).
fn mock_workflow_tool(
    name: &'static str,
    response: Value,
) -> (ToolDefinition, masday_mcp::ToolHandler) {
    let definition = ToolDefinition {
        name: name.to_string(),
        description: format!("Mock {} tool", name),
        input_schema: serde_json::json!({"type": "object", "properties": {}, "required": []}),
    };
    let response = Arc::new(response);
    let handler: masday_mcp::ToolHandler = Box::new(move |_args: Value| {
        let r = Arc::clone(&response);
        Box::pin(async move { Ok((*r).clone()) })
    });
    (definition, handler)
}

/// Test 1: Workflow lifecycle via handler
#[tokio::test]
async fn test_workflow_lifecycle_via_handler() {
    // Create registry with mock workflow tools
    let mut registry = ToolRegistry::new();

    // Register mock workflow tools that return canned responses
    let (create_def, create_handler) = mock_workflow_tool(
        "workflow_create",
        serde_json::json!({
            "id": "wf-123",
            "name": "test-workflow",
            "status": "INIT"
        }),
    );
    registry.register(create_def, create_handler);

    let (add_task_def, add_task_handler) = mock_workflow_tool(
        "workflow_addTask",
        serde_json::json!({
            "id": "task-456",
            "name": "test-task",
            "status": "PENDING"
        }),
    );
    registry.register(add_task_def, add_task_handler);

    let (complete_task_def, complete_task_handler) = mock_workflow_tool(
        "workflow_completeTask",
        serde_json::json!({
            "id": "task-456",
            "status": "DONE",
            "result": "Task completed successfully"
        }),
    );
    registry.register(complete_task_def, complete_task_handler);

    let handler = McpHandler::new(registry);

    // Step 1: Initialize
    let init_req = JsonRequest {
        jsonrpc: Some("2.0".to_string()),
        id: Some(serde_json::json!(1)),
        method: "initialize".to_string(),
        params: None,
    };
    let init_resp = handler.handle_request(init_req).await.unwrap();
    assert!(init_resp.result.is_some());
    assert!(init_resp.error.is_none());

    // Step 2: tools/list to verify workflow_create exists
    let list_req = JsonRequest {
        jsonrpc: Some("2.0".to_string()),
        id: Some(serde_json::json!(2)),
        method: "tools/list".to_string(),
        params: None,
    };
    let list_resp = handler.handle_request(list_req).await.unwrap();
    let list_result = list_resp.result.unwrap();
    let tools = list_result["tools"].as_array().unwrap();

    // Verify all three mock tools are registered
    let tool_names: Vec<&str> = tools.iter().filter_map(|t| t["name"].as_str()).collect();
    assert!(tool_names.contains(&"workflow_create"));
    assert!(tool_names.contains(&"workflow_addTask"));
    assert!(tool_names.contains(&"workflow_completeTask"));

    // Step 3: Call workflow_create
    let create_req = JsonRequest {
        jsonrpc: Some("2.0".to_string()),
        id: Some(serde_json::json!(3)),
        method: "tools/call".to_string(),
        params: Some(serde_json::json!({
            "name": "workflow_create",
            "arguments": {"name": "test-workflow"}
        })),
    };
    let create_resp = handler.handle_request(create_req).await.unwrap();
    let create_result = create_resp.result.unwrap();
    assert_eq!(
        create_result["content"][0]["text"],
        "{\"id\":\"wf-123\",\"name\":\"test-workflow\",\"status\":\"INIT\"}"
    );

    // Step 4: Call workflow_addTask
    let add_task_req = JsonRequest {
        jsonrpc: Some("2.0".to_string()),
        id: Some(serde_json::json!(4)),
        method: "tools/call".to_string(),
        params: Some(serde_json::json!({
            "name": "workflow_addTask",
            "arguments": {"workflow_id": "wf-123", "name": "test-task"}
        })),
    };
    let add_task_resp = handler.handle_request(add_task_req).await.unwrap();
    let add_task_result = add_task_resp.result.unwrap();
    assert_eq!(
        add_task_result["content"][0]["text"],
        "{\"id\":\"task-456\",\"name\":\"test-task\",\"status\":\"PENDING\"}"
    );

    // Step 5: Call workflow_completeTask
    let complete_task_req = JsonRequest {
        jsonrpc: Some("2.0".to_string()),
        id: Some(serde_json::json!(5)),
        method: "tools/call".to_string(),
        params: Some(serde_json::json!({
            "name": "workflow_completeTask",
            "arguments": {"workflow_id": "wf-123", "task_id": "task-456"}
        })),
    };
    let complete_task_resp = handler.handle_request(complete_task_req).await.unwrap();
    let complete_task_result = complete_task_resp.result.unwrap();
    // Check that the response contains the expected fields (JSON field order may vary)
    let complete_text = complete_task_result["content"][0]["text"].as_str().unwrap();
    assert!(complete_text.contains("\"id\":\"task-456\""));
    assert!(complete_text.contains("\"status\":\"DONE\""));
    assert!(complete_text.contains("\"result\":\"Task completed successfully\""));
}

/// Test 2: Memory lifecycle via handler (simplified)
#[tokio::test]
async fn test_memory_lifecycle_via_handler() {
    let mut registry = ToolRegistry::new();

    // Register mock memory tools
    let (store_def, store_handler) = mock_workflow_tool(
        "memory_store",
        serde_json::json!({
            "id": "mem-123",
            "summary": "test memory"
        }),
    );
    registry.register(store_def, store_handler);

    let (search_def, search_handler) = mock_workflow_tool(
        "memory_search",
        serde_json::json!({
            "results": [{"id": "mem-123", "summary": "test memory"}]
        }),
    );
    registry.register(search_def, search_handler);

    let (delete_def, delete_handler) = mock_workflow_tool(
        "memory_delete",
        serde_json::json!({
            "deleted": true
        }),
    );
    registry.register(delete_def, delete_handler);

    let handler = McpHandler::new(registry);

    // Call memory_store
    let store_req = JsonRequest {
        jsonrpc: Some("2.0".to_string()),
        id: Some(serde_json::json!(1)),
        method: "tools/call".to_string(),
        params: Some(serde_json::json!({
            "name": "memory_store",
            "arguments": {"summary": "test memory", "content": "test content"}
        })),
    };
    let store_resp = handler.handle_request(store_req).await.unwrap();
    let store_result = store_resp.result.unwrap();
    assert_eq!(
        store_result["content"][0]["text"],
        "{\"id\":\"mem-123\",\"summary\":\"test memory\"}"
    );

    // Call memory_search
    let search_req = JsonRequest {
        jsonrpc: Some("2.0".to_string()),
        id: Some(serde_json::json!(2)),
        method: "tools/call".to_string(),
        params: Some(serde_json::json!({
            "name": "memory_search",
            "arguments": {"query": "test"}
        })),
    };
    let search_resp = handler.handle_request(search_req).await.unwrap();
    let search_result = search_resp.result.unwrap();
    let search_text = search_result["content"][0]["text"].as_str().unwrap();
    assert!(search_text.contains("test memory"));

    // Call memory_delete
    let delete_req = JsonRequest {
        jsonrpc: Some("2.0".to_string()),
        id: Some(serde_json::json!(3)),
        method: "tools/call".to_string(),
        params: Some(serde_json::json!({
            "name": "memory_delete",
            "arguments": {"id": "mem-123"}
        })),
    };
    let delete_resp = handler.handle_request(delete_req).await.unwrap();
    let delete_result = delete_resp.result.unwrap();
    assert_eq!(delete_result["content"][0]["text"], "{\"deleted\":true}");
}

/// Test 3: Tool registration completeness
#[test]
fn test_tool_registration_completeness() {
    let registry = build_registry();
    let tool_count = registry.count();

    // Verify we have a reasonable number of tools (should be 89+ based on lib.rs)
    assert!(
        tool_count >= 89,
        "Expected at least 89 tools, got {}",
        tool_count
    );

    // Get all tool names
    let tools = registry.list_tools();
    let tool_names: Vec<&str> = tools.iter().map(|t| t.name.as_str()).collect();

    // Verify all expected namespace prefixes exist
    let expected_namespaces = vec![
        "workflow_",
        "memory_",
        "review_",
        "session_",
        "policy_",
        "capability_",
        "filesystem_",
        "git_",
        "npm_",
        "docker_",
        "cicd_",
        "github_",
        "tests_",
        "local_",
        "semantic-search_",
        "reminder_",
        "projectRules_",
    ];

    // Note: graph tools are under memory_ namespace (memory_create_entities, memory_search_nodes)

    for namespace in &expected_namespaces {
        let has_namespace = tool_names.iter().any(|name| name.starts_with(namespace));
        assert!(
            has_namespace,
            "Expected to find tools with namespace '{}', but none found",
            namespace
        );
    }

    // Verify use_masday exists (no underscore)
    assert!(
        tool_names.contains(&"use_masday"),
        "Expected use_masday tool to be registered"
    );
}

/// Test 4: Handler concurrent tool calls (simplified - just verify routing works)
#[tokio::test]
async fn test_handler_concurrent_tool_calls() {
    let mut registry = ToolRegistry::new();

    // Register a simple tool that returns success
    let (def, handler) =
        mock_workflow_tool("concurrent_test", serde_json::json!({"success": true}));
    registry.register(def, handler);

    let handler = Arc::new(McpHandler::new(registry));

    // Spawn 5 concurrent requests
    let mut handles = Vec::new();
    for i in 0..5 {
        let handler_clone = Arc::clone(&handler);
        let handle = tokio::spawn(async move {
            let req = JsonRequest {
                jsonrpc: Some("2.0".to_string()),
                id: Some(serde_json::json!(i + 10)),
                method: "tools/call".to_string(),
                params: Some(serde_json::json!({
                    "name": "concurrent_test",
                    "arguments": {}
                })),
            };
            handler_clone.handle_request(req).await
        });
        handles.push(handle);
    }

    // Wait for all to complete
    let mut results = Vec::new();
    for handle in handles {
        let result = handle.await.unwrap().unwrap();
        results.push(result);
    }

    // Verify all succeeded
    assert_eq!(results.len(), 5);
    for result in results {
        assert!(result.result.is_some());
        assert!(result.error.is_none());
        let result_obj = result.result.unwrap();
        let result_text = result_obj["content"][0]["text"].as_str().unwrap();
        assert!(result_text.contains("success"));
    }
}

/// Test 5: Tool namespaces in list
#[tokio::test]
async fn test_tool_namespaces_in_list() {
    let registry = build_registry();
    let handler = McpHandler::new(registry);

    // Call tools/list
    let req = JsonRequest {
        jsonrpc: Some("2.0".to_string()),
        id: Some(serde_json::json!(1)),
        method: "tools/list".to_string(),
        params: None,
    };
    let resp = handler.handle_request(req).await.unwrap();
    let result = resp.result.unwrap();
    let tools = result["tools"].as_array().unwrap();

    // Extract tool names
    let tool_names: Vec<&str> = tools.iter().filter_map(|t| t["name"].as_str()).collect();

    // Verify all expected namespace prefixes are present
    let expected_namespaces = vec![
        "workflow_",
        "memory_",
        "review_",
        "session_",
        "capability_",
        "filesystem_",
        "git_",
        "npm_",
        "docker_",
        "cicd_",
        "github_",
        "tests_",
        "local_",
        "semantic-search_",
        "policy_",
        "reminder_",
        "projectRules_",
        "use_masday",
    ];

    // Note: graph tools are under memory_ namespace (memory_create_entities, memory_search_nodes)

    for namespace in &expected_namespaces {
        let has_namespace = tool_names.iter().any(|name| {
            if *namespace == "use_masday" {
                *name == "use_masday"
            } else if namespace.ends_with('_') {
                name.starts_with(namespace)
            } else {
                name.starts_with(&format!("{}_", namespace))
            }
        });
        assert!(
            has_namespace,
            "Expected to find tools with namespace prefix '{}', but none found. Tool names: {:?}",
            namespace, tool_names
        );
    }

    // Verify we have a reasonable number of tools
    assert!(
        tool_names.len() >= 89,
        "Expected at least 89 tools, got {}",
        tool_names.len()
    );
}
