//! Project Rules MCP tools - validation

use serde_json::Value;

/// Check project rules
pub async fn projectrules_check(
    args: Value,
) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    let project_root = args
        .get("projectRoot")
        .and_then(|v| v.as_str())
        .ok_or("Missing 'projectRoot' argument")?;

    let rules_dir = std::path::Path::new(project_root)
        .join(".claude")
        .join("rules");

    // Check if rules directory exists
    let metadata_result = tokio::fs::metadata(&rules_dir).await;
    let is_dir = match metadata_result {
        Ok(m) => m.is_dir(),
        Err(_) => false,
    };

    if !is_dir {
        let errors: Vec<String> = vec![".claude/rules directory not found".to_string()];
        let warnings: Vec<String> = vec![];
        return Ok(serde_json::json!({
            "valid": false,
            "errors": errors,
            "warnings": warnings
        }));
    }

    let mut errors: Vec<String> = Vec::new();
    let mut warnings: Vec<String> = Vec::new();
    let mut files_checked = 0;

    // List all .md files in rules directory
    let mut dir = tokio::fs::read_dir(&rules_dir)
        .await
        .map_err(|e| format!("Failed to read rules directory: {}", e))?;

    while let Some(entry) = dir
        .next_entry()
        .await
        .map_err(|e| format!("Failed to read entry: {}", e))?
    {
        let path = entry.path();

        // Only check .md files
        if path.extension().and_then(|s| s.to_str()) != Some("md") {
            continue;
        }

        files_checked += 1;

        // Read file content
        let content = tokio::fs::read_to_string(&path)
            .await
            .map_err(|e| format!("Failed to read file {}: {}", path.display(), e))?;

        // Basic validation checks
        if content.is_empty() || content.len() < 50 {
            warnings.push(format!(
                "Rule file {} is very short ({} bytes)",
                path.display(),
                content.len()
            ));
        }

        // Check for markdown headers
        if !content.contains('#') {
            warnings.push(format!(
                "Rule file {} lacks markdown headers",
                path.display()
            ));
        }

        // Check for common rule sections
        if !content.contains("##") && content.len() > 200 {
            warnings.push(format!(
                "Rule file {} has no subsections (consider using ## headers)",
                path.display()
            ));
        }
    }

    // Basic project health checks
    if files_checked == 0 {
        errors.push("No rule files found in .claude/rules".to_string());
    }

    Ok(serde_json::json!({
        "valid": errors.is_empty(),
        "errors": errors,
        "warnings": warnings,
        "files_checked": files_checked
    }))
}
