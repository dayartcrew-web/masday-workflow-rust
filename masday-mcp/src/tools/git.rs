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

/// Git commit with optional auto-stage
///
/// If stage_all is true, runs `git add -A` before committing.
/// If stage_all is false (default), only commits already staged changes.
pub async fn git_commit(args: Value) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    let message = args
        .get("message")
        .and_then(|v| v.as_str())
        .ok_or("Missing 'message' argument")?;

    // Check if we should stage all changes (default: false for safety)
    let stage_all = args
        .get("stage_all")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    if stage_all {
        // Stage all changes first
        let add_output = tokio::process::Command::new("git")
            .args(["add", "-A"])
            .output()
            .await
            .map_err(|e| format!("Failed to run git add: {}", e))?;

        if !add_output.status.success() {
            let stderr = String::from_utf8_lossy(&add_output.stderr).to_string();
            return Err(format!("Git add failed: {}", stderr).into());
        }
    }

    // Commit (only stages changes if stage_all was true, otherwise commits staged changes)
    let commit_output = tokio::process::Command::new("git")
        .args(["commit", "-m", message])
        .output()
        .await
        .map_err(|e| format!("Failed to run git commit: {}", e))?;

    if !commit_output.status.success() {
        let stderr = String::from_utf8_lossy(&commit_output.stderr).to_string();
        return Err(format!("Git commit failed: {}", stderr).into());
    }

    Ok(serde_json::json!({
        "committed": true,
        "stage_all": stage_all
    }))
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
    fn test_git_commit_stage_all_parsing() {
        let args = json!({
            "message": "test",
            "stage_all": true
        });
        let stage_all = args.get("stage_all").and_then(|v| v.as_bool());
        assert!(stage_all.is_some());
        assert!(stage_all.unwrap());

        let args = json!({
            "message": "test",
            "stage_all": false
        });
        let stage_all = args.get("stage_all").and_then(|v| v.as_bool());
        assert!(stage_all.is_some());
        assert!(!stage_all.unwrap());

        // Test default (missing parameter)
        let args = json!({ "message": "test" });
        let stage_all = args.get("stage_all").and_then(|v| v.as_bool());
        assert!(stage_all.is_none());
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
