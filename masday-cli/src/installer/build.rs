//! Build orchestration for masday crates
//!
//! Provides cargo build commands and binary resolution for local mode.

use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use std::process::Command;

/// Build the masday CLI binary in release mode
pub fn build_crates(project_dir: &Path) -> Result<()> {
    println!(
        "{}",
        console::style("Building masday binary...").cyan()
    );

    let status = Command::new("cargo")
        .arg("build")
        .arg("-p")
        .arg("masday-cli")
        .arg("--release")
        .current_dir(project_dir)
        .status()
        .context("Failed to execute cargo build command")?;

    if !status.success() {
        anyhow::bail!("Cargo build failed with exit code: {:?}", status);
    }

    println!("{}", console::style("Build complete!").green());
    Ok(())
}

/// Find the masday binary in target/release or target/debug
pub fn find_mcp_binary(project_dir: &Path) -> Result<PathBuf> {
    let release_path = project_dir.join("target/release/masday");
    let debug_path = project_dir.join("target/debug/masday");

    if release_path.exists() {
        Ok(release_path)
    } else if debug_path.exists() {
        Ok(debug_path)
    } else {
        anyhow::bail!(
            "masday binary not found. Checked:\n  - {}\n  - {}\nRun: cargo build -p masday-cli --release",
            release_path.display(),
            debug_path.display()
        )
    }
}

/// Find the masday binary in target/release or target/debug (same binary serves as API server)
pub fn find_api_binary(project_dir: &Path) -> Result<PathBuf> {
    find_mcp_binary(project_dir)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_find_mcp_binary_missing() {
        let temp_dir = TempDir::new().unwrap();
        let project_dir = temp_dir.path();

        let result = find_mcp_binary(project_dir);
        assert!(result.is_err());
    }

    #[test]
    fn test_find_api_binary_missing() {
        let temp_dir = TempDir::new().unwrap();
        let project_dir = temp_dir.path();

        let result = find_api_binary(project_dir);
        assert!(result.is_err());
    }

    #[test]
    fn test_find_mcp_binary_release() {
        let temp_dir = TempDir::new().unwrap();
        let project_dir = temp_dir.path();
        let target_dir = project_dir.join("target/release");
        std::fs::create_dir_all(&target_dir).unwrap();
        std::fs::write(target_dir.join("masday"), "binary").unwrap();

        let result = find_mcp_binary(project_dir).unwrap();
        assert!(result.ends_with("target/release/masday"));
    }

    #[test]
    fn test_find_mcp_binary_debug() {
        let temp_dir = TempDir::new().unwrap();
        let project_dir = temp_dir.path();
        let target_dir = project_dir.join("target/debug");
        std::fs::create_dir_all(&target_dir).unwrap();
        std::fs::write(target_dir.join("masday"), "binary").unwrap();

        let result = find_mcp_binary(project_dir).unwrap();
        assert!(result.ends_with("target/debug/masday"));
    }

    #[test]
    fn test_find_api_binary_release() {
        let temp_dir = TempDir::new().unwrap();
        let project_dir = temp_dir.path();
        let target_dir = project_dir.join("target/release");
        std::fs::create_dir_all(&target_dir).unwrap();
        std::fs::write(target_dir.join("masday"), "binary").unwrap();

        let result = find_api_binary(project_dir).unwrap();
        assert!(result.ends_with("target/release/masday"));
    }

    #[test]
    fn test_find_mcp_binary_prefers_release() {
        let temp_dir = TempDir::new().unwrap();
        let project_dir = temp_dir.path();
        let release_dir = project_dir.join("target/release");
        let debug_dir = project_dir.join("target/debug");
        std::fs::create_dir_all(&release_dir).unwrap();
        std::fs::create_dir_all(&debug_dir).unwrap();
        std::fs::write(release_dir.join("masday"), "release").unwrap();
        std::fs::write(debug_dir.join("masday"), "debug").unwrap();

        let result = find_mcp_binary(project_dir).unwrap();
        assert!(result.ends_with("target/release/masday"));
    }
}
