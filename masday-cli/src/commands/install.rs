//! Install command implementation
//!
//! Orchestrates the full installation workflow for Masday in local or remote mode.

use std::path::Path;
use anyhow::Result;
use console::style;
use home;
use indicatif::{ProgressBar, ProgressStyle};

use crate::installer::{
    self,
    Platform,
    Prerequisites,
    check_prerequisites,
    ensure_env_file,
    detect_active_platforms,
    all_platforms,
    sync_agents_to_project,
    sync_agents_to_global,
    sync_skills_to_project,
    sync_skills_to_global,
    install_global_hooks,
    install_project_hooks,
    generate_mcp_config,
    update_global_settings,
    verify_remote_url,
    resolve_mcp_binary,
    McpConfig,
    SettingsUpdates,
    McpServerConfig,
};

/// Arguments for the install command
#[derive(Debug, Clone, Default)]
pub struct InstallArgs {
    /// Remote API URL (None = local mode)
    pub remote: Option<String>,
    /// API key for remote mode
    pub api_key: Option<String>,
    /// Specific platform to install (None = detect or all)
    pub platform: Option<String>,
    /// Skip cargo build step
    pub skip_build: bool,
    /// Only install to project, skip global sync
    pub local_only: bool,
    /// Force overwrite existing files
    pub force: bool,
}

/// Run the install command
pub fn run(args: InstallArgs, project_dir: &Path) -> Result<()> {
    let is_remote = args.remote.is_some();

    if is_remote {
        run_remote_install(args, project_dir)?;
    } else {
        run_local_install(args, project_dir)?;
    }

    Ok(())
}

/// Local mode installation
fn run_local_install(args: InstallArgs, project_dir: &Path) -> Result<()> {
    println!();
    println!("{}", style("Installing Masday (local mode)...").cyan().bold());
    println!();

    // Step 1: Check prerequisites
    let prereq = check_prerequisites(false)?;
    print_prerequisites(&prereq);

    if !prereq.cargo_available {
        anyhow::bail!("Cargo is required for local mode. Install Rust toolchain first.");
    }

    let pb = ProgressBar::new_spinner();
    pb.set_style(ProgressStyle::default_spinner()
        .template("{spinner} {msg}")
        .unwrap());

    // Step 2: Build crates (unless --skip-build)
    if !args.skip_build {
        pb.set_message("Building crates...");
        pb.enable_steady_tick(std::time::Duration::from_millis(100));
        installer::build_crates(project_dir)?;
        pb.finish_with_message("Build complete");
    }

    // Step 3: Ensure .env file
    pb.set_message("Setting up .env...");
    pb.enable_steady_tick(std::time::Duration::from_millis(100));
    ensure_env_file(project_dir)?;
    pb.finish_with_message(".env ready");

    // Step 4: Find MCP binary
    let mcp_binary = installer::find_mcp_binary(project_dir)?;
    println!(
        "{}",
        style(format!("Found MCP binary: {}", mcp_binary.display())).green()
    );

    // Step 5: Resolve platforms
    let platforms = resolve_platforms(&args.platform, project_dir)?;
    println!(
        "{}",
        style(format!("Installing for platforms: {}", platform_list(&platforms))).cyan()
    );

    // Step 6: Sync agents
    println!();
    println!("{}", style("Syncing agents...").cyan());
    let agent_reports = sync_agents_to_project(project_dir, &platforms, args.force)?;
    for report in &agent_reports {
        println!(
            "  {}: {} copied, {} skipped",
            report.platform,
            style(report.copied).green(),
            style(report.skipped).dim()
        );
    }

    if !args.local_only {
        let global_agent_reports = sync_agents_to_global(&platforms, args.force)?;
        for report in &global_agent_reports {
            println!(
                "  {}: {} copied, {} skipped",
                report.platform,
                style(report.copied).green(),
                style(report.skipped).dim()
            );
        }
    }

    // Step 7: Sync skills
    println!();
    println!("{}", style("Syncing skills...").cyan());
    let skill_reports = sync_skills_to_project(project_dir, &platforms, args.force)?;
    for report in &skill_reports {
        println!(
            "  {}: {} copied, {} skipped",
            report.platform,
            style(report.copied).green(),
            style(report.skipped).dim()
        );
    }

    if !args.local_only {
        let global_skill_reports = sync_skills_to_global(&platforms, args.force)?;
        for report in &global_skill_reports {
            println!(
                "  {}: {} copied, {} skipped",
                report.platform,
                style(report.copied).green(),
                style(report.skipped).dim()
            );
        }
    }

    // Step 8: Install hooks
    println!();
    println!("{}", style("Installing hooks...").cyan());

    if let Some(home) = home::home_dir() {
        let global_hooks = install_global_hooks(&home)?;
        println!(
            "  Global hooks: {}",
            style(format!("{} installed", global_hooks.copied)).green()
        );
    }

    let project_hooks = install_project_hooks(project_dir)?;
    println!(
        "  Project hooks: {}",
        style(format!("{} installed", project_hooks.copied)).green()
    );

    // Step 9: Generate MCP configs
    println!();
    println!("{}", style("Generating MCP configs...").cyan());

    let api_url = "http://localhost:3010".to_string();
    let api_key = "local-mode".to_string();

    for platform in &platforms {
        let config = McpConfig {
            mcp_binary_path: mcp_binary.clone(),
            api_url: api_url.clone(),
            api_key: api_key.clone(),
            database_url: None,
        };

        generate_mcp_config(platform, project_dir, &config)?;
        println!("  {}", style(platform.name()).green());
    }

    // Step 10: Update global settings
    if let Some(home) = home::home_dir() {
        let settings_path = home.join(".claude/settings.json");
        let hook_path = home.join(".claude/hooks/masday-statusline.js");

        let updates = SettingsUpdates {
            statusline_cmd: Some(hook_path.display().to_string()),
            auto_compact: Some(true),
            auto_compact_threshold: Some(0.9),
            mcp_server: Some(McpServerConfig {
                command: mcp_binary.display().to_string(),
                env: vec![
                    ("MASDAY_API_URL".to_string(), api_url),
                    ("MASDAY_API_KEY".to_string(), api_key),
                ],
            }),
        };

        update_global_settings(&settings_path, &updates)?;
        println!();
        println!(
            "{}",
            style("Global settings updated (statusline, autoCompact, MCP server)").green()
        );
    }

    // Success summary
    println!();
    println!("{}", style("Installation complete!").green().bold());
    println!();
    println!("Next steps:");
    println!("  1. Start the MCP server: cargo run -p masday-mcp");
    println!("  2. Start the API server: cargo run -p masday-api");
    println!("  3. Verify installation: masday status");

    Ok(())
}

/// Remote mode installation
fn run_remote_install(args: InstallArgs, project_dir: &Path) -> Result<()> {
    println!();
    println!("{}", style("Installing Masday (remote mode)...").cyan().bold());
    println!();

    let remote_url = args.remote.as_ref().expect("remote URL required in remote mode");

    // Step 1: Check prerequisites (node only for remote mode)
    let prereq = check_prerequisites(true)?;
    print_prerequisites(&prereq);

    if !prereq.node_available {
        anyhow::bail!("Node.js is required. Install Node.js first.");
    }

    // Step 2: Verify remote URL connectivity
    println!(
        "{}",
        style("Checking remote API connectivity...").cyan()
    );
    verify_remote_url(remote_url)?;

    // Step 3: Resolve MCP binary
    let mcp_binary = resolve_mcp_binary(remote_url)?;

    // Step 4-10: Same as local mode (platforms, sync, hooks, configs, settings)
    // Resolve platforms
    let platforms = resolve_platforms(&args.platform, project_dir)?;
    println!(
        "{}",
        style(format!("Installing for platforms: {}", platform_list(&platforms))).cyan()
    );

    // Sync agents
    println!();
    println!("{}", style("Syncing agents...").cyan());
    let agent_reports = sync_agents_to_project(project_dir, &platforms, args.force)?;
    for report in &agent_reports {
        println!(
            "  {}: {} copied, {} skipped",
            report.platform,
            style(report.copied).green(),
            style(report.skipped).dim()
        );
    }

    if !args.local_only {
        let global_agent_reports = sync_agents_to_global(&platforms, args.force)?;
        for report in &global_agent_reports {
            println!(
                "  {}: {} copied, {} skipped",
                report.platform,
                style(report.copied).green(),
                style(report.skipped).dim()
            );
        }
    }

    // Sync skills
    println!();
    println!("{}", style("Syncing skills...").cyan());
    let skill_reports = sync_skills_to_project(project_dir, &platforms, args.force)?;
    for report in &skill_reports {
        println!(
            "  {}: {} copied, {} skipped",
            report.platform,
            style(report.copied).green(),
            style(report.skipped).dim()
        );
    }

    if !args.local_only {
        let global_skill_reports = sync_skills_to_global(&platforms, args.force)?;
        for report in &global_skill_reports {
            println!(
                "  {}: {} copied, {} skipped",
                report.platform,
                style(report.copied).green(),
                style(report.skipped).dim()
            );
        }
    }

    // Install hooks
    println!();
    println!("{}", style("Installing hooks...").cyan());

    if let Some(home) = home::home_dir() {
        let global_hooks = install_global_hooks(&home)?;
        println!(
            "  Global hooks: {}",
            style(format!("{} installed", global_hooks.copied)).green()
        );
    }

    let project_hooks = install_project_hooks(project_dir)?;
    println!(
        "  Project hooks: {}",
        style(format!("{} installed", project_hooks.copied)).green()
    );

    // Generate MCP configs (with remote URL)
    println!();
    println!("{}", style("Generating MCP configs...").cyan());

    let api_url = remote_url.to_string();
    let api_key = args.api_key.unwrap_or_else(|| "remote-mode".to_string());

    for platform in &platforms {
        let config = McpConfig {
            mcp_binary_path: mcp_binary.clone(),
            api_url: api_url.clone(),
            api_key: api_key.clone(),
            database_url: None,
        };

        generate_mcp_config(platform, project_dir, &config)?;
        println!("  {}", style(platform.name()).green());
    }

    // Update global settings
    if let Some(home) = home::home_dir() {
        let settings_path = home.join(".claude/settings.json");
        let hook_path = home.join(".claude/hooks/masday-statusline.js");

        let updates = SettingsUpdates {
            statusline_cmd: Some(hook_path.display().to_string()),
            auto_compact: Some(true),
            auto_compact_threshold: Some(0.9),
            mcp_server: Some(McpServerConfig {
                command: mcp_binary.display().to_string(),
                env: vec![
                    ("MASDAY_API_URL".to_string(), api_url),
                    ("MASDAY_API_KEY".to_string(), api_key),
                ],
            }),
        };

        update_global_settings(&settings_path, &updates)?;
        println!();
        println!(
            "{}",
            style("Global settings updated (statusline, autoCompact, MCP server)").green()
        );
    }

    // Success summary
    println!();
    println!("{}", style("Installation complete!").green().bold());
    println!();
    println!("Remote mode configured:");
    println!("  API URL: {}", style(remote_url).cyan());
    println!("  MCP binary: {}", style(mcp_binary.display()).cyan());

    Ok(())
}

/// Resolve target platforms based on args and detection
fn resolve_platforms(platform_arg: &Option<String>, project_dir: &Path) -> Result<Vec<Platform>> {
    if let Some(ref name) = platform_arg {
        match name.to_lowercase().as_str() {
            "claude-code" => Ok(vec![Platform::ClaudeCode]),
            "claude" => Ok(vec![Platform::ClaudeCode]),
            "gemini" => Ok(vec![Platform::GeminiCli]),
            "vscode" => Ok(vec![Platform::VsCodeCopilot]),
            "copilot" => Ok(vec![Platform::VsCodeCopilot]),
            "opencode" => Ok(vec![Platform::OpenCode]),
            _ => {
                eprintln!("{}", style("Unknown platform, using all platforms").yellow());
                Ok(all_platforms())
            }
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

/// Format platform list for display
fn platform_list(platforms: &[Platform]) -> String {
    platforms
        .iter()
        .map(|p| p.name())
        .collect::<Vec<_>>()
        .join(", ")
}

/// Print prerequisites check results
fn print_prerequisites(prereq: &Prerequisites) {
    println!("{}", style("Prerequisites check:").cyan());

    let cargo_status = if prereq.cargo_available {
        style("✓").green()
    } else {
        style("✗").red()
    };

    let node_status = if prereq.node_available {
        style("✓").green()
    } else {
        style("✗").red()
    };

    let pnpm_status = if prereq.pnpm_available {
        style("✓").green()
    } else {
        style("✗").red()
    };

    println!("  Cargo: {}", cargo_status);
    println!("  Node.js: {}", node_status);
    println!("  pnpm: {}", pnpm_status);
    println!();
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_resolve_platforms_specific() {
        let temp_dir = TempDir::new().unwrap();
        let project_dir = temp_dir.path();

        let platforms = resolve_platforms(&Some("claude-code".to_string()), project_dir).unwrap();
        assert_eq!(platforms.len(), 1);
        assert_eq!(platforms[0], Platform::ClaudeCode);
    }

    #[test]
    fn test_resolve_platforms_all_when_empty() {
        let temp_dir = TempDir::new().unwrap();
        let project_dir = temp_dir.path();

        let platforms = resolve_platforms(&None, project_dir).unwrap();
        // No active platforms detected, should return all
        assert_eq!(platforms.len(), 4);
    }

    #[test]
    fn test_platform_list() {
        let platforms = vec![Platform::ClaudeCode, Platform::GeminiCli];
        let list = platform_list(&platforms);
        assert!(list.contains("claude-code"));
        assert!(list.contains("gemini"));
    }

    #[test]
    fn test_install_args_default() {
        let args = InstallArgs::default();
        assert!(args.remote.is_none());
        assert!(args.api_key.is_none());
        assert!(!args.skip_build);
        assert!(!args.local_only);
        assert!(!args.force);
    }
}
