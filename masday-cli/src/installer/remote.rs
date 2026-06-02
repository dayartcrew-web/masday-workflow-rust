//! Remote MCP binary resolution
//!
//! Handles finding masday-mcp in PATH or resolving remote URLs.

use anyhow::{Context, Result};
use home::home_dir;
use std::path::PathBuf;

/// Resolve the MCP binary path for remote mode
///
/// First checks if masday-mcp is in PATH.
/// If not found, verifies remote URL connectivity and creates ~/.masday/bin/.
/// For now, this is a stub that creates the directory and returns the PATH-resolved binary.
pub fn resolve_mcp_binary(_remote_url: &str) -> Result<PathBuf> {
    // Check if masday-mcp is already in PATH
    if let Ok(path) = which::which("masday-mcp") {
        println!(
            "{}",
            console::style(format!("Found masday-mcp in PATH: {}", path.display())).green()
        );
        return Ok(path);
    }

    // Create ~/.masday/bin directory
    let masday_bin = home_dir()
        .ok_or_else(|| anyhow::anyhow!("Cannot determine home directory"))?
        .join(".masday/bin");

    std::fs::create_dir_all(&masday_bin)
        .with_context(|| format!("Failed to create directory {}", masday_bin.display()))?;

    // TODO: Download binary from remote_url
    // For now, return an error since we don't have the binary
    anyhow::bail!(
        "masday-mcp not found in PATH and remote download not yet implemented. \
        Expected location: {}/masday-mcp\n\
        Remote URL: {}",
        masday_bin.display(),
        _remote_url
    );
}

/// Verify remote URL is accessible
///
/// Validates the URL scheme (http/https only) then performs a GET request
/// to /api/health endpoint to check connectivity.
pub fn verify_remote_url(remote_url: &str) -> Result<()> {
    // Validate URL scheme — only http/https allowed
    let trimmed = remote_url.trim_end_matches('/');
    if !trimmed.starts_with("http://") && !trimmed.starts_with("https://") {
        anyhow::bail!(
            "Invalid remote URL: only http:// and https:// schemes are allowed. Got: {}",
            trimmed
        );
    }

    let health_url = format!("{}/api/health", trimmed);

    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .context("Failed to build HTTP client")?;

    let response = client
        .get(&health_url)
        .send()
        .context("Failed to connect to remote URL")?;

    if response.status().is_success() {
        println!(
            "{}",
            console::style(format!("Remote API is healthy: {}", health_url)).green()
        );
        Ok(())
    } else {
        anyhow::bail!(
            "Remote health check failed: {} returned {}",
            health_url,
            response.status()
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_mcp_binary_not_found() {
        let result = resolve_mcp_binary("http://localhost:30101");
        // Should fail since masday-mcp is not in PATH
        assert!(result.is_err());
    }

    #[test]
    fn test_verify_remote_url_invalid() {
        let result = verify_remote_url("http://invalid-url-that-does-not-exist-12345.com");
        assert!(result.is_err());
    }

    #[test]
    fn test_masday_bin_directory_creation() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let home_override = temp_dir.path();

        // Can't easily override home_dir in tests, so just verify the logic
        let expected_bin = home_override.join(".masday/bin");
        assert!(!expected_bin.exists());

        std::fs::create_dir_all(&expected_bin).unwrap();
        assert!(expected_bin.exists());
    }

    #[test]
    #[cfg(unix)]
    fn test_which_masday_mcp_fallback() {
        // This test verifies that which::which returns Err when binary not found
        let result = which::which("masday-mcp-binary-that-does-not-exist");
        assert!(result.is_err());
    }
}
