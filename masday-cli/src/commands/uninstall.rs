//! Uninstall command implementation
//!
//! Removes Masday agents, skills, hooks, and MCP configurations.

use anyhow::{Context, Result};
use console::style;
use home;
use std::fs;
use std::path::Path;

use crate::installer::{
    all_platforms, detect_active_platforms, remove_masday_entries, remove_mcp_config,
    uninstall_global_hooks, uninstall_project_hooks, Platform,
};

/// Arguments for the uninstall command
#[derive(Debug, Clone, Default)]
pub struct UninstallArgs {
    /// Remove global hooks and global masday entries
    pub global: bool,
    /// Specific platform to uninstall (None = all)
    pub platform: Option<String>,
}

/// Run the uninstall command
pub fn run(args: UninstallArgs, project_dir: &Path) -> Result<()> {
    println!();
    println!("{}", style("Uninstalling Masday...").cyan().bold());
    println!();

    // Resolve platforms
    let platforms = resolve_platforms(&args.platform, project_dir)?;
    println!(
        "{}",
        style(format!(
            "Uninstalling from platforms: {}",
            platform_list(&platforms)
        ))
        .cyan()
    );

    // Remove agents from project dirs
    println!();
    println!(
        "{}",
        style("Removing agents from project directories...").cyan()
    );
    for platform in &platforms {
        let agents_dir = platform.project_agents_dir(project_dir);
        let removed = remove_masday_files(&agents_dir)?;
        println!(
            "  {}: {} removed",
            style(platform.name()).green(),
            style(removed).dim()
        );
    }

    // Remove skills from project dirs
    println!();
    println!(
        "{}",
        style("Removing skills from project directories...").cyan()
    );
    for platform in &platforms {
        let skills_dir = platform.project_skills_dir(project_dir);
        let removed = remove_masday_dirs(&skills_dir)?;
        println!(
            "  {}: {} removed",
            style(platform.name()).green(),
            style(removed).dim()
        );
    }

    // Remove project hooks
    println!();
    println!("{}", style("Removing project hooks...").cyan());
    let hooks_report = uninstall_project_hooks(project_dir)?;
    println!(
        "  {}",
        style(format!("{} hooks removed", hooks_report.copied)).green()
    );

    // Clean project settings (remove masday hook entries from .claude/settings.json)
    let project_settings = project_dir.join(".claude/settings.json");
    if project_settings.exists() {
        remove_masday_entries(&project_settings)?;
        println!("  {}", style("Project settings cleaned").green());
    }
    let project_local_settings = project_dir.join(".claude/settings.local.json");
    if project_local_settings.exists() {
        remove_masday_entries(&project_local_settings)?;
        println!("  {}", style("Project local settings cleaned").green());
    }

    // Remove MCP configs
    println!();
    println!("{}", style("Removing MCP configs...").cyan());
    for platform in &platforms {
        remove_mcp_config(platform, project_dir)?;
        println!("  {}", style(platform.name()).green());
    }

    // Global cleanup
    if args.global {
        println!();
        println!("{}", style("Removing global Masday components...").cyan());

        // Remove global hooks
        if let Some(home) = home::home_dir() {
            let global_hooks = uninstall_global_hooks(&home)?;
            println!(
                "  Global hooks: {} removed",
                style(global_hooks.copied).dim()
            );

            // Remove global skills
            for platform in &platforms {
                if let Some(global_skills_dir) = platform.global_skills_dir() {
                    let removed = remove_masday_dirs(&global_skills_dir)?;
                    if removed > 0 {
                        println!(
                            "  {} global skills: {} removed",
                            style(platform.name()).green(),
                            style(removed).dim()
                        );
                    }
                }
            }

            // Remove global agents
            for platform in &platforms {
                if let Some(global_agents_dir) = platform.global_agents_dir() {
                    let removed = remove_masday_files(&global_agents_dir)?;
                    if removed > 0 {
                        println!(
                            "  {} global agents: {} removed",
                            style(platform.name()).green(),
                            style(removed).dim()
                        );
                    }
                }
            }

            // Update global settings to remove masday entries
            let settings_path = home.join(".claude/settings.json");
            remove_masday_entries(&settings_path)?;
            println!("  {}", style("Global settings updated").green());
        }
    }

    // Summary
    println!();
    println!("{}", style("Uninstallation complete!").green().bold());
    println!();

    if args.global {
        println!("Global components removed:");
        println!("  - Global hooks");
        println!("  - Global agents");
        println!("  - Global skills");
        println!("  - Claude Code settings entries");
    } else {
        println!("Project-only uninstall. To remove global components, use:");
        println!("  {}", style("masday uninstall --global").cyan());
    }

    Ok(())
}

/// Resolve target platforms
fn resolve_platforms(platform_arg: &Option<String>, project_dir: &Path) -> Result<Vec<Platform>> {
    if let Some(ref name) = platform_arg {
        match name.to_lowercase().as_str() {
            "claude-code" | "claude" => Ok(vec![Platform::ClaudeCode]),
            "claude-desktop" => Ok(vec![Platform::ClaudeDesktop]),
            "gemini" => Ok(vec![Platform::GeminiCli]),
            "vscode" | "copilot" => Ok(vec![Platform::VsCodeCopilot]),
            "opencode" => Ok(vec![Platform::OpenCode]),
            "cursor" => Ok(vec![Platform::Cursor]),
            "windsurf" => Ok(vec![Platform::Windsurf]),
            "codex" => Ok(vec![Platform::Codex]),
            "zcode" => Ok(vec![Platform::Zcode]),
            _ => Ok(all_platforms()),
        }
    } else {
        let detected = detect_active_platforms(project_dir);
        if detected.is_empty() {
            Ok(all_platforms())
        } else {
            Ok(detected)
        }
    }
}

/// Format platform list
fn platform_list(platforms: &[Platform]) -> String {
    platforms
        .iter()
        .map(|p| p.name())
        .collect::<Vec<_>>()
        .join(", ")
}

/// Remove masday-* files from a directory
fn remove_masday_files(dir: &Path) -> Result<usize> {
    if !dir.exists() {
        return Ok(0);
    }

    let mut removed = 0;

    for entry in
        fs::read_dir(dir).with_context(|| format!("Failed to read directory {}", dir.display()))?
    {
        let entry = entry?;
        let file_name = entry.file_name();

        if file_name.to_string_lossy().starts_with("masday-") {
            fs::remove_file(entry.path())
                .with_context(|| format!("Failed to remove {}", entry.path().display()))?;
            removed += 1;
        }
    }

    Ok(removed)
}

/// Remove masday-* directories from a directory
fn remove_masday_dirs(dir: &Path) -> Result<usize> {
    if !dir.exists() {
        return Ok(0);
    }

    let mut removed = 0;

    for entry in
        fs::read_dir(dir).with_context(|| format!("Failed to read directory {}", dir.display()))?
    {
        let entry = entry?;
        let file_name = entry.file_name();

        if file_name.to_string_lossy().starts_with("masday-") {
            fs::remove_dir_all(entry.path())
                .with_context(|| format!("Failed to remove {}", entry.path().display()))?;
            removed += 1;
        }
    }

    Ok(removed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_remove_masday_files() {
        let temp_dir = TempDir::new().unwrap();
        let dir = temp_dir.path();

        // Create test files
        fs::write(dir.join("masday-agent.md"), "content").unwrap();
        fs::write(dir.join("other-agent.md"), "content").unwrap();
        fs::write(dir.join("masday-another.md"), "content").unwrap();

        let removed = remove_masday_files(dir).unwrap();
        assert_eq!(removed, 2);

        assert!(!dir.join("masday-agent.md").exists());
        assert!(dir.join("other-agent.md").exists());
    }

    #[test]
    fn test_remove_masday_dirs() {
        let temp_dir = TempDir::new().unwrap();
        let dir = temp_dir.path();

        // Create test directories
        fs::create_dir_all(dir.join("masday-skill")).unwrap();
        fs::create_dir_all(dir.join("other-skill")).unwrap();
        fs::create_dir_all(dir.join("masday-another")).unwrap();

        let removed = remove_masday_dirs(dir).unwrap();
        assert_eq!(removed, 2);

        assert!(!dir.join("masday-skill").exists());
        assert!(dir.join("other-skill").exists());
    }

    #[test]
    fn test_remove_masday_files_nonexistent() {
        let temp_dir = TempDir::new().unwrap();
        let dir = temp_dir.path().join("nonexistent");

        let removed = remove_masday_files(&dir).unwrap();
        assert_eq!(removed, 0);
    }

    #[test]
    fn test_uninstall_args_default() {
        let args = UninstallArgs::default();
        assert!(!args.global);
        assert!(args.platform.is_none());
    }

    #[test]
    fn test_resolve_platforms_specific() {
        let temp_dir = TempDir::new().unwrap();
        let project_dir = temp_dir.path();

        let platforms = resolve_platforms(&Some("claude".to_string()), project_dir).unwrap();
        assert_eq!(platforms.len(), 1);
        assert_eq!(platforms[0], Platform::ClaudeCode);
    }
}
