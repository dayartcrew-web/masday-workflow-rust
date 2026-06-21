//! NPM MCP tools - local operations

use serde_json::Value;

/// Run npm install
pub async fn npm_install(args: Value) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    let packages = args.get("packages").and_then(|v| v.as_array());

    let output = if let Some(pkgs) = packages {
        let pkg_names: Vec<&str> = pkgs.iter().filter_map(|v| v.as_str()).collect();

        // Each package is a bare positional to `pnpm add`; reject dash-leading
        // names up front (defense-in-depth alongside the `--` separator below).
        for p in &pkg_names {
            crate::tools::cmd::reject_flag_like(p, "package")?;
        }

        if pkg_names.is_empty() {
            let mut cmd = tokio::process::Command::new("pnpm");
            cmd.args(["install"]);
            crate::tools::cmd::run(&mut cmd).await
        } else {
            let mut cmd = tokio::process::Command::new("pnpm");
            cmd.args(["add", "--"]).args(&pkg_names);
            crate::tools::cmd::run(&mut cmd).await
        }
    } else {
        let mut cmd = tokio::process::Command::new("pnpm");
        cmd.args(["install"]);
        crate::tools::cmd::run(&mut cmd).await
    }
    .map_err(|e| format!("Failed to run pnpm install: {}", e))?;

    let stdout = crate::tools::cmd::truncate_output(&String::from_utf8_lossy(&output.stdout));
    let stderr = crate::tools::cmd::truncate_output(&String::from_utf8_lossy(&output.stderr));

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

    // `script` is a bare positional to `pnpm run`; reject a dash-leading value
    // up front (defense-in-depth alongside the `--` separator below).
    crate::tools::cmd::reject_flag_like(script, "script")?;

    let mut cmd = tokio::process::Command::new("pnpm");
    cmd.args(["run", "--", script]);
    let output = crate::tools::cmd::run(&mut cmd)
        .await
        .map_err(|e| format!("Failed to run pnpm {}: {}", script, e))?;

    let stdout = crate::tools::cmd::truncate_output(&String::from_utf8_lossy(&output.stdout));
    let stderr = crate::tools::cmd::truncate_output(&String::from_utf8_lossy(&output.stderr));

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
