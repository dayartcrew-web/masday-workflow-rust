//! GitHub MCP tools - local operations

use serde_json::Value;

/// Create GitHub PR
pub async fn github_pr_create(
    args: Value,
) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    let title = args
        .get("title")
        .and_then(|v| v.as_str())
        .ok_or("Missing 'title' argument")?;

    let body = args
        .get("body")
        .and_then(|v| v.as_str())
        .ok_or("Missing 'body' argument")?;

    let output = tokio::process::Command::new("gh")
        .args(["pr", "create", "--title", title, "--body", body])
        .output()
        .await
        .map_err(|e| format!("Failed to run gh pr create: {}", e))?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        return Err(format!("gh pr create failed: {}", stderr).into());
    }

    // Extract URL from output (gh typically outputs the URL)
    let url = stdout.lines().next().unwrap_or(&stdout).trim();

    Ok(serde_json::json!({ "url": url }))
}

/// List GitHub PRs
pub async fn github_pr_list(
    _args: Value,
) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    let output = tokio::process::Command::new("gh")
        .args(["pr", "list", "--json", "number,title,state"])
        .output()
        .await
        .map_err(|e| format!("Failed to run gh pr list: {}", e))?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        return Err(format!("gh pr list failed: {}", stderr).into());
    }

    let prs: Vec<Value> = serde_json::from_str(&stdout)
        .map_err(|e| format!("Failed to parse PR list JSON: {}", e))?;

    Ok(serde_json::json!({ "prs": prs }))
}

/// List GitHub issues
pub async fn github_issue_list(
    _args: Value,
) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    let output = tokio::process::Command::new("gh")
        .args(["issue", "list", "--json", "number,title,state"])
        .output()
        .await
        .map_err(|e| format!("Failed to run gh issue list: {}", e))?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        return Err(format!("gh issue list failed: {}", stderr).into());
    }

    let issues: Vec<Value> = serde_json::from_str(&stdout)
        .map_err(|e| format!("Failed to parse issue list JSON: {}", e))?;

    Ok(serde_json::json!({ "issues": issues }))
}
