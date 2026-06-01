//! Git MCP tools - local operations

use serde_json::Value;

/// Get git status
pub async fn git_status(_args: Value) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    let output = tokio::process::Command::new("git")
        .args(["status", "--porcelain"])
        .output()
        .await
        .map_err(|e| format!("Failed to run git status: {}", e))?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        return Err(format!("Git status failed: {}", stderr).into());
    }

    Ok(serde_json::json!({ "output": stdout }))
}

/// Get git diff
pub async fn git_diff(_args: Value) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    let output = tokio::process::Command::new("git")
        .args(["diff"])
        .output()
        .await
        .map_err(|e| format!("Failed to run git diff: {}", e))?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        return Err(format!("Git diff failed: {}", stderr).into());
    }

    Ok(serde_json::json!({ "output": stdout }))
}

/// Git commit
pub async fn git_commit(args: Value) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    let message = args
        .get("message")
        .and_then(|v| v.as_str())
        .ok_or("Missing 'message' argument")?;

    // First, stage all changes
    let add_output = tokio::process::Command::new("git")
        .args(["add", "-A"])
        .output()
        .await
        .map_err(|e| format!("Failed to run git add: {}", e))?;

    if !add_output.status.success() {
        let stderr = String::from_utf8_lossy(&add_output.stderr).to_string();
        return Err(format!("Git add failed: {}", stderr).into());
    }

    // Then commit
    let commit_output = tokio::process::Command::new("git")
        .args(["commit", "-m", message])
        .output()
        .await
        .map_err(|e| format!("Failed to run git commit: {}", e))?;

    if !commit_output.status.success() {
        let stderr = String::from_utf8_lossy(&commit_output.stderr).to_string();
        return Err(format!("Git commit failed: {}", stderr).into());
    }

    Ok(serde_json::json!({ "committed": true }))
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    #[test]
    fn test_git_commit_args_parsing() {
        let args = json!({ "message": "feat: add new feature" });
        let message = args.get("message").and_then(|v| v.as_str());
        assert!(message.is_some());
        assert_eq!(message.unwrap(), "feat: add new feature");

        let args = json!({});
        let message = args.get("message").and_then(|v| v.as_str());
        assert!(message.is_none());
    }

    #[test]
    fn test_command_building() {
        let message = "test commit";
        let args = vec!["commit", "-m", message];
        assert_eq!(args, vec!["commit", "-m", "test commit"]);

        let args = vec!["status", "--porcelain"];
        assert_eq!(args, vec!["status", "--porcelain"]);
    }
}
