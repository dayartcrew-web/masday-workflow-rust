//! CI/CD MCP tools - local operations

use serde_json::Value;

/// Get pipeline status
pub async fn cicd_pipeline_status(
    _args: Value,
) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    let mut cmd = tokio::process::Command::new("gh");
    cmd.args(["run", "list", "--limit", "5"]);
    let output = crate::tools::cmd::run(&mut cmd)
        .await
        .map_err(|e| format!("Failed to run gh run list: {}", e))?;

    let stdout = crate::tools::cmd::truncate_output(&String::from_utf8_lossy(&output.stdout));

    if !output.status.success() {
        let stderr = crate::tools::cmd::truncate_output(&String::from_utf8_lossy(&output.stderr));
        return Err(format!("gh run list failed: {}", stderr).into());
    }

    Ok(serde_json::json!({ "runs": stdout }))
}

/// Trigger pipeline
pub async fn cicd_pipeline_trigger(
    args: Value,
) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    let pipeline = args
        .get("pipeline")
        .and_then(|v| v.as_str())
        .ok_or("Missing 'pipeline' argument")?;

    let mut cmd = tokio::process::Command::new("gh");
    cmd.args(["workflow", "run", "--", pipeline]);
    let output = crate::tools::cmd::run(&mut cmd)
        .await
        .map_err(|e| format!("Failed to run gh workflow run: {}", e))?;

    if !output.status.success() {
        let stderr = crate::tools::cmd::truncate_output(&String::from_utf8_lossy(&output.stderr));
        return Err(format!("gh workflow run failed: {}", stderr).into());
    }

    Ok(serde_json::json!({ "triggered": true }))
}

/// View pipeline runs
pub async fn cicd_runs_view(
    _args: Value,
) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    // Use `gh run list --json` to get structured run data (doesn't require interactive TTY)
    let mut cmd = tokio::process::Command::new("gh");
    cmd.args([
        "run",
        "list",
        "--limit",
        "10",
        "--json",
        "number,status,conclusion,name,headBranch,createdAt,updatedAt",
    ]);
    let output = crate::tools::cmd::run(&mut cmd)
        .await
        .map_err(|e| format!("Failed to run gh run list: {}", e))?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();

    if !output.status.success() {
        let stderr = crate::tools::cmd::truncate_output(&String::from_utf8_lossy(&output.stderr));
        return Err(format!("gh run list failed: {}", stderr).into());
    }

    let runs: Vec<Value> =
        serde_json::from_str(&stdout).map_err(|e| format!("Failed to parse run list JSON: {}", e))?;

    Ok(serde_json::json!({ "runs": runs, "count": runs.len() }))
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    #[test]
    fn test_cicd_pipeline_trigger_args() {
        let args = json!({ "pipeline": "ci.yml" });
        let pipeline = args.get("pipeline").and_then(|v| v.as_str());
        assert!(pipeline.is_some());
        assert_eq!(pipeline.unwrap(), "ci.yml");

        let args = json!({});
        let pipeline = args.get("pipeline").and_then(|v| v.as_str());
        assert!(pipeline.is_none());
    }
}
