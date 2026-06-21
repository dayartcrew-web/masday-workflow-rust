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

    let mut cmd = tokio::process::Command::new("docker");
    cmd.args(&cmd_args);
    let output = crate::tools::cmd::run(&mut cmd)
        .await
        .map_err(|e| format!("Failed to run docker build: {}", e))?;

    let stdout = crate::tools::cmd::truncate_output(&String::from_utf8_lossy(&output.stdout));
    let stderr = crate::tools::cmd::truncate_output(&String::from_utf8_lossy(&output.stderr));

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

    // `image` is a bare positional to `docker run`; reject a dash-leading value
    // up front (defense-in-depth alongside the `--` separator below; a clear
    // error beats docker's opaque "invalid reference format").
    crate::tools::cmd::reject_flag_like(image, "image")?;

    let mut cmd = tokio::process::Command::new("docker");
    cmd.args(["run", "--", image]);
    let output = crate::tools::cmd::run(&mut cmd)
        .await
        .map_err(|e| format!("Failed to run docker run: {}", e))?;

    let stdout = crate::tools::cmd::truncate_output(&String::from_utf8_lossy(&output.stdout));
    let stderr = crate::tools::cmd::truncate_output(&String::from_utf8_lossy(&output.stderr));

    if !output.status.success() {
        return Err(format!("Docker run failed: {}", stderr).into());
    }

    Ok(serde_json::json!({ "output": stdout }))
}

/// List Docker containers
pub async fn docker_ps(_args: Value) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    let mut cmd = tokio::process::Command::new("docker");
    cmd.args(["ps", "--format", "json"]);
    let output = crate::tools::cmd::run(&mut cmd)
        .await
        .map_err(|e| format!("Failed to run docker ps: {}", e))?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();

    if !output.status.success() {
        let stderr = crate::tools::cmd::truncate_output(&String::from_utf8_lossy(&output.stderr));
        return Err(format!("Docker ps failed: {}", stderr).into());
    }

    // Parse JSON lines into an array
    let mut parse_errors = 0i32;
    let containers: Vec<Value> = stdout
        .lines()
        .filter(|line| !line.is_empty())
        .filter_map(|line| match serde_json::from_str(line) {
            Ok(v) => Some(v),
            Err(e) => {
                tracing::warn!("docker_ps: failed to parse container JSON: {e} | line: {line}");
                parse_errors += 1;
                None
            }
        })
        .collect();

    let mut result = serde_json::json!({ "containers": containers });
    if parse_errors > 0 {
        result["parse_warnings"] = serde_json::json!(parse_errors);
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    #[test]
    fn test_docker_build_args_parsing() {
        // Test argument parsing logic (without actually running docker)
        let tag = Some("myimage:latest");
        let cmd_args = if let Some(t) = tag {
            vec!["build", "-t", t, "."]
        } else {
            vec!["build", "."]
        };
        assert_eq!(cmd_args, vec!["build", "-t", "myimage:latest", "."]);

        let tag = None;
        let cmd_args = if let Some(t) = tag {
            vec!["build", "-t", t, "."]
        } else {
            vec!["build", "."]
        };
        assert_eq!(cmd_args, vec!["build", "."]);
    }

    #[test]
    fn test_docker_parse_args() {
        let args = json!({ "tag": "test:latest" });
        assert!(args.get("tag").and_then(|v| v.as_str()).is_some());

        let args = json!({ "image": "nginx:latest" });
        assert_eq!(
            args.get("image").and_then(|v| v.as_str()).unwrap(),
            "nginx:latest"
        );
    }

    #[test]
    fn test_docker_container_json_parsing() {
        let stdout = r#"{"ID":"abc123","Names":"web"}
{"ID":"def456","Names":"api"}"#;

        let mut parse_errors = 0i32;
        let containers: Vec<serde_json::Value> = stdout
            .lines()
            .filter(|line| !line.is_empty())
            .filter_map(|line| match serde_json::from_str(line) {
                Ok(v) => Some(v),
                Err(_) => {
                    parse_errors += 1;
                    None
                }
            })
            .collect();

        assert_eq!(containers.len(), 2);
        assert_eq!(parse_errors, 0);

        // Test with mixed valid/invalid lines
        let stdout = r#"{"ID":"abc123"}
not-json
{"ID":"def456"}"#;

        let mut parse_errors = 0i32;
        let containers: Vec<serde_json::Value> = stdout
            .lines()
            .filter(|line| !line.is_empty())
            .filter_map(|line| match serde_json::from_str(line) {
                Ok(v) => Some(v),
                Err(_) => {
                    parse_errors += 1;
                    None
                }
            })
            .collect();

        assert_eq!(containers.len(), 2);
        assert_eq!(parse_errors, 1);
    }
}
