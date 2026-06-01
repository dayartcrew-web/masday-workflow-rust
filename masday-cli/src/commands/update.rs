//! Update command implementation
//!
//! Updates Masday installation by re-running install with preserved .env.

use std::path::Path;
use anyhow::Result;
use console::style;

use super::install::{run as install_run, InstallArgs};
use crate::installer::load_env;

/// Run the update command
///
/// Preserves existing .env content, then runs full install with force=true.
pub fn run(project_dir: &Path) -> Result<()> {
    println!();
    println!("{}", style("Updating Masday installation...").cyan().bold());
    println!();

    // Save current .env content
    let env_backup = load_env(project_dir)?;

    if !env_backup.is_empty() {
        println!(
            "{}",
            style(format!("Preserved {} environment variables from .env", env_backup.len())).cyan()
        );
    }

    // Run full install with force=true
    let install_args = InstallArgs {
        skip_build: false,
        local_only: false,
        force: true,
        ..Default::default()
    };

    install_run(install_args, project_dir)?;

    // Note: .env is preserved by ensure_env_file which doesn't overwrite if exists
    // No restore needed, just confirm
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
    use tempfile::TempDir;
    use std::fs;

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
