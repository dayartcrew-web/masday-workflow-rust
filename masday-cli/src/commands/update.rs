//! Update command implementation
//!
//! Updates Masday by downloading the latest binary from GitHub Releases
//! and re-syncing configuration while preserving ~/.masday/config.toml.

use anyhow::{Context, Result};
use console::style;
use home;
use indicatif::{ProgressBar, ProgressStyle};
use std::fs;
use std::io::Read;
use std::path::Path;

use super::install::{run as install_run, InstallArgs};

const GITHUB_RELEASE_REPO: &str = "dayartcrew-web/masday-workflow-release";

fn release_binary_name() -> &'static str {
    if cfg!(windows) {
        "masday-windows-x86_64.exe"
    } else {
        "masday-linux-x86_64"
    }
}

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

/// Run the update command
///
/// Downloads the latest binary from GitHub Releases and re-syncs configuration.
pub fn run(project_dir: &Path) -> Result<()> {
    println!();
    println!("{}", style("Updating Masday installation...").cyan().bold());
    println!();

    // Backup config.toml before update
    let home =
        home::home_dir().ok_or_else(|| anyhow::anyhow!("Cannot determine home directory"))?;
    let masday_dir = home.join(".masday");
    let config_path = masday_dir.join("config.toml");
    let config_backup = if config_path.exists() {
        let content = fs::read_to_string(&config_path).ok();
        println!("  {} Preserved config.toml", style("✓").green());
        content
    } else {
        None
    };

    // Download latest binary from GitHub Releases
    let binary_name = release_binary_name();
    let url = format!(
        "https://github.com/{}/releases/latest/download/{}",
        GITHUB_RELEASE_REPO, binary_name
    );

    println!("  Downloading latest release...");
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

    // Atomic rename
    fs::rename(&tmp_dest, &dest)?;

    println!("  {} Installed to {}", style("✓").green(), dest.display());

    // Restore config.toml (install_run may overwrite)
    if let Some(ref content) = config_backup {
        let _ = fs::write(&config_path, content);
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
        let _ = fs::write(&config_path, content);
    }

    println!();
    println!("{}", style("Update complete!").green().bold());
    println!();

    if config_backup.is_some() {
        println!("Configuration preserved from config.toml");
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

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
}
