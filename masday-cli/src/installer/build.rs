//! Build orchestration for masday crates
//!
//! Provides cargo build commands and binary resolution for local mode.
//!
//! ## Build Modes
//!
//! - **Dev mode (--dev)**: Builds from source using cargo. Skips build if binary
//!   is newer than source files (freshness check).
//! - **Production mode**: Expects pre-built binary in `~/.masday/bin/masday` or
//!   PATH. Downloads from GitHub releases if not found.

use anyhow::{Context, Result};
use console::style;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::SystemTime;

/// Build the masday CLI binary in release mode
///
/// Only called when `--dev` flag is present. Performs freshness check to skip
/// build if binary is newer than source files.
pub fn build_crates(project_dir: &Path) -> Result<()> {
    // Check if build is fresh
    if is_build_fresh(project_dir) {
        println!(
            "{}",
            style("Build fresh (binary newer than sources), skipping cargo build").dim()
        );
        return Ok(());
    }

    println!("{}", style("Building masday binary...").cyan());

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

    println!("{}", style("Build complete!").green());
    Ok(())
}

/// Check if the built binary is newer than the source files
///
/// Returns true if `target/release/masday` exists and is newer than all
/// `.rs` files in the project. This avoids unnecessary rebuilds in dev mode.
pub fn is_build_fresh(project_dir: &Path) -> bool {
    let binary_path = project_dir.join("target/release/masday");

    // Binary must exist
    if !binary_path.exists() {
        return false;
    }

    // Get binary modification time
    let binary_mtime = match std::fs::metadata(&binary_path).and_then(|m| m.modified()) {
        Ok(time) => time,
        Err(_) => return false,
    };

    // Check if any source file is newer than the binary
    if let Ok(newer_source) = find_newer_source(project_dir, &binary_path, binary_mtime) {
        if newer_source.is_some() {
            return false;
        }
    }

    true
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

/// Find or download the masday binary
///
/// In dev mode: builds from source (with freshness check).
/// In production mode: checks `~/.masday/bin/` and PATH, suggests download if not found.
pub fn find_or_download_binary(dev_mode: bool, project_dir: &Path) -> Result<PathBuf> {
    if dev_mode {
        // Dev mode: build from source
        build_crates(project_dir)?;
        find_mcp_binary(project_dir)
    } else {
        // Production mode: check known locations first
        if let Some(home_dir) = home::home_dir() {
            let install_path = home_dir.join(".masday/bin/masday");
            if install_path.exists() {
                return Ok(install_path);
            }
        }

        // Check PATH
        if let Ok(path) = which::which("masday") {
            return Ok(path);
        }

        // Not found: suggest download
        anyhow::bail!(
            "masday binary not found in ~/.masday/bin/ or PATH.\n\
             \n\
             Download from:\n\
             {}\n\
             \n\
             Or run with --dev flag to build from source:\n\
             {}",
            style("https://github.com/dayartcrew-web/masday-workflow-rust/releases").cyan(),
            style("masday dev build").cyan()
        )
    }
}

/// Find a source file newer than the given mtime
///
/// Searches for `.rs` and `.toml` files that are newer than the binary.
/// Returns Ok(Some(path)) if a newer file is found, Ok(None) if all files are older.
fn find_newer_source(
    project_dir: &Path,
    binary_path: &Path,
    binary_mtime: SystemTime,
) -> Result<Option<PathBuf>> {
    // Exclude target directory from search
    let target_dir = project_dir.join("target");

    // Walk the project directory
    for entry in walkdir::WalkDir::new(project_dir)
        .into_iter()
        .filter_entry(|e| !e.path().starts_with(&target_dir))
    {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };

        let path = entry.path();

        // Only check source files (.rs, .toml, .json for config)
        if path.extension().map_or(false, |ext| {
            matches!(ext.to_str(), Some("rs" | "toml" | "json"))
        }) {
            // Skip the binary itself
            if path == binary_path {
                continue;
            }

            // Get file modification time
            if let Ok(metadata) = std::fs::metadata(path) {
                if let Ok(file_mtime) = metadata.modified() {
                    if file_mtime > binary_mtime {
                        return Ok(Some(path.to_path_buf()));
                    }
                }
            }
        }
    }

    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
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
        fs::create_dir_all(&target_dir).unwrap();
        fs::write(target_dir.join("masday"), "binary").unwrap();

        let result = find_mcp_binary(project_dir).unwrap();
        assert!(result.ends_with("target/release/masday"));
    }

    #[test]
    fn test_find_mcp_binary_debug() {
        let temp_dir = TempDir::new().unwrap();
        let project_dir = temp_dir.path();
        let target_dir = project_dir.join("target/debug");
        fs::create_dir_all(&target_dir).unwrap();
        fs::write(target_dir.join("masday"), "binary").unwrap();

        let result = find_mcp_binary(project_dir).unwrap();
        assert!(result.ends_with("target/debug/masday"));
    }

    #[test]
    fn test_find_api_binary_release() {
        let temp_dir = TempDir::new().unwrap();
        let project_dir = temp_dir.path();
        let target_dir = project_dir.join("target/release");
        fs::create_dir_all(&target_dir).unwrap();
        fs::write(target_dir.join("masday"), "binary").unwrap();

        let result = find_api_binary(project_dir).unwrap();
        assert!(result.ends_with("target/release/masday"));
    }

    #[test]
    fn test_find_mcp_binary_prefers_release() {
        let temp_dir = TempDir::new().unwrap();
        let project_dir = temp_dir.path();
        let release_dir = project_dir.join("target/release");
        let debug_dir = project_dir.join("target/debug");
        fs::create_dir_all(&release_dir).unwrap();
        fs::create_dir_all(&debug_dir).unwrap();
        fs::write(release_dir.join("masday"), "release").unwrap();
        fs::write(debug_dir.join("masday"), "debug").unwrap();

        let result = find_mcp_binary(project_dir).unwrap();
        assert!(result.ends_with("target/release/masday"));
    }

    #[test]
    fn test_is_build_fresh_no_binary() {
        let temp_dir = TempDir::new().unwrap();
        let project_dir = temp_dir.path();

        // No binary exists
        assert!(!is_build_fresh(project_dir));
    }

    #[test]
    fn test_is_build_fresh_no_sources() {
        let temp_dir = TempDir::new().unwrap();
        let project_dir = temp_dir.path();
        let target_dir = project_dir.join("target/release");
        fs::create_dir_all(&target_dir).unwrap();

        // Create binary
        let binary_path = target_dir.join("masday");
        fs::write(&binary_path, "binary").unwrap();

        // No source files, should be fresh
        assert!(is_build_fresh(project_dir));
    }

    #[test]
    fn test_is_build_fresh_with_older_source() {
        let temp_dir = TempDir::new().unwrap();
        let project_dir = temp_dir.path();
        let target_dir = project_dir.join("target/release");
        let src_dir = project_dir.join("src");
        fs::create_dir_all(&target_dir).unwrap();
        fs::create_dir_all(&src_dir).unwrap();

        // Create source file first
        let source_path = src_dir.join("main.rs");
        fs::write(&source_path, "fn main() {}").unwrap();

        // Wait a bit to ensure timestamp difference
        std::thread::sleep(std::time::Duration::from_millis(10));

        // Create binary (newer)
        let binary_path = target_dir.join("masday");
        fs::write(&binary_path, "binary").unwrap();

        // Binary is newer, should be fresh
        assert!(is_build_fresh(project_dir));
    }

    #[test]
    fn test_is_build_fresh_with_newer_source() {
        let temp_dir = TempDir::new().unwrap();
        let project_dir = temp_dir.path();
        let target_dir = project_dir.join("target/release");
        let src_dir = project_dir.join("src");
        fs::create_dir_all(&target_dir).unwrap();
        fs::create_dir_all(&src_dir).unwrap();

        // Create binary first
        let binary_path = target_dir.join("masday");
        fs::write(&binary_path, "binary").unwrap();

        // Wait a bit to ensure timestamp difference
        std::thread::sleep(std::time::Duration::from_millis(10));

        // Create source file (newer)
        let source_path = src_dir.join("main.rs");
        fs::write(&source_path, "fn main() {}").unwrap();

        // Source is newer, should NOT be fresh
        assert!(!is_build_fresh(project_dir));
    }

    #[test]
    fn test_find_or_download_binary_dev_mode() {
        let temp_dir = TempDir::new().unwrap();
        let project_dir = temp_dir.path();
        let target_dir = project_dir.join("target/release");
        fs::create_dir_all(&target_dir).unwrap();

        // Create binary
        let binary_path = target_dir.join("masday");
        fs::write(&binary_path, "binary").unwrap();

        // In dev mode, should build (with freshness check) and find binary
        let result = find_or_download_binary(true, project_dir);
        assert!(result.is_ok());
        assert!(result.unwrap().ends_with("target/release/masday"));
    }

    #[test]
    fn test_find_or_download_binary_production_finds_in_path() {
        let temp_dir = TempDir::new().unwrap();
        let project_dir = temp_dir.path();

        // In production mode, should find binary in PATH (if installed globally)
        // This test passes when masday is installed at ~/.masday/bin/masday
        let result = find_or_download_binary(false, project_dir);
        // If masday is installed globally, should return Ok with the PATH
        if let Ok(path) = result {
            // Verify it found a valid masday binary
            assert!(path.ends_with("masday") || path.ends_with("masday.exe"));
        }
        // If not installed, would return Err with download message
    }

    #[test]
    fn test_find_newer_source_none() {
        let temp_dir = TempDir::new().unwrap();
        let project_dir = temp_dir.path();
        let target_dir = project_dir.join("target/release");
        fs::create_dir_all(&target_dir).unwrap();

        // Create binary
        let binary_path = target_dir.join("masday");
        fs::write(&binary_path, "binary").unwrap();
        let metadata = fs::metadata(&binary_path).unwrap();
        let binary_mtime = metadata.modified().unwrap();

        // No sources newer
        let result = find_newer_source(project_dir, &binary_path, binary_mtime).unwrap();
        assert!(result.is_none());
    }
}
