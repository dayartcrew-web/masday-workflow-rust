//! Update command implementation
//!
//! Updates Masday installation by downloading the latest binary from GitHub Releases
//! and re-syncing configuration while preserving .env.

use anyhow::{Context, Result};
use console::style;
use home;
use indicatif::{ProgressBar, ProgressStyle};
use std::fs;
use std::io::Read;
use std::path::Path;

use super::install::{run as install_run, InstallArgs};
use crate::installer::load_env;

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

    // Save current .env content
    let env_backup = load_env(project_dir)?;

    if !env_backup.is_empty() {
        println!(
            "{}",
            style(format!(
                "Preserved {} environment variables from .env",
                env_backup.len()
            ))
            .cyan()
        );
    }

    // Download latest binary from GitHub Releases
    let binary_name = release_binary_name();
    let url = format!(
        "https://github.com/{}/releases/latest/download/{}",
        GITHUB_RELEASE_REPO, binary_name
    );

    println!("  Downloading latest release...");
    println!("  URL: {}", url);

    let home =
        home::home_dir().ok_or_else(|| anyhow::anyhow!("Cannot determine home directory"))?;
    let install_dir = home.join(".masday").join("bin");
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

    // Re-run install to re-sync agents/skills/hooks/MCP config
    println!();
    println!("{}", style("Re-syncing configuration...").cyan());

    let install_args = InstallArgs {
        skip_build: true,
        force: true,
        ..Default::default()
    };

    install_run(install_args, project_dir)?;

    println!();
    println!("{}", style("Update complete!").green().bold());
    println!();

    if !env_backup.is_empty() {
        println!("Environment configuration preserved from .env");
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_load_env_backup() {
        let temp_dir = TempDir::new().unwrap();
        let project_dir = temp_dir.path();
        let env_path = project_dir.join(".env");

        fs::write(
            &env_path,
            "KEY1=value1\nKEY2=value2\n# comment\n\nKEY3=value3",
        )
        .unwrap();

        let env_backup = load_env(project_dir).unwrap();
        assert_eq!(env_backup.len(), 3);
        assert_eq!(env_backup.get("KEY1"), Some(&"value1".to_string()));
        assert_eq!(env_backup.get("KEY2"), Some(&"value2".to_string()));
        assert_eq!(env_backup.get("KEY3"), Some(&"value3".to_string()));
    }

    #[test]
    fn test_load_env_empty() {
        let temp_dir = TempDir::new().unwrap();
        let project_dir = temp_dir.path();

        let env_backup = load_env(project_dir).unwrap();
        assert!(env_backup.is_empty());
    }

    #[test]
    fn test_load_env_nonexistent() {
        let temp_dir = TempDir::new().unwrap();
        let project_dir = temp_dir.path();

        // .env doesn't exist
        let env_backup = load_env(project_dir).unwrap();
        assert!(env_backup.is_empty());
    }
}
