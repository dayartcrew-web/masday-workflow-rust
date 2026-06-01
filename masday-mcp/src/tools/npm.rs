//! NPM MCP tools - local operations

use serde_json::Value;

/// Run npm install
pub async fn npm_install(args: Value) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    let packages = args.get("packages").and_then(|v| v.as_array());

    let output = if let Some(pkgs) = packages {
        let pkg_names: Vec<&str> = pkgs.iter().filter_map(|v| v.as_str()).collect();

        if pkg_names.is_empty() {
            tokio::process::Command::new("pnpm")
                .args(["install"])
                .output()
                .await
        } else {
            tokio::process::Command::new("pnpm")
                .args(["add"])
                .args(&pkg_names)
                .output()
                .await
        }
    } else {
        tokio::process::Command::new("pnpm")
            .args(["install"])
            .output()
            .await
    }
    .map_err(|e| format!("Failed to run pnpm install: {}", e))?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    if !output.status.success() {
        return Err(format!("pnpm install failed: {}", stderr).into());
    }

    Ok(serde_json::json!({ "output": stdout }))
}

/// Run npm script
pub async fn npm_run(args: Value) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    let script = args
        .get("script")
        .and_then(|v| v.as_str())
        .ok_or("Missing 'script' argument")?;

    let output = tokio::process::Command::new("pnpm")
        .args([script])
        .output()
        .await
        .map_err(|e| format!("Failed to run pnpm {}: {}", script, e))?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    if !output.status.success() {
        return Err(format!("pnpm {} failed: {}", script, stderr).into());
    }

    Ok(serde_json::json!({ "output": stdout }))
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    #[test]
    fn test_npm_install_args_parsing() {
        let args = json!({
            "packages": ["react", "react-dom"]
        });

        let packages = args.get("packages").and_then(|v| v.as_array());
        assert!(packages.is_some());

        let pkg_names: Vec<&str> = packages
            .unwrap()
            .iter()
            .filter_map(|v| v.as_str())
            .collect();
        assert_eq!(pkg_names, vec!["react", "react-dom"]);
    }

    #[test]
    fn test_npm_install_no_packages() {
        let args = json!({});
        let packages = args.get("packages").and_then(|v| v.as_array());
        assert!(packages.is_none());

        let args = json!({ "packages": [] });
        let packages = args.get("packages").and_then(|v| v.as_array());
        assert!(packages.is_some());
        assert!(packages.unwrap().is_empty());
    }

    #[test]
    fn test_npm_run_args_parsing() {
        let args = json!({ "script": "build" });
        let script = args.get("script").and_then(|v| v.as_str());
        assert!(script.is_some());
        assert_eq!(script.unwrap(), "build");

        let args = json!({});
        let script = args.get("script").and_then(|v| v.as_str());
        assert!(script.is_none());
    }
}
