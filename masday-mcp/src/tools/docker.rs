//! Docker MCP tools - local operations

use serde_json::Value;

/// Build Docker image
pub async fn docker_build(args: Value) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    let tag = args.get("tag").and_then(|v| v.as_str());

    let cmd_args = if let Some(t) = tag {
        vec!["build", "-t", t, "."]
    } else {
        vec!["build", "."]
    };

    let output = tokio::process::Command::new("docker")
        .args(&cmd_args)
        .output()
        .await
        .map_err(|e| format!("Failed to run docker build: {}", e))?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    if !output.status.success() {
        return Err(format!("Docker build failed: {}", stderr).into());
    }

    Ok(serde_json::json!({ "output": stdout }))
}

/// Run Docker container
pub async fn docker_run(args: Value) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    let image = args
        .get("image")
        .and_then(|v| v.as_str())
        .ok_or("Missing 'image' argument")?;

    let output = tokio::process::Command::new("docker")
        .args(["run", image])
        .output()
        .await
        .map_err(|e| format!("Failed to run docker run: {}", e))?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    if !output.status.success() {
        return Err(format!("Docker run failed: {}", stderr).into());
    }

    Ok(serde_json::json!({ "output": stdout }))
}

/// List Docker containers
pub async fn docker_ps(_args: Value) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    let output = tokio::process::Command::new("docker")
        .args(["ps", "--format", "json"])
        .output()
        .await
        .map_err(|e| format!("Failed to run docker ps: {}", e))?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        return Err(format!("Docker ps failed: {}", stderr).into());
    }

    // Parse JSON lines into an array
    let containers: Vec<Value> = stdout
        .lines()
        .filter(|line| !line.is_empty())
        .filter_map(|line| serde_json::from_str(line).ok())
        .collect();

    Ok(serde_json::json!({ "containers": containers }))
}
