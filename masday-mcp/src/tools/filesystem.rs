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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn test_filesystem_read_write() {
        let temp_dir = tempfile::tempdir().unwrap();
        let test_file = temp_dir.path().join("test.txt");
        let path = test_file.to_str().unwrap();

        // Write file
        let write_args = json!({
            "path": path,
            "content": "Hello, World!"
        });
        let write_result = filesystem_write(write_args).await;
        assert!(write_result.is_ok());
        assert_eq!(write_result.unwrap()["written"], true);

        // Read file
        let read_args = json!({ "path": path });
        let read_result = filesystem_read(read_args).await;
        assert!(read_result.is_ok());
        assert_eq!(read_result.unwrap()["content"], "Hello, World!");
    }

    #[tokio::test]
    async fn test_filesystem_read_missing_path() {
        let args = json!({});
        let result = filesystem_read(args).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_filesystem_write_missing_args() {
        let temp_dir = tempfile::tempdir().unwrap();
        let test_file = temp_dir.path().join("test.txt");
        let path = test_file.to_str().unwrap();

        // Missing content
        let args = json!({ "path": path });
        let result = filesystem_write(args).await;
        assert!(result.is_err());

        // Missing path
        let args = json!({ "content": "test" });
        let result = filesystem_write(args).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_filesystem_list() {
        let temp_dir = tempfile::tempdir().unwrap();
        let path = temp_dir.path().to_str().unwrap();

        // Create test files
        tokio::fs::write(temp_dir.path().join("file1.txt"), "content1")
            .await
            .unwrap();
        tokio::fs::create_dir(temp_dir.path().join("subdir"))
            .await
            .unwrap();

        let args = json!({ "path": path });
        let result = filesystem_list(args).await;
        assert!(result.is_ok());
        let result_json = result.unwrap();
        let entries = result_json["entries"].as_array().unwrap();
        assert_eq!(entries.len(), 2);
        drop(result_json); // Explicitly drop to fix borrow issue
    }

    #[tokio::test]
    async fn test_filesystem_delete() {
        let temp_dir = tempfile::tempdir().unwrap();
        let test_file = temp_dir.path().join("test.txt");
        tokio::fs::write(&test_file, "content").await.unwrap();

        let args = json!({ "path": test_file.to_str().unwrap() });
        let result = filesystem_delete(args).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap()["deleted"], true);
        assert!(!test_file.exists());
    }

    #[tokio::test]
    async fn test_filesystem_stat() {
        let temp_dir = tempfile::tempdir().unwrap();
        let test_file = temp_dir.path().join("test.txt");
        tokio::fs::write(&test_file, "test content").await.unwrap();

        let args = json!({ "path": test_file.to_str().unwrap() });
        let result = filesystem_stat(args).await;
        assert!(result.is_ok());
        let stat = result.unwrap();
        assert_eq!(stat["is_file"], true);
        assert_eq!(stat["is_dir"], false);
        assert!(stat["size"].as_u64().unwrap() > 0);
    }

    #[tokio::test]
    async fn test_filesystem_stat_directory() {
        let temp_dir = tempfile::tempdir().unwrap();
        let path = temp_dir.path().to_str().unwrap();

        let args = json!({ "path": path });
        let result = filesystem_stat(args).await;
        assert!(result.is_ok());
        let stat = result.unwrap();
        assert_eq!(stat["is_file"], false);
        assert_eq!(stat["is_dir"], true);
    }
}
