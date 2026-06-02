//! Local MCP tools - file-based state operations

use serde_json::Value;

/// Initialize local state directory
pub async fn local_init(args: Value) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    let cwd = args
        .get("cwd")
        .and_then(|v| v.as_str())
        .ok_or("Missing 'cwd' argument")?;

    let masday_dir = std::path::Path::new(cwd).join(".masday");

    // Create main directory
    tokio::fs::create_dir(&masday_dir)
        .await
        .map_err(|e| format!("Failed to create .masday directory: {}", e))?;

    // Create subdirectories
    let subdirs = [
        "research",
        "context",
        "plans",
        "notes",
        "state/workflows",
        "state/tasks",
        "reports",
    ];

    for subdir in &subdirs {
        let dir_path = masday_dir.join(subdir);
        tokio::fs::create_dir_all(&dir_path)
            .await
            .map_err(|e| format!("Failed to create directory {}: {}", subdir, e))?;
    }

    Ok(serde_json::json!({
        "initialized": true,
        "path": masday_dir.to_string_lossy().to_string()
    }))
}

/// Validate ID contains only safe characters (alphanumeric, hyphens, underscores).
/// Blocks path traversal characters like `/`, `\`, `..`.
fn sanitize_id(id: &str) -> Result<&str, String> {
    if id.is_empty() {
        return Err("ID cannot be empty".into());
    }
    if !id
        .chars()
        .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
    {
        return Err(format!(
            "Invalid ID '{}': must contain only alphanumeric characters, hyphens, or underscores",
            id
        ));
    }
    Ok(id)
}

/// Sync local state (read from .masday directory)
///
/// Ensures the .masday/state/workflows directory exists and gracefully
/// handles missing workflow state files by returning an empty state.
pub async fn local_sync(args: Value) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    let cwd = args
        .get("cwd")
        .and_then(|v| v.as_str())
        .ok_or("Missing 'cwd' argument")?;

    let workflow_id = args
        .get("workflow_id")
        .and_then(|v| v.as_str())
        .ok_or("Missing 'workflow_id' argument")?;

    let workflow_id =
        sanitize_id(workflow_id).map_err(|e| format!("Invalid workflow_id: {}", e))?;

    let state_dir = std::path::Path::new(cwd)
        .join(".masday")
        .join("state")
        .join("workflows");

    // Ensure directory exists
    tokio::fs::create_dir_all(&state_dir)
        .await
        .map_err(|e| format!("Failed to create state directory: {}", e))?;

    let workflow_file = state_dir.join(format!("{}.json", workflow_id));

    let state = if workflow_file.exists() {
        let content = tokio::fs::read_to_string(&workflow_file)
            .await
            .map_err(|e| format!("Failed to read workflow state: {}", e))?;
        serde_json::from_str(&content)
            .map_err(|e| format!("Failed to parse workflow state JSON: {}", e))?
    } else {
        serde_json::json!(null)
    };

    Ok(serde_json::json!({
        "workflow_id": workflow_id,
        "state": state
    }))
}

/// Push local state to database via API
///
/// Reads workflow and task state from .masday/ directory and pushes to remote API.
/// - If workflow_id is provided: pushes specific workflow state
/// - If workflow_id is null/empty: pushes all workflows found in .masday/state/workflows/
pub async fn local_push(args: Value) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    use crate::client;

    let cwd = args
        .get("cwd")
        .and_then(|v| v.as_str())
        .ok_or("Missing 'cwd' argument")?;

    let workflow_id_opt = args.get("workflow_id").and_then(|v| v.as_str());

    let state_dir = std::path::Path::new(cwd)
        .join(".masday")
        .join("state")
        .join("workflows");

    // Ensure directory exists
    if !state_dir.exists() {
        return Ok(serde_json::json!({
            "pushed": false,
            "error": "State directory does not exist",
            "path": state_dir.to_string_lossy().to_string()
        }));
    }

    let mut pushed_workflows: Vec<String> = Vec::new();
    let mut errors: Vec<serde_json::Value> = Vec::new();

    // Collect workflow files to push
    let workflow_files: Vec<std::path::PathBuf> = if let Some(workflow_id) = workflow_id_opt {
        // Push specific workflow
        let workflow_id =
            sanitize_id(workflow_id).map_err(|e| format!("Invalid workflow_id: {}", e))?;
        let workflow_file = state_dir.join(format!("{}.json", workflow_id));
        if workflow_file.exists() {
            vec![workflow_file]
        } else {
            return Ok(serde_json::json!({
                "pushed": false,
                "error": "Workflow file not found",
                "workflow_id": workflow_id,
                "expected_path": workflow_file.to_string_lossy().to_string()
            }));
        }
    } else {
        // Push all workflows
        let mut entries = tokio::fs::read_dir(&state_dir)
            .await
            .map_err(|e| format!("Failed to read state directory: {}", e))?;

        let mut files = Vec::new();
        while let Some(entry) = entries
            .next_entry()
            .await
            .map_err(|e| format!("Failed to read directory entry: {}", e))?
        {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("json") {
                files.push(path);
            }
        }
        files
    };

    // Push each workflow state
    for workflow_file in workflow_files {
        let file_stem = workflow_file
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown");

        let content = match tokio::fs::read_to_string(&workflow_file).await {
            Ok(c) => c,
            Err(e) => {
                errors.push(serde_json::json!({
                    "workflow_id": file_stem,
                    "error": format!("Failed to read file: {}", e)
                }));
                continue;
            }
        };

        let state: Value = match serde_json::from_str(&content) {
            Ok(s) => s,
            Err(e) => {
                errors.push(serde_json::json!({
                    "workflow_id": file_stem,
                    "error": format!("Failed to parse JSON: {}", e)
                }));
                continue;
            }
        };

        // Extract workflow data
        let workflow_data = state.get("workflow").cloned().unwrap_or(state.clone());
        let workflow_id = workflow_data
            .get("id")
            .or_else(|| workflow_data.get("workflow_id"))
            .and_then(|v| v.as_str())
            .unwrap_or(file_stem);

        // Validate workflow_id is safe
        if sanitize_id(workflow_id).is_err() {
            errors.push(serde_json::json!({
                "workflow_id": file_stem,
                "error": "Invalid workflow_id (contains disallowed characters)"
            }));
            continue;
        }

        // Push workflow state via API (update endpoint)
        match client::api_post(
            &format!("/api/workflows/{}/update", workflow_id),
            workflow_data.clone(),
        )
        .await
        {
            Ok(_) => {
                pushed_workflows.push(workflow_id.to_string());

                // Also push task states if present
                if let Some(tasks) = state.get("tasks").and_then(|v| v.as_array()) {
                    for task in tasks {
                        if let Some(task_id) = task.get("id").and_then(|v| v.as_str()) {
                            if let Some(task_status) = task.get("status") {
                                let task_update = serde_json::json!({
                                    "task_id": task_id,
                                    "status": task_status,
                                    "result": task.get("result"),
                                    "output": task.get("output")
                                });
                                if let Err(e) = client::api_post(
                                    &format!("/api/workflows/{}/complete-task", workflow_id),
                                    task_update,
                                )
                                .await
                                {
                                    errors.push(serde_json::json!({
                                        "workflow_id": workflow_id,
                                        "task_id": task_id,
                                        "error": format!("Failed to push task: {}", e)
                                    }));
                                }
                            }
                        }
                    }
                }
            }
            Err(e) => {
                errors.push(serde_json::json!({
                    "workflow_id": workflow_id,
                    "error": format!("Failed to push workflow: {}", e)
                }));
            }
        }
    }

    Ok(serde_json::json!({
        "pushed": true,
        "workflows_pushed": pushed_workflows,
        "count": pushed_workflows.len(),
        "errors": errors
    }))
}

/// Save artifact to .masday directory
pub async fn local_save_artifact(
    args: Value,
) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    let cwd = args
        .get("cwd")
        .and_then(|v| v.as_str())
        .ok_or("Missing 'cwd' argument")?;

    let category = args
        .get("category")
        .and_then(|v| v.as_str())
        .ok_or("Missing 'category' argument")?;

    let filename = args
        .get("filename")
        .and_then(|v| v.as_str())
        .ok_or("Missing 'filename' argument")?;

    let content = args
        .get("content")
        .and_then(|v| v.as_str())
        .ok_or("Missing 'content' argument")?;

    let artifact_path = std::path::Path::new(cwd)
        .join(".masday")
        .join(category)
        .join(filename);

    // Ensure directory exists
    if let Some(parent) = artifact_path.parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .map_err(|e| format!("Failed to create directory: {}", e))?;
    }

    tokio::fs::write(&artifact_path, content)
        .await
        .map_err(|e| format!("Failed to write artifact: {}", e))?;

    Ok(serde_json::json!({
        "saved": true,
        "path": artifact_path.to_string_lossy().to_string()
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_sanitize_id_valid() {
        assert_eq!(sanitize_id("abc123").unwrap(), "abc123");
        assert_eq!(sanitize_id("test_workflow").unwrap(), "test_workflow");
        assert_eq!(sanitize_id("my-workflow").unwrap(), "my-workflow");
        assert_eq!(sanitize_id("Workflow123").unwrap(), "Workflow123");
    }

    #[test]
    fn test_sanitize_id_invalid() {
        assert!(sanitize_id("").is_err());
        assert!(sanitize_id("abc/123").is_err());
        assert!(sanitize_id("abc..123").is_err());
        assert!(sanitize_id("abc@123").is_err());
        assert!(sanitize_id("abc def").is_err());
    }

    #[tokio::test]
    async fn test_local_init() {
        let temp_dir = tempfile::tempdir().unwrap();
        let cwd = temp_dir.path().to_str().unwrap();

        let args = json!({ "cwd": cwd });
        let result = local_init(args).await;

        assert!(result.is_ok());
        let result_json = result.unwrap();
        assert_eq!(result_json["initialized"], true);
        assert!(result_json["path"].as_str().unwrap().contains(".masday"));

        // Verify directories were created
        let masday_dir = temp_dir.path().join(".masday");
        assert!(masday_dir.exists());
        assert!(masday_dir.join("research").exists());
        assert!(masday_dir.join("context").exists());
        assert!(masday_dir.join("plans").exists());
        assert!(masday_dir.join("notes").exists());
        assert!(masday_dir.join("state").exists());
        assert!(masday_dir.join("state/workflows").exists());
        assert!(masday_dir.join("state/tasks").exists());
        assert!(masday_dir.join("reports").exists());
    }

    #[tokio::test]
    async fn test_local_sync_missing_file() {
        let temp_dir = tempfile::tempdir().unwrap();
        let cwd = temp_dir.path().to_str().unwrap();

        // Create .masday/state/workflows directory
        let state_dir = temp_dir.path().join(".masday/state/workflows");
        tokio::fs::create_dir_all(&state_dir).await.unwrap();

        let args = json!({
            "cwd": cwd,
            "workflow_id": "test_workflow"
        });
        let result = local_sync(args).await;

        assert!(result.is_ok());
        let result_json = result.unwrap();
        assert_eq!(result_json["workflow_id"], "test_workflow");
        assert_eq!(result_json["state"], json!(null));
    }

    #[tokio::test]
    async fn test_local_sync_invalid_id() {
        let temp_dir = tempfile::tempdir().unwrap();
        let cwd = temp_dir.path().to_str().unwrap();

        let args = json!({
            "cwd": cwd,
            "workflow_id": "invalid/workflow"
        });
        let result = local_sync(args).await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_local_push_no_directory() {
        let temp_dir = tempfile::tempdir().unwrap();
        let cwd = temp_dir.path().to_str().unwrap();

        // Don't create .masday directory
        let args = json!({
            "cwd": cwd,
            "workflow_id": "test_workflow"
        });
        let result = local_push(args).await;

        assert!(result.is_ok());
        let result_json = result.unwrap();
        assert_eq!(result_json["pushed"], false);
        assert!(result_json["error"]
            .as_str()
            .unwrap()
            .contains("does not exist"));
    }

    #[tokio::test]
    async fn test_local_push_with_workflow_file() {
        let temp_dir = tempfile::tempdir().unwrap();
        let _cwd = temp_dir.path().to_str().unwrap();

        // Create .masday/state/workflows directory
        let state_dir = temp_dir.path().join(".masday/state/workflows");
        tokio::fs::create_dir_all(&state_dir).await.unwrap();

        // Create a test workflow file
        let workflow_file = state_dir.join("test_workflow.json");
        let workflow_data = json!({
            "workflow": {
                "id": "test_workflow",
                "name": "Test Workflow",
                "status": "EXECUTE"
            }
        });
        tokio::fs::write(&workflow_file, workflow_data.to_string())
            .await
            .unwrap();

        // Test file reading logic without API calls - check that file can be read and parsed
        let content = tokio::fs::read_to_string(&workflow_file).await.unwrap();
        let parsed: Value = serde_json::from_str(&content).unwrap();
        assert_eq!(parsed["workflow"]["id"], "test_workflow");
        assert_eq!(parsed["workflow"]["status"], "EXECUTE");
    }

    #[tokio::test]
    async fn test_local_save_artifact() {
        let temp_dir = tempfile::tempdir().unwrap();
        let cwd = temp_dir.path().to_str().unwrap();

        let args = json!({
            "cwd": cwd,
            "category": "research",
            "filename": "test.md",
            "content": "# Test Content"
        });
        let result = local_save_artifact(args).await;

        assert!(result.is_ok());
        let result_json = result.unwrap();
        assert_eq!(result_json["saved"], true);
        assert!(result_json["path"].as_str().unwrap().contains("test.md"));

        // Verify file was created
        let artifact_path = temp_dir.path().join(".masday/research/test.md");
        assert!(artifact_path.exists());
        let content = tokio::fs::read_to_string(artifact_path).await.unwrap();
        assert_eq!(content, "# Test Content");
    }

    #[tokio::test]
    async fn test_local_save_artifact_missing_args() {
        let temp_dir = tempfile::tempdir().unwrap();
        let cwd = temp_dir.path().to_str().unwrap();

        // Missing category
        let args = json!({
            "cwd": cwd,
            "filename": "test.md",
            "content": "# Test"
        });
        assert!(local_save_artifact(args).await.is_err());

        // Missing filename
        let args = json!({
            "cwd": cwd,
            "category": "research",
            "content": "# Test"
        });
        assert!(local_save_artifact(args).await.is_err());

        // Missing content
        let args = json!({
            "cwd": cwd,
            "category": "research",
            "filename": "test.md"
        });
        assert!(local_save_artifact(args).await.is_err());
    }
}
