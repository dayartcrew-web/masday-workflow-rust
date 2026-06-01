//! CI/CD MCP tools - local operations

use serde_json::Value;

/// Get pipeline status
pub async fn cicd_pipeline_status(
    _args: Value,
) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    let output = tokio::process::Command::new("gh")
        .args(["run", "list", "--limit", "5"])
        .output()
        .await
        .map_err(|e| format!("Failed to run gh run list: {}", e))?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
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

    let output = tokio::process::Command::new("gh")
        .args(["workflow", "run", pipeline])
        .output()
        .await
        .map_err(|e| format!("Failed to run gh workflow run: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        return Err(format!("gh workflow run failed: {}", stderr).into());
    }

    Ok(serde_json::json!({ "triggered": true }))
}

/// View pipeline runs
pub async fn cicd_runs_view(
    _args: Value,
) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    let output = tokio::process::Command::new("gh")
        .args(["run", "view"])
        .output()
        .await
        .map_err(|e| format!("Failed to run gh run view: {}", e))?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        return Err(format!("gh run view failed: {}", stderr).into());
    }

    Ok(serde_json::json!({ "output": stdout }))
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
