//! Filesystem MCP tools - local operations

use serde_json::Value;
use std::path::{Path, PathBuf};

/// Validate and sanitize a file path to prevent path traversal attacks.
/// Rejects paths containing '..' components or paths that resolve outside the project root.
fn validate_path(path: &str, project_root: Option<&Path>) -> Result<PathBuf, String> {
    let path_buf = PathBuf::from(path);

    // Check for path traversal components
    for component in path_buf.components() {
        match component {
            std::path::Component::ParentDir => {
                return Err(format!(
                    "Path traversal detected: '{}' contains '..' component",
                    path
                ));
            }
            std::path::Component::Prefix(_) => {
                // Allow absolute paths on Windows (e.g., C:\)
            }
            _ => {}
        }
    }

    // If project_root is provided, ensure the path resolves within it
    if let Some(root) = project_root {
        let absolute_path = if path_buf.is_absolute() {
            path_buf.clone()
        } else {
            let current_dir = std::env::current_dir()
                .map_err(|e| format!("Failed to get current directory: {}", e))?;
            current_dir.join(&path_buf)
        };

        // Check if the path is within the project root
        match absolute_path.canonicalize() {
            Ok(abs_path) => {
                match root.canonicalize() {
                    Ok(canonical_root) => {
                        if !abs_path.starts_with(&canonical_root) {
                            return Err(format!(
                                "Path '{}' resolves outside project root '{}'",
                                path,
                                canonical_root.display()
                            ));
                        }
                    }
                    Err(_) => {
                        // If we can't canonicalize the root, just check string prefix
                        if !abs_path.starts_with(root) {
                            return Err(format!(
                                "Path '{}' may resolve outside project root '{}'",
                                path,
                                root.display()
                            ));
                        }
                    }
                }
            }
            Err(_) => {
                // Path doesn't exist yet, check parent directory
                if let Some(parent) = absolute_path.parent() {
                    match parent.canonicalize() {
                        Ok(canonical_parent) => match root.canonicalize() {
                            Ok(canonical_root) => {
                                if !canonical_parent.starts_with(&canonical_root) {
                                    return Err(format!(
                                        "Path '{}' parent directory is outside project root '{}'",
                                        path,
                                        canonical_root.display()
                                    ));
                                }
                            }
                            Err(_) => {
                                if !canonical_parent.starts_with(root) {
                                    return Err(format!(
                                            "Path '{}' parent directory may be outside project root '{}'",
                                            path,
                                            root.display()
                                        ));
                                }
                            }
                        },
                        Err(_) => {
                            // Parent doesn't exist, allow it for write operations
                        }
                    }
                }
            }
        }
    }

    Ok(path_buf)
}

/// Read file
pub async fn filesystem_read(
    args: Value,
) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    let path = args
        .get("path")
        .and_then(|v| v.as_str())
        .ok_or("Missing 'path' argument")?;

    let project_root = args
        .get("project_root")
        .and_then(|v| v.as_str())
        .map(Path::new);
    let validated_path =
        validate_path(path, project_root).map_err(|e| format!("Path validation failed: {}", e))?;

    // Check file size before reading to prevent loading huge files into memory
    let metadata = tokio::fs::metadata(&validated_path).await.map_err(|e| {
        format!(
            "Failed to get metadata for {}: {}",
            validated_path.display(),
            e
        )
    })?;

    if metadata.len() > crate::tools::cmd::MAX_OUTPUT_BYTES as u64 {
        return Err(format!(
            "file too large: {} bytes (max {} MiB)",
            metadata.len(),
            crate::tools::cmd::MAX_OUTPUT_BYTES >> 20
        )
        .into());
    }

    let content = tokio::fs::read_to_string(&validated_path)
        .await
        .map_err(|e| format!("Failed to read file {}: {}", validated_path.display(), e))?;

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

    let project_root = args
        .get("project_root")
        .and_then(|v| v.as_str())
        .map(Path::new);
    let validated_path =
        validate_path(path, project_root).map_err(|e| format!("Path validation failed: {}", e))?;

    tokio::fs::write(&validated_path, content)
        .await
        .map_err(|e| format!("Failed to write file {}: {}", validated_path.display(), e))?;

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

    let project_root = args
        .get("project_root")
        .and_then(|v| v.as_str())
        .map(Path::new);
    let validated_path =
        validate_path(path, project_root).map_err(|e| format!("Path validation failed: {}", e))?;

    let mut entries = Vec::new();
    let mut dir = tokio::fs::read_dir(&validated_path).await.map_err(|e| {
        format!(
            "Failed to read directory {}: {}",
            validated_path.display(),
            e
        )
    })?;

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

    let project_root = args
        .get("project_root")
        .and_then(|v| v.as_str())
        .map(Path::new);
    let validated_path =
        validate_path(path, project_root).map_err(|e| format!("Path validation failed: {}", e))?;

    tokio::fs::remove_file(&validated_path)
        .await
        .map_err(|e| format!("Failed to delete file {}: {}", validated_path.display(), e))?;

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

    let project_root = args
        .get("project_root")
        .and_then(|v| v.as_str())
        .map(Path::new);
    let validated_path =
        validate_path(path, project_root).map_err(|e| format!("Path validation failed: {}", e))?;

    let metadata = tokio::fs::metadata(&validated_path).await.map_err(|e| {
        format!(
            "Failed to get metadata for {}: {}",
            validated_path.display(),
            e
        )
    })?;

    let modified = metadata
        .modified()
        .map_err(|e| format!("Failed to get modified time: {}", e))?
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| format!("Time error: {}", e))?
        .as_secs();

    let file_type = if metadata.is_dir() {
        "directory"
    } else {
        "file"
    };

    Ok(serde_json::json!({
        "path": validated_path.display().to_string(),
        "type": file_type,
        "size": metadata.len(),
        "modified_unix_secs": modified,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_path_rejects_traversal() {
        let result = validate_path("../etc/passwd", None);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("traversal"));
    }

    #[test]
    fn test_validate_path_rejects_nested_traversal() {
        let result = validate_path("foo/../../etc/passwd", None);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_path_accepts_normal() {
        let result = validate_path("src/main.rs", None);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_path_accepts_absolute() {
        let result = validate_path("/tmp/test.txt", None);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_path_enforces_project_root() {
        // Use actual current directory as project root for realistic test
        let cwd = std::env::current_dir().expect("current dir");
        // Path inside project should be accepted
        let result = validate_path("src/file.rs", Some(cwd.as_path()));
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_path_rejects_outside_project_root() {
        let root = Path::new("/home/user/project");
        // Path traversal with .. should be rejected regardless
        let result = validate_path("../../../etc/passwd", Some(root));
        assert!(result.is_err());
    }
}
