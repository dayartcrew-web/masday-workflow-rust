//! Filesystem MCP tools - local operations

use serde_json::Value;

/// Read file
pub async fn filesystem_read(
    args: Value,
) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    let path = args
        .get("path")
        .and_then(|v| v.as_str())
        .ok_or("Missing 'path' argument")?;

    let content = tokio::fs::read_to_string(path)
        .await
        .map_err(|e| format!("Failed to read file {}: {}", path, e))?;

    Ok(serde_json::json!({ "content": content }))
}

/// Write file
pub async fn filesystem_write(
    args: Value,
) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    let path = args
        .get("path")
        .and_then(|v| v.as_str())
        .ok_or("Missing 'path' argument")?;

    let content = args
        .get("content")
        .and_then(|v| v.as_str())
        .ok_or("Missing 'content' argument")?;

    tokio::fs::write(path, content)
        .await
        .map_err(|e| format!("Failed to write file {}: {}", path, e))?;

    Ok(serde_json::json!({ "written": true }))
}

/// List directory
pub async fn filesystem_list(
    args: Value,
) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    let path = args
        .get("path")
        .and_then(|v| v.as_str())
        .ok_or("Missing 'path' argument")?;

    let mut entries = Vec::new();
    let mut dir = tokio::fs::read_dir(path)
        .await
        .map_err(|e| format!("Failed to read directory {}: {}", path, e))?;

    while let Some(entry) = dir
        .next_entry()
        .await
        .map_err(|e| format!("Failed to read entry: {}", e))?
    {
        let name = entry.file_name().to_string_lossy().to_string();
        let metadata = entry
            .metadata()
            .await
            .map_err(|e| format!("Failed to get metadata: {}", e))?;
        entries.push(serde_json::json!({
            "name": name,
            "is_file": metadata.is_file(),
            "is_dir": metadata.is_dir(),
        }));
    }

    Ok(serde_json::json!({ "entries": entries }))
}

/// Delete file
pub async fn filesystem_delete(
    args: Value,
) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    let path = args
        .get("path")
        .and_then(|v| v.as_str())
        .ok_or("Missing 'path' argument")?;

    tokio::fs::remove_file(path)
        .await
        .map_err(|e| format!("Failed to delete file {}: {}", path, e))?;

    Ok(serde_json::json!({ "deleted": true }))
}

/// Get file stat
pub async fn filesystem_stat(
    args: Value,
) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    let path = args
        .get("path")
        .and_then(|v| v.as_str())
        .ok_or("Missing 'path' argument")?;

    let metadata = tokio::fs::metadata(path)
        .await
        .map_err(|e| format!("Failed to get metadata for {}: {}", path, e))?;

    let modified = metadata
        .modified()
        .map_err(|e| format!("Failed to get modified time: {}", e))?
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| format!("Failed to convert timestamp: {}", e))?
        .as_secs();

    Ok(serde_json::json!({
        "size": metadata.len(),
        "is_file": metadata.is_file(),
        "is_dir": metadata.is_dir(),
        "modified": modified,
    }))
}
