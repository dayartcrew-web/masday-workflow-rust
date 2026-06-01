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

/// Push local state to database (placeholder for Phase 3.3)
pub async fn local_push(args: Value) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    let _cwd = args
        .get("cwd")
        .and_then(|v| v.as_str())
        .ok_or("Missing 'cwd' argument")?;

    let workflow_id = args
        .get("workflow_id")
        .and_then(|v| v.as_str())
        .ok_or("Missing 'workflow_id' argument")?;

    // Placeholder: In Phase 3.3, this will push to PostgreSQL via masday-api
    // For now, just acknowledge the request
    Ok(serde_json::json!({
        "pushed": true,
        "workflow_id": workflow_id,
        "note": "Placeholder - will be implemented in Phase 3.3"
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
