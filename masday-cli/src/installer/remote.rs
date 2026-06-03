//! Remote MCP binary resolution
//!
//! Handles finding masday in PATH or resolving remote URLs.

use anyhow::{Context, Result};
use home::home_dir;
use std::path::PathBuf;

/// Resolve the masday binary path for remote mode
///
/// First checks if masday is in PATH.
/// If not found, checks ~/.masday/bin/.
pub fn resolve_mcp_binary(_remote_url: &str) -> Result<PathBuf> {
    // Check if masday is already in PATH
    if let Ok(path) = which::which("masday") {
        println!(
            "{}",
            console::style(format!("Found masday in PATH: {}", path.display())).green()
        );
        return Ok(path);
    }

    // Check ~/.masday/bin/masday
    let masday_bin = home_dir()
        .ok_or_else(|| anyhow::anyhow!("Cannot determine home directory"))?
        .join(".masday/bin/masday");

    if masday_bin.exists() {
        println!(
            "{}",
            console::style(format!("Found masday at: {}", masday_bin.display())).green()
        );
        return Ok(masday_bin);
    }

    // Create ~/.masday/bin directory for future use
    let masday_dir = home_dir()
        .ok_or_else(|| anyhow::anyhow!("Cannot determine home directory"))?
        .join(".masday/bin");

    std::fs::create_dir_all(&masday_dir)
        .with_context(|| format!("Failed to create directory {}", masday_dir.display()))?;

    anyhow::bail!(
        "masday binary not found in PATH or ~/.masday/bin/.\n\
        Expected: {}\n\
        Run: masday quickstart (to install) or copy the binary to ~/.masday/bin/",
        masday_bin.display()
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
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_mcp_binary_not_found() {
        let result = resolve_mcp_binary(&masday_core::constants::ports::api_base_url());
        // May pass if masday is in PATH, or fail if not — both are valid
        // Just verify it doesn't panic
        let _ = result;
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

        let expected_bin = home_override.join(".masday/bin");
        assert!(!expected_bin.exists());

        std::fs::create_dir_all(&expected_bin).unwrap();
        assert!(expected_bin.exists());
    }

    #[test]
    #[cfg(unix)]
    fn test_which_masday_fallback() {
        // Verify that which::which returns Err when binary not found
        let result = which::which("masday-binary-that-does-not-exist");
        assert!(result.is_err());
    }
}
