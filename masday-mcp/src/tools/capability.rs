//! Capability MCP tools - HTTP client calls to API

use crate::client;
use serde_json::Value;

pub async fn capability_create_agent(
    args: Value,
) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    client::api_post("/api/capabilities/agent", args).await
}

pub async fn capability_create_skill(
    args: Value,
) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    client::api_post("/api/capabilities/skill", args).await
}

pub async fn capability_list_agents(
    _args: Value,
) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    client::api_get("/api/capabilities/agents").await
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
    let task = args
        .get("task")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "Missing task".to_string())?;
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
    let workflow_id = args
        .get("workflow_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "Missing workflow_id".to_string())?;
    client::api_get(&format!("/api/capabilities/audit/{}", workflow_id)).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_capability_match_agent_args() {
        let args = serde_json::json!({ "task": "fix bug in auth module" });
        let task = args.get("task").and_then(|v| v.as_str());
        assert!(task.is_some());
        assert_eq!(task.unwrap(), "fix bug in auth module");

        let args = serde_json::json!({});
        let task = args.get("task").and_then(|v| v.as_str());
        assert!(task.is_none());
    }

    #[test]
    fn test_capability_workflow_audit_args() {
        let args = serde_json::json!({ "workflow_id": "wf-123" });
        let workflow_id = args.get("workflow_id").and_then(|v| v.as_str());
        assert!(workflow_id.is_some());
        assert_eq!(workflow_id.unwrap(), "wf-123");

        let args = serde_json::json!({});
        let workflow_id = args.get("workflow_id").and_then(|v| v.as_str());
        assert!(workflow_id.is_none());
    }
}
