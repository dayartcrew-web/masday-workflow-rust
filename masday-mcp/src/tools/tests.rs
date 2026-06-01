//! Tests MCP tools - local operations

use serde_json::Value;

/// Run tests
pub async fn tests_run(args: Value) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    let pattern = args.get("pattern").and_then(|v| v.as_str());

    // Check if we're in a Rust project (Cargo.toml exists) or npm project (package.json exists)
    let is_rust = tokio::fs::metadata("Cargo.toml").await.is_ok();
    let is_npm = tokio::fs::metadata("package.json").await.is_ok();

    let output = if is_rust {
        let cmd_args = if let Some(p) = pattern {
            vec!["test", p]
        } else {
            vec!["test"]
        };

        tokio::process::Command::new("cargo")
            .args(&cmd_args)
            .output()
            .await
    } else if is_npm {
        let cmd_args = if let Some(p) = pattern {
            vec!["test", p]
        } else {
            vec!["test"]
        };

        tokio::process::Command::new("pnpm")
            .args(&cmd_args)
            .output()
            .await
    } else {
        return Err("No Cargo.toml or package.json found".into());
    };

    let output = output.map_err(|e| format!("Failed to run tests: {}", e))?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    let passed = output.status.success();

    Ok(serde_json::json!({
        "output": format!("{}\n{}", stdout, stderr),
        "passed": passed
    }))
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    #[test]
    fn test_tests_run_args_parsing() {
        let args = json!({ "pattern": "unit" });
        let pattern = args.get("pattern").and_then(|v| v.as_str());
        assert!(pattern.is_some());
        assert_eq!(pattern.unwrap(), "unit");

        let args = json!({});
        let pattern = args.get("pattern").and_then(|v| v.as_str());
        assert!(pattern.is_none());
    }

    #[test]
    fn test_command_args_building() {
        let pattern = Some("unit");
        let cmd_args = if let Some(p) = pattern {
            vec!["test", p]
        } else {
            vec!["test"]
        };
        assert_eq!(cmd_args, vec!["test", "unit"]);

        let pattern = None;
        let cmd_args = if let Some(p) = pattern {
            vec!["test", p]
        } else {
            vec!["test"]
        };
        assert_eq!(cmd_args, vec!["test"]);
    }
}
