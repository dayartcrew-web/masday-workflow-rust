//! Session MCP tools - HTTP client calls to API

use crate::client;
use serde_json::Value;

pub async fn session_init_context(
    args: Value,
) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    // Generate a deterministic session key from cwd
    let cwd = args
        .get("cwd")
        .and_then(|v| v.as_str())
        .unwrap_or("default")
        .to_string();

    // Create a stable session key from the cwd path
    let session_key = cwd.replace('/', "_").replace('.', "-");

    // Initialize session via API
    let session = client::api_post(&format!("/api/sessions/{}/init", session_key), args).await?;

    // Also fetch any active reminders
    let reminders = client::api_get("/api/reminders")
        .await
        .unwrap_or(serde_json::json!([]));

    Ok(serde_json::json!({
        "initialized": true,
        "session_key": session_key,
        "cwd": cwd,
        "session": session,
        "reminders": reminders
    }))
}

pub async fn session_get_state(
    args: Value,
) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    let key = args
        .get("session_key")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "Missing session_key".to_string())?;
    client::api_get(&format!("/api/sessions/{}", key)).await
}

pub async fn session_patch_state(
    args: Value,
) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    let key = args
        .get("session_key")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "Missing session_key".to_string())?;
    client::api_patch(&format!("/api/sessions/{}", key), args).await
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    #[test]
    fn test_session_key_generation() {
        let cwd = "/home/user/project";
        let session_key = cwd.replace('/', "_").replace('.', "-");
        assert_eq!(session_key, "_home_user_project");

        let cwd = "/path/to.mcp/project";
        let session_key = cwd.replace('/', "_").replace('.', "-");
        assert_eq!(session_key, "_path_to-mcp_project");
    }

    #[test]
    fn test_session_init_args_parsing() {
        let args = json!({ "cwd": "/home/user" });
        let cwd = args.get("cwd").and_then(|v| v.as_str());
        assert!(cwd.is_some());
        assert_eq!(cwd.unwrap(), "/home/user");

        let args = json!({});
        let cwd = args.get("cwd").and_then(|v| v.as_str());
        assert!(cwd.is_none());
    }

    #[test]
    fn test_session_get_state_args() {
        let args = json!({ "session_key": "sess-123" });
        let key = args.get("session_key").and_then(|v| v.as_str());
        assert!(key.is_some());
        assert_eq!(key.unwrap(), "sess-123");

        let args = json!({});
        let key = args.get("session_key").and_then(|v| v.as_str());
        assert!(key.is_none());
    }
}
