//! Project Rules MCP tools - validation

use serde_json::Value;

/// Check project rules
pub async fn projectrules_check(
    args: Value,
) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    let project_root = args
        .get("project_root")
        .and_then(|v| v.as_str())
        .or_else(|| args.get("projectRoot").and_then(|v| v.as_str()))
        .ok_or("Missing 'project_root' argument")?;

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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn test_projectrules_check_missing_rules_dir() {
        let temp_dir = tempfile::tempdir().unwrap();
        let project_root = temp_dir.path().to_str().unwrap();

        let args = json!({ "project_root": project_root });
        let result = projectrules_check(args).await;

        assert!(result.is_ok());
        let result_json = result.unwrap();
        assert_eq!(result_json["valid"], false);
        assert!(!result_json["errors"].as_array().unwrap().is_empty());
    }

    #[tokio::test]
    async fn test_projectrules_check_with_rules() {
        let temp_dir = tempfile::tempdir().unwrap();
        let rules_dir = temp_dir.path().join(".claude/rules");
        tokio::fs::create_dir_all(&rules_dir).await.unwrap();

        // Create a test rule file
        let rule_file = rules_dir.join("test-rule.md");
        tokio::fs::write(&rule_file, "# Test Rule\n\n## Section\n\nContent here")
            .await
            .unwrap();

        let project_root = temp_dir.path().to_str().unwrap();
        let args = json!({ "project_root": project_root });
        let result = projectrules_check(args).await;

        assert!(result.is_ok());
        let result_json = result.unwrap();
        assert_eq!(result_json["valid"], true);
        assert_eq!(result_json["files_checked"], 1);
    }

    #[tokio::test]
    async fn test_projectrules_check_missing_project_root() {
        let args = json!({});
        let result = projectrules_check(args).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_projectrules_check_empty_rule_file() {
        let temp_dir = tempfile::tempdir().unwrap();
        let rules_dir = temp_dir.path().join(".claude/rules");
        tokio::fs::create_dir_all(&rules_dir).await.unwrap();

        // Create an empty rule file (should generate warning)
        let rule_file = rules_dir.join("empty.md");
        tokio::fs::write(&rule_file, "").await.unwrap();

        let project_root = temp_dir.path().to_str().unwrap();
        let args = json!({ "project_root": project_root });
        let result = projectrules_check(args).await;

        assert!(result.is_ok());
        let result_json = result.unwrap();
        // Empty file should still be valid but with warning
        assert!(!result_json["warnings"].as_array().unwrap().is_empty());
    }

    #[tokio::test]
    async fn test_projectrules_check_no_headers() {
        let temp_dir = tempfile::tempdir().unwrap();
        let rules_dir = temp_dir.path().join(".claude/rules");
        tokio::fs::create_dir_all(&rules_dir).await.unwrap();

        // Create file without markdown headers
        let rule_file = rules_dir.join("no-headers.md");
        tokio::fs::write(&rule_file, "Just plain text without any markdown headers")
            .await
            .unwrap();

        let project_root = temp_dir.path().to_str().unwrap();
        let args = json!({ "project_root": project_root });
        let result = projectrules_check(args).await;

        assert!(result.is_ok());
        let result_json = result.unwrap();
        // Should generate warning about missing headers
        assert!(!result_json["warnings"].as_array().unwrap().is_empty());
    }
}
