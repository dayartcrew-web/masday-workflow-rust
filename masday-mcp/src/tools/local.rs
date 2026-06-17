//! Local MCP tools - file-based state operations

use crate::embedding;
use reqwest::Client;
use serde_json::Value;
use tracing::{info, warn};

/// Initialize local state directory
pub async fn local_init(args: Value) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    let cwd = args
        .get("cwd")
        .and_then(|v| v.as_str())
        .ok_or("Missing 'cwd' argument")?;

    let masday_dir = std::path::Path::new(cwd).join(".masday");

    // Create main directory (idempotent — ok if already exists)
    tokio::fs::create_dir_all(&masday_dir)
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

/// Generate embedding vector for text content.
///
/// Supports three providers (configured via EMBEDDING_PROVIDER env var):
/// - "mock": Use feature hashing vectorizer (default, zero dependencies)
/// - "ollama": HTTP POST to Ollama API with nomic-embed-text model
/// - "openai": HTTP POST to OpenAI API with text-embedding-3-small model
///
/// Returns Result with vector or error message.
async fn generate_embedding(text: &str) -> Result<Vec<f64>, String> {
    // Read provider from ~/.masday/config.toml directly (production: no env vars).
    let provider = crate::pg::read_embedding_provider()
        .unwrap_or_else(|| "mock".to_string())
        .to_lowercase();

    match provider.as_str() {
        "mock" => {
            // Use existing feature hashing vectorizer
            let vector = embedding::text_to_vector(text);
            // Convert f32 to f64 for API compatibility
            Ok(vector.into_iter().map(|v| v as f64).collect())
        }
        "ollama" => {
            // Call Ollama API (config from ~/.masday/config.toml — no env)
            let base_url = crate::pg::read_embedding_base_url()
                .unwrap_or_else(|| "http://localhost:11434".to_string());
            let model = crate::pg::read_embedding_model()
                .unwrap_or_else(|| "nomic-embed-text".to_string());
            let url = format!("{}/api/embeddings", base_url.trim_end_matches('/'));

            // Timeout prevents indefinite hang when Ollama is down/unreachable.
            let client = Client::builder()
                .timeout(std::time::Duration::from_secs(15))
                .build()
                .map_err(|e| format!("Failed to build HTTP client: {}", e))?;
            let payload = serde_json::json!({
                "model": model,
                "input": text
            });

            let response = client
                .post(&url)
                .json(&payload)
                .send()
                .await
                .map_err(|e| format!("Ollama embedding request failed: {}", e))?;

            if !response.status().is_success() {
                return Err(format!(
                    "Ollama embedding request failed: {}",
                    response.status()
                ));
            }

            let result = response
                .json::<serde_json::Value>()
                .await
                .map_err(|e| format!("Failed to parse Ollama response: {}", e))?;

            result
                .get("embedding")
                .and_then(|v| v.as_array())
                .map(|embedding| {
                    let vector: Vec<f64> = embedding.iter().filter_map(|v| v.as_f64()).collect();
                    info!(
                        "Generated Ollama embedding with {} dimensions",
                        vector.len()
                    );
                    vector
                })
                .ok_or_else(|| "Invalid Ollama embedding response format".to_string())
        }
        "openai" => {
            // Call OpenAI API (config from ~/.masday/config.toml — no env)
            let base_url = crate::pg::read_embedding_base_url()
                .unwrap_or_else(|| "https://api.openai.com".to_string());
            let model = crate::pg::read_embedding_model()
                .unwrap_or_else(|| "text-embedding-3-small".to_string());
            let api_key = crate::pg::read_config_value("embedding_api_key")
                .or_else(|| crate::pg::read_config_value("openai_api_key"))
                .ok_or_else(|| {
                    "OpenAI embedding key not set: add embedding_api_key/openai_api_key to ~/.masday/config.toml"
                        .to_string()
                })?;
            let url = format!("{}/v1/embeddings", base_url.trim_end_matches('/'));

            let client = Client::builder()
                .timeout(std::time::Duration::from_secs(15))
                .build()
                .map_err(|e| format!("Failed to build HTTP client: {}", e))?;
            let payload = serde_json::json!({
                "model": model,
                "input": text
            });

            let response = client
                .post(&url)
                .header("Authorization", format!("Bearer {}", api_key))
                .json(&payload)
                .send()
                .await
                .map_err(|e| format!("OpenAI embedding request failed: {}", e))?;

            if !response.status().is_success() {
                return Err(format!(
                    "OpenAI embedding request failed: {}",
                    response.status()
                ));
            }

            let result = response
                .json::<serde_json::Value>()
                .await
                .map_err(|e| format!("Failed to parse OpenAI response: {}", e))?;

            result
                .get("data")
                .and_then(|v| v.as_array())
                .and_then(|data| data.first())
                .and_then(|v| v.get("embedding"))
                .and_then(|v| v.as_array())
                .map(|embedding| {
                    let vector: Vec<f64> = embedding.iter().filter_map(|v| v.as_f64()).collect();
                    info!(
                        "Generated OpenAI embedding with {} dimensions",
                        vector.len()
                    );
                    vector
                })
                .ok_or_else(|| "Invalid OpenAI embedding response format".to_string())
        }
        _ => {
            warn!(
                "Unknown embedding provider: {}, defaulting to mock",
                provider
            );
            let vector = embedding::text_to_vector(text);
            Ok(vector.into_iter().map(|v| v as f64).collect())
        }
    }
}

/// Sync local state from API (HTTP mode)
///
/// Pulls workflow and task state from the remote API and writes it to
/// .masday/state/workflows/{id}.json. This is the HTTP mode version that
/// uses client::api_get() to fetch data from the API server.
pub async fn local_sync(args: Value) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    use crate::client;

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

    // Query API for workflow data
    let workflow_data = client::api_get(&format!("/api/workflows/{}", workflow_id))
        .await
        .map_err(|e| format!("Failed to fetch workflow from API: {}", e))?;

    // Query API for task data
    let tasks_data = client::api_get(&format!("/api/workflows/{}/tasks", workflow_id))
        .await
        .map_err(|e| format!("Failed to fetch tasks from API: {}", e))?;

    // Build state object - API returns tasks as array directly or wrapped in "tasks" field
    let tasks = if tasks_data.is_array() {
        tasks_data.as_array().cloned().unwrap_or_default()
    } else {
        tasks_data
            .get("tasks")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default()
    };

    let state = serde_json::json!({
        "workflow": workflow_data,
        "tasks": tasks,
        "syncedAt": chrono::Utc::now().to_rfc3339()
    });

    // Write to file
    let workflow_file = state_dir.join(format!("{}.json", workflow_id));
    let state_json = serde_json::to_string_pretty(&state)
        .map_err(|e| format!("Failed to serialize state: {}", e))?;

    tokio::fs::write(&workflow_file, state_json)
        .await
        .map_err(|e| format!("Failed to write workflow state file: {}", e))?;

    Ok(state)
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

    let total_files = workflow_files.len();

    // Push each workflow state
    for (idx, workflow_file) in workflow_files.iter().enumerate() {
        let file_stem = workflow_file
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown");

        info!(
            "local_push [{}/{}] processing {}",
            idx + 1,
            total_files,
            file_stem
        );

        let content = match tokio::fs::read_to_string(workflow_file).await {
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

        // Push workflow state via API with per-workflow timeout
        let push_result = tokio::time::timeout(
            std::time::Duration::from_secs(10),
            client::api_post(
                &format!("/api/workflows/{}/update", workflow_id),
                workflow_data.clone(),
            ),
        )
        .await;

        match push_result {
            Ok(Ok(_)) => {
                pushed_workflows.push(workflow_id.to_string());

                // Also push task states if present
                if let Some(tasks) = state.get("tasks").and_then(|v| v.as_array()) {
                    for task in tasks {
                        if let Some(task_id) = task.get("id").and_then(|v| v.as_str()) {
                            if let Some(task_status) = task.get("status") {
                                let mut task_update = serde_json::json!({
                                    "task_id": task_id,
                                    "status": task_status,
                                    "result": task.get("result"),
                                    "output": task.get("output")
                                });

                                // Generate embedding for task output/result content (with timeout)
                                let embedding_text = {
                                    let output =
                                        task.get("output").and_then(|v| v.as_str()).unwrap_or("");
                                    let result =
                                        task.get("result").and_then(|v| v.as_str()).unwrap_or("");
                                    format!("{} {}", output, result)
                                };

                                if !embedding_text.trim().is_empty() {
                                    match tokio::time::timeout(
                                        std::time::Duration::from_secs(5),
                                        generate_embedding(&embedding_text),
                                    )
                                    .await
                                    {
                                        Ok(Ok(embedding)) => {
                                            task_update["embedding"] = serde_json::json!(embedding);
                                            info!(
                                                "Generated embedding for task {}: {} dimensions",
                                                task_id,
                                                embedding.len()
                                            );
                                        }
                                        Ok(Err(e)) => {
                                            warn!(
                                                "Embedding generation failed for task {}: {}",
                                                task_id, e
                                            );
                                            // Continue without embedding
                                        }
                                        Err(_) => {
                                            warn!(
                                                "Embedding generation timed out for task {}",
                                                task_id
                                            );
                                            // Continue without embedding
                                        }
                                    }
                                }

                                // Push task with timeout
                                let task_result = tokio::time::timeout(
                                    std::time::Duration::from_secs(10),
                                    client::api_post(
                                        &format!("/api/workflows/{}/complete-task", workflow_id),
                                        task_update,
                                    ),
                                )
                                .await;

                                match task_result {
                                    Ok(Ok(_)) => {}
                                    Ok(Err(e)) => {
                                        errors.push(serde_json::json!({
                                            "workflow_id": workflow_id,
                                            "task_id": task_id,
                                            "error": format!("Failed to push task: {}", e)
                                        }));
                                    }
                                    Err(_) => {
                                        errors.push(serde_json::json!({
                                            "workflow_id": workflow_id,
                                            "task_id": task_id,
                                            "error": "Task push timed out after 10s"
                                        }));
                                    }
                                }
                            }
                        }
                    }
                }
            }
            Ok(Err(e)) => {
                errors.push(serde_json::json!({
                    "workflow_id": workflow_id,
                    "error": format!("Failed to push workflow: {}", e)
                }));
            }
            Err(_) => {
                errors.push(serde_json::json!({
                    "workflow_id": workflow_id,
                    "error": "Workflow push timed out after 10s"
                }));
            }
        }
    }

    let pushed = !pushed_workflows.is_empty();
    Ok(serde_json::json!({
        "pushed": pushed,
        "workflows_pushed": pushed_workflows,
        "count": pushed_workflows.len(),
        "skipped": total_files - pushed_workflows.len() - errors.len(),
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
    async fn test_local_sync_invalid_id() {
        let temp_dir = tempfile::tempdir().unwrap();
        let cwd = temp_dir.path().to_str().unwrap();

        let args = json!({
            "cwd": cwd,
            "workflow_id": "invalid/workflow"
        });
        let result = local_sync(args).await;

        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Invalid workflow_id"));
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

    #[tokio::test]
    async fn test_generate_embedding_mock() {
        // Ensure clean state before test
        std::env::remove_var("OPENAI_API_KEY");
        std::env::set_var("EMBEDDING_PROVIDER", "mock");
        let text = "Hello world test";
        let embedding = generate_embedding(text).await;

        assert!(embedding.is_ok());
        let vector = embedding.unwrap();
        assert_eq!(vector.len(), 768); // Feature hashing produces 768-dim vectors
                                       // Check that values are normalized (unit vector)
        let norm_sq: f64 = vector.iter().map(|&x| x * x).sum();
        let norm = norm_sq.sqrt();
        assert!((norm - 1.0).abs() < 0.01, "Vector should be normalized");
        // Cleanup
        std::env::remove_var("EMBEDDING_PROVIDER");
    }

    #[tokio::test]
    async fn test_generate_embedding_empty_text() {
        // Ensure clean state before test
        std::env::remove_var("OPENAI_API_KEY");
        std::env::set_var("EMBEDDING_PROVIDER", "mock");
        let text = "";
        let embedding = generate_embedding(text).await;

        assert!(embedding.is_ok());
        let vector = embedding.unwrap();
        assert_eq!(vector.len(), 768);
        // Empty text produces zero vector
        let norm_sq: f64 = vector.iter().map(|&x| x * x).sum();
        assert_eq!(norm_sq, 0.0);
        // Cleanup
        std::env::remove_var("EMBEDDING_PROVIDER");
    }

    #[tokio::test]
    async fn test_generate_embedding_deterministic() {
        // Ensure clean state
        std::env::remove_var("OPENAI_API_KEY");
        std::env::set_var("EMBEDDING_PROVIDER", "mock");
        let text = "deterministic test";
        let embedding1 = generate_embedding(text).await;
        let embedding2 = generate_embedding(text).await;

        assert!(embedding1.is_ok());
        assert!(embedding2.is_ok());
        let vec1 = embedding1.unwrap();
        let vec2 = embedding2.unwrap();
        assert_eq!(vec1.len(), vec2.len());
        // Check vectors are identical
        for (i, (v1, v2)) in vec1.iter().zip(vec2.iter()).enumerate() {
            assert_eq!(v1, v2, "Vectors differ at index {}", i);
        }
    }

    #[tokio::test]
    async fn test_generate_embedding_unknown_provider_fallback() {
        // Ensure clean state before test
        std::env::remove_var("OPENAI_API_KEY");
        std::env::set_var("EMBEDDING_PROVIDER", "unknown_provider");
        let text = "fallback test";
        let embedding = generate_embedding(text).await;

        // Should fallback to mock
        assert!(embedding.is_ok());
        let vector = embedding.unwrap();
        assert_eq!(vector.len(), 768);
        // Cleanup
        std::env::remove_var("EMBEDDING_PROVIDER");
    }

    #[tokio::test]
    async fn test_generate_embedding_openai_missing_key() {
        // Ensure clean state before test
        std::env::remove_var("OPENAI_API_KEY");
        std::env::set_var("EMBEDDING_PROVIDER", "openai");
        let text = "test";
        let embedding = generate_embedding(text).await;

        // Should fail with missing API key error
        assert!(embedding.is_err());
        assert!(embedding.unwrap_err().contains("OPENAI_API_KEY"));
        // Cleanup
        std::env::remove_var("EMBEDDING_PROVIDER");
    }
}
