//! Capability MCP tools - HTTP client calls to API

use crate::client;
use serde_json::Value;

/// Resolve a string arg by trying the schema-advertised key first, then a legacy
/// alias. The HTTP capability handlers must read the camelCase keys advertised in
/// their `schema!` (`taskDescription`, `workflowId`, `projectRoot`) — otherwise a
/// client sending the advertised param gets a "Missing …" error. The stdio
/// handlers in `direct.rs` already read these; this restores parity. The legacy
/// fallback keeps existing callers working.
fn arg_str<'a>(args: &'a Value, advertised: &str, legacy: &str) -> Option<&'a str> {
    args.get(advertised)
        .and_then(|v| v.as_str())
        .or_else(|| args.get(legacy).and_then(|v| v.as_str()))
}

/// Validate agent/skill name format
fn validate_name(name: &str) -> Result<(), String> {
    if name.is_empty() {
        return Err("name cannot be empty".into());
    }
    if name.len() > 100 {
        return Err("name must be 100 characters or less".into());
    }
    if !name
        .chars()
        .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
    {
        return Err(
            "name must contain only alphanumeric characters, hyphens, and underscores".into(),
        );
    }
    Ok(())
}

/// Validate agent role
fn validate_role(role: &str) -> Result<(), String> {
    if role.is_empty() {
        return Err("role cannot be empty".into());
    }
    if role.len() > 200 {
        return Err("role must be 200 characters or less".into());
    }
    Ok(())
}

/// Validate description
fn validate_description(desc: &str) -> Result<(), String> {
    if desc.is_empty() {
        return Err("description cannot be empty".into());
    }
    if desc.len() > 500 {
        return Err("description must be 500 characters or less".into());
    }
    Ok(())
}

/// Validate instructions (for agents)
fn validate_instructions(instructions: &str) -> Result<(), String> {
    if instructions.is_empty() {
        return Err("instructions cannot be empty".into());
    }
    Ok(())
}

/// Validate trigger (for skills)
fn validate_trigger(trigger: &str) -> Result<(), String> {
    if trigger.is_empty() {
        return Err("trigger cannot be empty".into());
    }
    Ok(())
}

/// Validate steps array (for skills)
fn validate_steps(steps: &Value) -> Result<(), String> {
    if let Some(arr) = steps.as_array() {
        if arr.is_empty() {
            return Err("steps cannot be empty".into());
        }
        Ok(())
    } else {
        Err("steps must be an array".into())
    }
}

pub async fn capability_create_agent(
    args: Value,
) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    // Extract and validate fields before API call
    let name = args
        .get("name")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "Missing name field".to_string())?;

    let role = args
        .get("role")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "Missing role field".to_string())?;

    let description = args
        .get("description")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "Missing description field".to_string())?;

    let instructions = args
        .get("instructions")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "Missing instructions field".to_string())?;

    // Run validations
    validate_name(name).map_err(|e| format!("Invalid name: {}", e))?;
    validate_role(role).map_err(|e| format!("Invalid role: {}", e))?;
    validate_description(description).map_err(|e| format!("Invalid description: {}", e))?;
    validate_instructions(instructions).map_err(|e| format!("Invalid instructions: {}", e))?;

    client::api_post("/api/capabilities/agent", args).await
}

pub async fn capability_create_skill(
    args: Value,
) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    // Extract and validate fields before API call
    let name = args
        .get("name")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "Missing name field".to_string())?;

    let description = args
        .get("description")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "Missing description field".to_string())?;

    let trigger = args
        .get("trigger")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "Missing trigger field".to_string())?;

    let steps = args
        .get("steps")
        .ok_or_else(|| "Missing steps field".to_string())?;

    // Run validations
    validate_name(name).map_err(|e| format!("Invalid name: {}", e))?;
    validate_description(description).map_err(|e| format!("Invalid description: {}", e))?;
    validate_trigger(trigger).map_err(|e| format!("Invalid trigger: {}", e))?;
    validate_steps(steps).map_err(|e| format!("Invalid steps: {}", e))?;

    client::api_post("/api/capabilities/skill", args).await
}

pub async fn capability_list_agents(
    args: Value,
) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    // Forward the advertised `projectRoot` so the route loads the registry from
    // the user's project, not the API server's CWD (the route defaults to ".").
    // Stdio `direct.rs` already reads projectRoot here.
    let project_root = arg_str(&args, "projectRoot", "project_root").unwrap_or(".");
    client::api_get(&format!(
        "/api/capabilities/agents?project_root={}",
        project_root
    ))
    .await
}

pub async fn capability_list_skills(
    _args: Value,
) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    client::api_get("/api/capabilities/skills").await
}

pub async fn capability_list_templates(
    _args: Value,
) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    client::api_get("/api/capabilities/templates").await
}

pub async fn capability_match_agent(
    args: Value,
) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    // Read the schema-advertised `taskDescription` (falls back to legacy `task`).
    // Without this a client sending `taskDescription` gets "Missing task" — the
    // stdio handler in `direct.rs` already reads both keys.
    let task = arg_str(&args, "taskDescription", "task")
        .ok_or_else(|| "Missing taskDescription".to_string())?;
    client::api_get(&format!("/api/capabilities/match?task={}", task)).await
}

pub async fn capability_scaffold_feature(
    args: Value,
) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    client::api_post("/api/capabilities/scaffold", args).await
}

pub async fn capability_scaffold_mcp_server(
    args: Value,
) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    client::api_post("/api/capabilities/mcp-server", args).await
}

pub async fn capability_system_readiness(
    _args: Value,
) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    client::api_get("/api/capabilities/readiness").await
}

pub async fn capability_workflow_audit(
    args: Value,
) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    // Read the schema-advertised `workflowId` (falls back to legacy
    // `workflow_id`). Without this a client sending `workflowId` gets
    // "Missing workflow_id" — the stdio handler in `direct.rs` already reads
    // both keys.
    let workflow_id = arg_str(&args, "workflowId", "workflow_id")
        .ok_or_else(|| "Missing workflowId".to_string())?;
    client::api_get(&format!("/api/capabilities/audit/{}", workflow_id)).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_name_valid() {
        assert!(validate_name("test-agent").is_ok());
        assert!(validate_name("my_agent").is_ok());
        assert!(validate_name("Agent123").is_ok());
        assert!(validate_name("a").is_ok());
        assert!(validate_name("test_agent-123").is_ok());
    }

    #[test]
    fn test_validate_name_invalid() {
        assert!(validate_name("").is_err());
        assert!(validate_name("test agent").is_err());
        assert!(validate_name("test.agent").is_err());
        assert!(validate_name("test/agent").is_err());
        assert!(validate_name("test@agent").is_err());
        assert!(validate_name("test agent").is_err());
        // Test 101 characters
        assert!(validate_name(&"a".repeat(101)).is_err());
        // Test 100 characters (should be OK)
        assert!(validate_name(&"a".repeat(100)).is_ok());
    }

    #[test]
    fn test_validate_role_valid() {
        assert!(validate_role("Backend developer").is_ok());
        assert!(validate_role("a").is_ok());
        assert!(validate_role(&"a".repeat(200)).is_ok());
    }

    #[test]
    fn test_validate_role_invalid() {
        assert!(validate_role("").is_err());
        assert!(validate_role(&"a".repeat(201)).is_err());
    }

    #[test]
    fn test_validate_description_valid() {
        assert!(validate_description("Test description").is_ok());
        assert!(validate_description("a").is_ok());
        assert!(validate_description(&"a".repeat(500)).is_ok());
    }

    #[test]
    fn test_validate_description_invalid() {
        assert!(validate_description("").is_err());
        assert!(validate_description(&"a".repeat(501)).is_err());
    }

    #[test]
    fn test_validate_instructions_valid() {
        assert!(validate_instructions("Some instructions").is_ok());
        assert!(validate_instructions("a").is_ok());
    }

    #[test]
    fn test_validate_instructions_invalid() {
        assert!(validate_instructions("").is_err());
    }

    #[test]
    fn test_validate_trigger_valid() {
        assert!(validate_trigger("When user mentions X").is_ok());
        assert!(validate_trigger("a").is_ok());
    }

    #[test]
    fn test_validate_trigger_invalid() {
        assert!(validate_trigger("").is_err());
    }

    #[test]
    fn test_validate_steps_valid() {
        assert!(validate_steps(&serde_json::json!(["step1", "step2"])).is_ok());
        assert!(validate_steps(&serde_json::json!(["step1"])).is_ok());
        assert!(validate_steps(&serde_json::json!([1, 2, 3])).is_ok());
    }

    #[test]
    fn test_validate_steps_invalid() {
        assert!(validate_steps(&serde_json::json!([])).is_err());
        assert!(validate_steps(&serde_json::json!("not array")).is_err());
        assert!(validate_steps(&serde_json::json!(null)).is_err());
        assert!(validate_steps(&serde_json::json!(123)).is_err());
    }

    // arg_str underpins the HTTP capability handlers reading the schema-
    // advertised camelCase keys (taskDescription/workflowId/projectRoot) with a
    // legacy fallback — see capability_match_agent / capability_workflow_audit /
    // capability_list_agents. The stdio handlers in direct.rs already read both.

    #[test]
    fn arg_str_prefers_advertised_key() {
        let args = serde_json::json!({"taskDescription": "build api", "task": "legacy"});
        assert_eq!(arg_str(&args, "taskDescription", "task"), Some("build api"));
    }

    #[test]
    fn arg_str_falls_back_to_legacy() {
        let args = serde_json::json!({"task": "legacy-only"});
        assert_eq!(
            arg_str(&args, "taskDescription", "task"),
            Some("legacy-only")
        );
    }

    #[test]
    fn arg_str_none_when_both_absent() {
        let args = serde_json::json!({"other": "x"});
        assert_eq!(arg_str(&args, "taskDescription", "task"), None);
    }

    #[test]
    fn arg_str_none_when_not_a_string() {
        // A non-string value must not satisfy the lookup (matches the .as_str() guard).
        let args = serde_json::json!({"taskDescription": 123});
        assert_eq!(arg_str(&args, "taskDescription", "task"), None);
    }

    #[test]
    fn arg_str_workflow_audit_keys() {
        let advertised = serde_json::json!({"workflowId": "wf-1"});
        let legacy = serde_json::json!({"workflow_id": "wf-2"});
        assert_eq!(
            arg_str(&advertised, "workflowId", "workflow_id"),
            Some("wf-1")
        );
        assert_eq!(arg_str(&legacy, "workflowId", "workflow_id"), Some("wf-2"));
    }
}
