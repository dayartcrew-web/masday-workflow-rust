//! Policy MCP tools - HTTP client calls to API

use crate::client;
use serde_json::Value;

pub async fn policy_check_session_readiness(
    args: Value,
) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    client::api_post("/api/policy/session-readiness", args).await
}

pub async fn policy_validate_completion(
    args: Value,
) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    client::api_post("/api/policy/validate-completion", args).await
}

pub async fn policy_validate_execution(
    args: Value,
) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    client::api_post("/api/policy/validate-execution", args).await
}

pub async fn policy_validate_parallel_completion(
    args: Value,
) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    client::api_post("/api/policy/validate-parallel", args).await
}

pub async fn policy_detect_scope_drift(
    args: Value,
) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    let workflow_id = args
        .get("workflow_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "Missing workflow_id".to_string())?;
    client::api_post(&format!("/api/policy/drift/{}", workflow_id), args).await
}

pub async fn policy_require_context_refresh(
    args: Value,
) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    client::api_post("/api/policy/context-refresh", args).await
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    #[test]
    fn test_policy_detect_scope_drift_args() {
        let args = json!({ "workflow_id": "wf-123" });
        let workflow_id = args.get("workflow_id").and_then(|v| v.as_str());
        assert!(workflow_id.is_some());
        assert_eq!(workflow_id.unwrap(), "wf-123");

        let args = json!({});
        let workflow_id = args.get("workflow_id").and_then(|v| v.as_str());
        assert!(workflow_id.is_none());
    }

    #[test]
    fn test_url_building() {
        let workflow_id = "wf-123";
        let url = format!("/api/policy/drift/{}", workflow_id);
        assert_eq!(url, "/api/policy/drift/wf-123");
    }
}
