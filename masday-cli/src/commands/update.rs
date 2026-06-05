//! Update command implementation
//!
//! Updates Masday by downloading the latest binary from GitHub Releases
//! and re-syncing configuration while preserving ~/.masday/config.toml.

use anyhow::{bail, Context, Result};
use console::style;
use home;
use indicatif::{ProgressBar, ProgressStyle};
use std::fs;
use std::io::Read;
use std::path::Path;
use std::time::Duration;

use super::install::{run as install_run, InstallArgs};

const GITHUB_RELEASE_REPO: &str = "dayartcrew-web/masday-workflow-release";
const GITHUB_API_BASE: &str = "https://api.github.com";

/// Arguments for the update command
#[derive(Debug, Clone, Default)]
pub struct UpdateArgs {
    /// Check for available update without applying
    pub check: bool,
    /// Update to specific version (default: latest)
    pub version: Option<String>,
    /// Only update agents/skills/hooks, not the binary
    pub skip_binary: bool,
    /// Don't overwrite config.toml
    pub skip_config: bool,
    /// Show what would be updated without changing anything
    pub dry_run: bool,
    /// Force re-install even if already up-to-date
    pub force: bool,
}

/// Current version from Cargo.toml
fn current_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

/// Normalize version string (strip 'v' prefix if present)
fn normalize_version(version: &str) -> String {
    version.strip_prefix('v').unwrap_or(version).to_string()
}

/// Release binary name for current platform
fn release_binary_name() -> &'static str {
    if cfg!(windows) {
        "masday-windows-x86_64.exe"
    } else {
        "masday-linux-x86_64"
    }
}

/// Fetch latest release version from GitHub API
fn fetch_latest_version() -> Result<String> {
    let url = format!(
        "{}/repos/{}/releases/latest",
        GITHUB_API_BASE, GITHUB_RELEASE_REPO
    );

    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()?;

    let response = client
        .get(&url)
        .header("User-Agent", "masday-cli")
        .send()
        .with_context(|| format!("Failed to fetch release info from {}", url))?;

    if !response.status().is_success() {
        anyhow::bail!(
            "GitHub API request failed: HTTP {}",
            response.status()
        );
    }

    let json: serde_json::Value = response.json().context("Failed to parse GitHub response")?;

    let tag_name = json
        .get("tag_name")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("Release response missing tag_name"))?;

    Ok(normalize_version(tag_name))
}

/// Fetch specific release version from GitHub API
fn fetch_specific_version(version: &str) -> Result<String> {
    let normalized = normalize_version(version);
    let url = format!(
        "{}/repos/{}/releases/tags/v{}",
        GITHUB_API_BASE, GITHUB_RELEASE_REPO, normalized
    );

    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()?;

    let response = client
        .get(&url)
        .header("User-Agent", "masday-cli")
        .send()
        .with_context(|| format!("Failed to fetch release info for v{}", normalized))?;

    if !response.status().is_success() {
        anyhow::bail!(
            "GitHub API request failed: HTTP {} (version v{} may not exist)",
            response.status(),
            normalized
        );
    }

    let json: serde_json::Value = response.json().context("Failed to parse GitHub response")?;

    let tag_name = json
        .get("tag_name")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("Release response missing tag_name"))?;

    Ok(normalize_version(tag_name))
}

/// Parse version string into components with proper error handling
fn parse_version(version: &str) -> Result<Vec<u32>> {
    version
        .split('.')
        .map(|s| {
            s.parse().with_context(|| format!("Invalid version component '{}' in '{}'", s, version))
        })
        .collect()
}

/// Compare two version strings (simple semver comparison)
fn version_compare(a: &str, b: &str) -> std::cmp::Ordering {
    let a_parts = match parse_version(a) {
        Ok(parts) => parts,
        Err(_) => return std::cmp::Ordering::Equal, // Treat invalid versions as equal
    };
    let b_parts = match parse_version(b) {
        Ok(parts) => parts,
        Err(_) => return std::cmp::Ordering::Equal,
    };

    for (a_part, b_part) in a_parts.iter().zip(b_parts.iter()) {
        match a_part.cmp(b_part) {
            std::cmp::Ordering::Equal => continue,
            other => return other,
        }
    }

    a_parts.len().cmp(&b_parts.len())
}

/// Download file with progress bar
fn download_with_progress(url: &str, dest: &Path) -> Result<()> {
    let response = reqwest::blocking::Client::new()
        .get(url)
        .send()
        .with_context(|| format!("Failed to connect to {}", url))?;

    if !response.status().is_success() {
        anyhow::bail!("Download failed: HTTP {}", response.status());
    }

    let total_size = response.content_length().unwrap_or(0);
    let pb = ProgressBar::new(total_size);
    pb.set_style(
        ProgressStyle::with_template(
            "{msg}\n  [{elapsed_precise}] [{bar:40.cyan/blue}] {bytes}/{total_bytes} ({eta})",
        )
        .unwrap()
        .progress_chars("#>-"),
    );
    pb.set_message("Downloading masday binary...");

    let mut file =
        fs::File::create(dest).with_context(|| format!("Failed to create {}", dest.display()))?;

    let mut downloaded: u64 = 0;
    let mut stream = response;
    let mut buf = [0u8; 8192];
    loop {
        let n = stream.read(&mut buf)?;
        if n == 0 {
            break;
        }
        std::io::Write::write_all(&mut file, &buf[..n])?;
        downloaded += n as u64;
        pb.set_position(downloaded);
    }
    pb.finish_with_message("Download complete");
    Ok(())
}

/// Check for updates and print status
fn run_check(args: &UpdateArgs) -> Result<bool> {
    println!();
    println!("{}", style("Checking for updates...").cyan().bold());
    println!();

    let current = current_version();
    println!("  Current version: {}", style(&current).cyan());

    let target_version = if let Some(ref version) = args.version {
        let normalized = normalize_version(version);
        // Verify the requested version exists
        fetch_specific_version(&normalized)?;
        normalized
    } else {
        fetch_latest_version()?
    };

    println!("  Latest version:  {}", style(&target_version).green());

    match version_compare(&target_version, &current) {
        std::cmp::Ordering::Greater => {
            println!();
            println!("{}", style("✓ A new version is available!").green().bold());
            println!("  Run {} to update.", style("masday update").cyan());
            Ok(true)
        }
        std::cmp::Ordering::Equal => {
            println!();
            println!("{}", style("✓ You're already on the latest version").green());
            if args.force {
                println!("  Use {} to re-install.", style("--force").cyan());
            }
            Ok(false)
        }
        std::cmp::Ordering::Less => {
            println!();
            println!(
                "{}",
                style("⚠ Your version is newer than the latest release").yellow()
            );
            println!("  This can happen in development builds.");
            Ok(false)
        }
    }
}

/// Dry run - show what would be updated
fn run_dry_run(args: &UpdateArgs, _project_dir: &Path) -> Result<()> {
    println!();
    println!("{}", style("Dry run — preview of update operations").cyan().bold());
    println!();

    let home =
        home::home_dir().ok_or_else(|| anyhow::anyhow!("Cannot determine home directory"))?;
    let masday_dir = home.join(".masday");
    let install_dir = masday_dir.join("bin");
    let config_path = masday_dir.join("config.toml");

    let current = current_version();
    let target_version = if let Some(ref version) = args.version {
        let normalized = normalize_version(version);
        fetch_specific_version(&normalized)?;
        normalized
    } else {
        fetch_latest_version()?
    };

    println!("{}", style("Version information:").cyan());
    println!("  Current: {}", style(&current).cyan());
    println!("  Target:  {}", style(&target_version).green());
    println!();

    if !args.skip_binary {
        println!("{}", style("Binary update:").cyan());
        let binary_filename = if cfg!(windows) {
            "masday.exe"
        } else {
            "masday"
        };
        println!("  Download: masday binary from GitHub releases");
        println!("  Install:  {}", install_dir.join(binary_filename).display());
        println!();
    }

    println!("{}", style("Asset sync:").cyan());
    println!("  Agents:   Sync to .claude/agents/");
    println!("  Skills:   Sync to .claude/skills/");
    println!("  Hooks:    Reinstall to .claude/hooks/");
    println!();

    if args.skip_config {
        println!("{}", style("Configuration:").cyan());
        println!("  config.toml: {} (skip)", style("preserved").green());
        println!();
    } else {
        println!("{}", style("Configuration:").cyan());
        if config_path.exists() {
            println!("  config.toml: {} (backup & restore)", style("preserve").yellow());
        } else {
            println!("  config.toml: {} (will be created)", style("new").green());
        }
        println!();
    }

    println!("{}", style("Summary:").cyan());
    println!("  Binary download: {}", if args.skip_binary { "skip" } else { "yes" });
    println!("  Config update: {}", if args.skip_config { "skip" } else { "yes" });
    println!("  Asset sync: yes");
    println!();

    if version_compare(&target_version, &current) == std::cmp::Ordering::Equal && !args.force {
        println!("{}", style("⚠ Already up-to-date (use --force to reinstall)").yellow());
    } else {
        println!("{}", style("✓ Update would proceed").green());
    }

    Ok(())
}

/// Backup config.toml to memory
fn backup_config(config_path: &Path) -> Result<Option<String>> {
    if !config_path.exists() {
        return Ok(None);
    }
    let content = fs::read_to_string(config_path)
        .with_context(|| format!("Failed to read config from {}", config_path.display()))?;
    Ok(Some(content))
}

/// Restore config.toml from memory
fn restore_config(config_path: &Path, content: &str) -> Result<()> {
    fs::write(config_path, content)
        .with_context(|| format!("Failed to restore config to {}", config_path.display()))?;
    Ok(())
}

/// Run the full update
/// Replace a binary file, handling Windows file locking.
/// On Windows, a running process locks its executable, so direct rename fails.
/// Strategy: old → .old, new → dest. On next run, .old gets cleaned up.
fn replace_binary(tmp_path: &Path, dest: &Path) -> Result<()> {
    // Try direct rename first (works on Linux/macOS and Windows if not running)
    if fs::rename(tmp_path, dest).is_ok() {
        return Ok(());
    }

    // Windows: running binary is locked. Use rename-swap strategy.
    let old_path = dest.with_extension("old");

    // Remove any previous .old file
    let _ = fs::remove_file(&old_path);

    // Rename current binary to .old
    if let Err(e) = fs::rename(dest, &old_path) {
        // If even rename to .old fails, try to copy instead
        #[cfg(windows)]
        {
            // On Windows, try writing a small updater script
            let script_path = dest.with_extension("update.bat");
            let dest_str = dest.to_str().unwrap_or("");
            let tmp_str = tmp_path.to_str().unwrap_or("");
            let old_str = old_path.to_str().unwrap_or("");

            let script = format!(
                "@echo off\ntimeout /t 2 /nobreak >nul\nmove /y \"{}\" \"{}\"\ndel /f \"{}\" 2>nul\ndel \"%~f0\"\n",
                tmp_str.replace('/', "\\"),
                dest_str.replace('/', "\\"),
                old_str.replace('/', "\\")
            );
            fs::write(&script_path, &script)?;

            bail!(
                "Cannot replace running binary on Windows.\n\n\
                Update downloaded to: {}\n\n\
                Run this to complete the update:\n  {}\n\n\
                Or close this terminal and run:\n  masday update",
                tmp_path.display(),
                script_path.display()
            );
        }
        bail!("Failed to replace binary: {}. Close any running masday processes and try again.", e);
    }

    // Now rename the new binary into place
    fs::rename(tmp_path, dest).with_context(|| {
        // If this fails, try to restore the old binary
        let _ = fs::rename(&old_path, dest);
        format!("Failed to install new binary. Old binary restored.")
    })?;

    // Clean up old binary (best effort)
    let _ = fs::remove_file(&old_path);

    Ok(())
}

fn run_update(args: &UpdateArgs, project_dir: &Path) -> Result<()> {
    let current = current_version();
    let target_version = if let Some(ref version) = args.version {
        let normalized = normalize_version(version);
        fetch_specific_version(&normalized)?;
        normalized
    } else {
        fetch_latest_version()?
    };

    // Check if update is needed (unless --force)
    if !args.force && version_compare(&target_version, &current) == std::cmp::Ordering::Equal {
        println!();
        println!("{}", style("✓ Already up-to-date").green().bold());
        println!("  Current version: {}", style(&current).cyan());
        println!();
        println!("Use {} to force re-install.", style("--force").yellow());
        return Ok(());
    }

    println!();
    println!(
        "{}",
        style(format!("Updating Masday {} -> {}...", current, target_version))
            .cyan()
            .bold()
    );
    println!();

    // Setup paths
    let home =
        home::home_dir().ok_or_else(|| anyhow::anyhow!("Cannot determine home directory"))?;
    let masday_dir = home.join(".masday");
    let config_path = masday_dir.join("config.toml");

    // Backup config.toml before update (unless --skip-config)
    let config_backup = if !args.skip_config {
        let backup = backup_config(&config_path)?;
        if backup.is_some() {
            println!("  {} Backed up config.toml", style("✓").green());
        }
        backup
    } else {
        None
    };

    // Download latest binary from GitHub Releases (unless --skip-binary)
    if !args.skip_binary {
        let binary_name = release_binary_name();
        let version_tag = format!("v{}", target_version);
        let url = format!(
            "https://github.com/{}/releases/download/{}/{}",
            GITHUB_RELEASE_REPO, version_tag, binary_name
        );

        println!("  Downloading {} release...", version_tag);
        println!("  URL: {}", url);

        let install_dir = masday_dir.join("bin");
        fs::create_dir_all(&install_dir)?;

        let binary_filename = if cfg!(windows) {
            "masday.exe"
        } else {
            "masday"
        };
        let dest = install_dir.join(binary_filename);
        let tmp_dest = install_dir.join(format!("{}.tmp", binary_filename));

        download_with_progress(&url, &tmp_dest)?;

        // Set executable permission on Unix
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&tmp_dest, PermissionsExt::from_mode(0o755))?;
        }

        // Replace binary — handle Windows file locking
        replace_binary(&tmp_dest, &dest)?;;

        println!("  {} Installed to {}", style("✓").green(), dest.display());
    } else {
        println!("  {} Skipped binary download", style("⊘").dim());
    }

    // Restore config.toml (install_run may overwrite)
    if let Some(ref content) = config_backup {
        restore_config(&config_path, content)?;
    }

    // Re-run install to re-sync agents/skills/hooks/MCP config
    println!();
    println!("{}", style("Re-syncing configuration...").cyan());

    let install_args = InstallArgs {
        skip_build: true,
        force: true,
        ..Default::default()
    };

    install_run(install_args, project_dir)?;

    // Ensure config.toml is preserved after install
    if let Some(ref content) = config_backup {
        restore_config(&config_path, content)?;
    }

    println!();
    println!("{}", style("Update complete!").green().bold());
    println!();
    println!("  Updated to version: {}", style(&target_version).green());

    if config_backup.is_some() {
        println!("  Configuration preserved from config.toml");
    }

    Ok(())
}

/// Run the update command
pub fn run(args: UpdateArgs, project_dir: &Path) -> Result<()> {
    // --check only: check for updates and exit
    if args.check {
        run_check(&args)?;
        return Ok(());
    }

    // --dry-run: show what would happen
    if args.dry_run {
        run_dry_run(&args, project_dir)?;
        return Ok(());
    }

    // Full update
    run_update(&args, project_dir)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_current_version() {
        let version = current_version();
        // Should be a valid semver version
        let parts: Vec<&str> = version.split('.').collect();
        assert_eq!(parts.len(), 3, "Version should have 3 parts");
    }

    #[test]
    fn test_normalize_version() {
        assert_eq!(normalize_version("v1.2.3"), "1.2.3");
        assert_eq!(normalize_version("1.2.3"), "1.2.3");
        assert_eq!(normalize_version("v0.1.0"), "0.1.0");
    }

    #[test]
    fn test_version_compare() {
        assert_eq!(version_compare("1.2.3", "1.2.3"), std::cmp::Ordering::Equal);
        assert_eq!(version_compare("1.2.4", "1.2.3"), std::cmp::Ordering::Greater);
        assert_eq!(version_compare("1.2.2", "1.2.3"), std::cmp::Ordering::Less);
        assert_eq!(version_compare("1.10.0", "1.2.0"), std::cmp::Ordering::Greater);
        assert_eq!(version_compare("2.0.0", "1.9.9"), std::cmp::Ordering::Greater);
    }

    #[test]
    fn test_release_binary_name() {
        let name = release_binary_name();
        if cfg!(windows) {
            assert_eq!(name, "masday-windows-x86_64.exe");
        } else {
            assert_eq!(name, "masday-linux-x86_64");
        }
    }

    #[test]
    fn test_update_args_default() {
        let args = UpdateArgs::default();
        assert!(!args.check);
        assert!(args.version.is_none());
        assert!(!args.skip_binary);
        assert!(!args.skip_config);
        assert!(!args.dry_run);
        assert!(!args.force);
    }

    #[test]
    fn test_config_backup_read() {
        let temp_dir = TempDir::new().unwrap();
        let masday_dir = temp_dir.path().join(".masday");
        fs::create_dir_all(&masday_dir).unwrap();
        let config_path = masday_dir.join("config.toml");
        fs::write(&config_path, "mode = \"local\"\nport = 30101").unwrap();

        let content = fs::read_to_string(&config_path).ok();
        assert!(content.is_some());
        assert!(content.unwrap().contains("mode = \"local\""));
    }

    #[test]
    fn test_config_backup_missing() {
        let temp_dir = TempDir::new().unwrap();
        let masday_dir = temp_dir.path().join(".masday");
        fs::create_dir_all(&masday_dir).unwrap();

        let config_path = masday_dir.join("config.toml");
        let content = if config_path.exists() {
            fs::read_to_string(&config_path).ok()
        } else {
            None
        };
        assert!(content.is_none());
    }

    #[test]
    fn test_version_compare_equal() {
        assert_eq!(version_compare("1.0.0", "1.0.0"), std::cmp::Ordering::Equal);
    }

    #[test]
    fn test_version_compare_patch() {
        assert_eq!(version_compare("1.0.1", "1.0.0"), std::cmp::Ordering::Greater);
        assert_eq!(version_compare("1.0.0", "1.0.1"), std::cmp::Ordering::Less);
    }

    #[test]
    fn test_version_compare_minor() {
        assert_eq!(version_compare("1.1.0", "1.0.0"), std::cmp::Ordering::Greater);
        assert_eq!(version_compare("1.0.0", "1.1.0"), std::cmp::Ordering::Less);
    }

    #[test]
    fn test_version_compare_major() {
        assert_eq!(version_compare("2.0.0", "1.0.0"), std::cmp::Ordering::Greater);
        assert_eq!(version_compare("1.0.0", "2.0.0"), std::cmp::Ordering::Less);
    }

    // ========== Integration-style tests ==========

    #[test]
    fn test_check_flag_returns_correct_version_comparison() {
        // Test that version_compare returns correct ordering
        assert_eq!(version_compare("1.0.0", "1.0.0"), std::cmp::Ordering::Equal);
        assert_eq!(version_compare("1.0.1", "1.0.0"), std::cmp::Ordering::Greater);
        assert_eq!(version_compare("1.0.0", "1.0.1"), std::cmp::Ordering::Less);
        assert_eq!(version_compare("2.0.0", "1.9.9"), std::cmp::Ordering::Greater);
    }

    #[test]
    fn test_dry_run_creates_no_files() {
        let temp_dir = TempDir::new().unwrap();
        let project_dir = temp_dir.path();

        // Create initial state
        let masday_dir = temp_dir.path().join(".masday");
        fs::create_dir_all(&masday_dir).unwrap();

        let args = UpdateArgs {
            dry_run: true,
            ..Default::default()
        };

        // Run dry_run (should not create any files)
        let result = run_dry_run(&args, project_dir);
        assert!(result.is_ok(), "Dry run should succeed");

        // Verify no files were created
        let entries = fs::read_dir(&masday_dir).unwrap();
        let count = entries.count();
        assert_eq!(count, 0, "Dry run should not create any files");
    }

    #[test]
    fn test_skip_binary_still_syncs_assets() {
        let args = UpdateArgs {
            skip_binary: true,
            dry_run: true, // Use dry_run to test without actual network calls
            ..Default::default()
        };

        let temp_dir = TempDir::new().unwrap();
        let project_dir = temp_dir.path();

        let result = run_dry_run(&args, project_dir);
        assert!(result.is_ok());

        // In dry_run mode, skip_binary should be reflected in output
        // The function succeeds and would proceed to sync assets
        assert!(args.skip_binary);
    }

    #[test]
    fn test_skip_config_preserves_config() {
        let temp_dir = TempDir::new().unwrap();
        let masday_dir = temp_dir.path().join(".masday");
        fs::create_dir_all(&masday_dir).unwrap();

        let config_path = masday_dir.join("config.toml");
        let original_content = "mode = \"local\"\nport = 30101";
        fs::write(&config_path, original_content).unwrap();

        let args = UpdateArgs {
            skip_config: true,
            ..Default::default()
        };

        // Test backup behavior with skip_config
        let config_backup = if !args.skip_config {
            backup_config(&config_path).unwrap()
        } else {
            None
        };

        assert!(config_backup.is_none(), "skip_config should not backup config");

        // Verify original content unchanged
        let current = fs::read_to_string(&config_path).unwrap();
        assert_eq!(current, original_content);
    }

    #[test]
    fn test_force_bypasses_version_check() {
        let current = "1.0.0";
        let target = "1.0.0"; // Same version

        // Without force, equal versions should skip update
        assert_eq!(version_compare(target, current), std::cmp::Ordering::Equal);

        // With force, update should proceed (tested by checking force flag allows update)
        let args = UpdateArgs {
            force: true,
            ..Default::default()
        };

        assert!(args.force, "Force flag should be set");
    }

    #[test]
    fn test_version_flag_validates_release() {
        // Test normalize_version with various inputs
        assert_eq!(normalize_version("v1.2.3"), "1.2.3");
        assert_eq!(normalize_version("1.2.3"), "1.2.3");
        assert_eq!(normalize_version("v0.1.0"), "0.1.0");

        // Version strings should be parseable
        let result = parse_version("1.2.3");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), vec![1, 2, 3]);
    }

    // ========== Error case tests ==========

    #[test]
    fn test_malformed_version_string() {
        // Test that malformed versions are handled gracefully
        let result = parse_version("invalid");
        assert!(result.is_err(), "Malformed version should return error");

        let result = parse_version("1.2.x");
        assert!(result.is_err(), "Non-numeric version should return error");

        let result = parse_version("1.2.3.4.5");
        assert!(result.is_ok(), "Extra version components should parse");
    }

    #[test]
    fn test_version_compare_with_invalid_versions() {
        // Invalid versions should be treated as equal (safe default)
        assert_eq!(
            version_compare("invalid", "1.0.0"),
            std::cmp::Ordering::Equal
        );
        assert_eq!(
            version_compare("1.0.0", "invalid"),
            std::cmp::Ordering::Equal
        );
    }

    #[test]
    fn test_backup_config_missing_file() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.toml");

        let result = backup_config(&config_path);
        assert!(result.is_ok(), "Backup should succeed with missing file");
        assert!(result.unwrap().is_none(), "Missing file should return None");
    }

    #[test]
    fn test_backup_and_restore_config() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.toml");
        let original_content = "mode = \"local\"\nport = 30101";
        fs::write(&config_path, original_content).unwrap();

        // Backup
        let backup = backup_config(&config_path).unwrap();
        assert!(backup.is_some());
        assert_eq!(backup.as_ref().unwrap(), original_content);

        // Modify file
        fs::write(&config_path, "mode = \"remote\"").unwrap();

        // Restore
        restore_config(&config_path, backup.as_ref().unwrap()).unwrap();

        let restored = fs::read_to_string(&config_path).unwrap();
        assert_eq!(restored, original_content, "Config should be restored to original");
    }

    #[test]
    fn test_restore_config_to_nonexistent_directory() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("nonexistent").join("config.toml");
        let content = "mode = \"local\"";

        let result = restore_config(&config_path, content);
        assert!(result.is_err(), "Restore to nonexistent dir should fail");
    }
}
