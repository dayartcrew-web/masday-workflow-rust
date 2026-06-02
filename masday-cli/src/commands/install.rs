//! Install command implementation
//!
//! Orchestrates the full installation workflow for Masday in local, remote, or standalone mode.

use anyhow::Result;
use console::style;
use home;
use indicatif::{ProgressBar, ProgressStyle};
use std::path::Path;

use crate::installer::{
    self, all_platforms, check_prerequisites, detect_active_platforms, ensure_env_file,
    generate_mcp_config, install_global_hooks, install_project_hooks, resolve_mcp_binary,
    sync_agents_to_global, sync_agents_to_project, sync_skills_to_global, sync_skills_to_project,
    update_global_settings, verify_remote_url, AgentSyncReport, McpConfig, McpServerConfig,
    Platform, Prerequisites, SettingsUpdates, SkillSyncReport,
};

/// Arguments for the install command
#[derive(Debug, Clone, Default)]
pub struct InstallArgs {
    /// Remote API URL (None = local or standalone mode)
    pub remote: Option<String>,
    /// API key for remote mode
    pub api_key: Option<String>,
    /// Specific platform to install (None = detect or all)
    pub platform: Option<String>,
    /// Force standalone mode (extract templates only, no build)
    pub standalone: bool,
    /// Force local mode (cargo build from source)
    pub local_mode: bool,
    /// Skip cargo build step
    pub skip_build: bool,
    /// Only install to project, skip global sync
    pub local_only: bool,
    /// Force overwrite existing files
    pub force: bool,
}

/// Install mode enum
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum InstallMode {
    /// Build from source (requires Rust toolchain + Cargo.toml)
    Local,
    /// Connect to remote API server
    Remote,
    /// Extract embedded templates only (no build, no API server)
    Standalone,
}

/// Detect which install mode to use based on args and project directory
fn detect_mode(args: &InstallArgs, project_dir: &Path) -> InstallMode {
    // Explicit flags take highest priority
    if args.remote.is_some() {
        return InstallMode::Remote;
    }
    if args.standalone {
        return InstallMode::Standalone;
    }
    if args.local_mode {
        return InstallMode::Local;
    }
    // Auto-detect: if Cargo.toml exists in cwd, use local; otherwise standalone
    if project_dir.join("Cargo.toml").exists() {
        InstallMode::Local
    } else {
        InstallMode::Standalone
    }
}

/// Run the install command
pub fn run(args: InstallArgs, project_dir: &Path) -> Result<()> {
    // Validate mutually exclusive flags
    if args.remote.is_some() && args.standalone {
        anyhow::bail!("Cannot use --remote and --standalone together. Choose one install mode.");
    }
    if args.remote.is_some() && args.local_mode {
        anyhow::bail!("Cannot use --remote and --local together. Choose one install mode.");
    }
    if args.standalone && args.local_mode {
        anyhow::bail!("Cannot use --standalone and --local together. Choose one install mode.");
    }

    let mode = detect_mode(&args, project_dir);
    match mode {
        InstallMode::Local => run_local_install(args, project_dir),
        InstallMode::Remote => run_remote_install(args, project_dir),
        InstallMode::Standalone => run_standalone_install(args, project_dir),
    }
}

// ── Shared sync logic ────────────────────────────────────────────────────────

/// Sync agents, skills, and hooks to project and optionally global dirs.
/// Returns (agent_reports, skill_reports, global_hook_count, project_hook_count).
fn sync_templates(
    project_dir: &Path,
    platforms: &[Platform],
    force: bool,
    local_only: bool,
) -> Result<(Vec<AgentSyncReport>, Vec<SkillSyncReport>, usize, usize)> {
    // Sync agents
    let agent_reports = sync_agents_to_project(project_dir, platforms, force)?;
    for report in &agent_reports {
        println!(
            "  {}: {} copied, {} skipped",
            report.platform,
            style(report.copied).green(),
            style(report.skipped).dim()
        );
    }

    if !local_only {
        let global_agent_reports = sync_agents_to_global(platforms, force)?;
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
    let skill_reports = sync_skills_to_project(project_dir, platforms, force)?;
    for report in &skill_reports {
        println!(
            "  {}: {} copied, {} skipped",
            report.platform,
            style(report.copied).green(),
            style(report.skipped).dim()
        );
    }

    if !local_only {
        let global_skill_reports = sync_skills_to_global(platforms, force)?;
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

    let mut global_hook_count = 0;
    if let Some(home) = home::home_dir() {
        let global_hooks = install_global_hooks(&home)?;
        global_hook_count = global_hooks.copied;
        println!(
            "  Global hooks: {}",
            style(format!("{} installed", global_hook_count)).green()
        );
    }

    let project_hooks = install_project_hooks(project_dir)?;
    let project_hook_count = project_hooks.copied;
    println!(
        "  Project hooks: {}",
        style(format!("{} installed", project_hook_count)).green()
    );

    Ok((
        agent_reports,
        skill_reports,
        global_hook_count,
        project_hook_count,
    ))
}

// ── Standalone mode ──────────────────────────────────────────────────────────

/// Standalone mode installation — extract templates only, no build, no API server
fn run_standalone_install(args: InstallArgs, project_dir: &Path) -> Result<()> {
    println!();
    println!(
        "{}",
        style("Installing Masday (standalone mode)...")
            .cyan()
            .bold()
    );
    println!();

    // Step 1: Resolve platforms
    let platforms = resolve_platforms(&args.platform, project_dir)?;
    println!(
        "{}",
        style(format!(
            "Installing for platforms: {}",
            platform_list(&platforms)
        ))
        .cyan()
    );

    // Step 2: Sync agents, skills, hooks
    println!();
    println!("{}", style("Syncing agents...").cyan());
    let (agent_reports, skill_reports, _gh, _ph) =
        sync_templates(project_dir, &platforms, args.force, args.local_only)?;

    // Step 3: Count totals
    let total_agents: usize = agent_reports.iter().map(|r| r.copied).sum();
    let total_skills: usize = skill_reports.iter().map(|r| r.copied).sum();

    // Success summary
    println!();
    println!(
        "{}",
        style("Installation complete! (standalone mode)")
            .green()
            .bold()
    );
    println!();
    println!("Installed:");
    println!("  {} agents", style(total_agents).green());
    println!("  {} skills", style(total_skills).green());
    println!();
    println!("Next steps:");
    println!("  Agents and skills are available in your project.");
    println!("  For full MCP tools support, connect to an API server:");
    println!(
        "  {}",
        style("masday install --remote <url> --api-key <key>").cyan()
    );

    Ok(())
}

// ── Local mode ───────────────────────────────────────────────────────────────

/// Local mode installation
fn run_local_install(args: InstallArgs, project_dir: &Path) -> Result<()> {
    println!();
    println!(
        "{}",
        style("Installing Masday (local mode)...").cyan().bold()
    );
    println!();

    // Step 1: Check prerequisites
    let prereq = check_prerequisites(false)?;
    print_prerequisites(&prereq);

    if !prereq.cargo_available {
        anyhow::bail!("Cargo is required for local mode. Install Rust toolchain first.");
    }

    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner} {msg}")
            .unwrap(),
    );

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
        style(format!(
            "Installing for platforms: {}",
            platform_list(&platforms)
        ))
        .cyan()
    );

    // Step 6: Sync agents, skills, hooks
    println!();
    println!("{}", style("Syncing agents...").cyan());
    sync_templates(project_dir, &platforms, args.force, args.local_only)?;

    // Step 7: Generate MCP configs
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

    // Step 8: Update global settings
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

// ── Remote mode ──────────────────────────────────────────────────────────────

/// Remote mode installation
fn run_remote_install(args: InstallArgs, project_dir: &Path) -> Result<()> {
    println!();
    println!(
        "{}",
        style("Installing Masday (remote mode)...").cyan().bold()
    );
    println!();

    let remote_url = args
        .remote
        .as_ref()
        .expect("remote URL required in remote mode");

    // Step 1: Check prerequisites (node only for remote mode)
    let prereq = check_prerequisites(true)?;
    print_prerequisites(&prereq);

    if !prereq.node_available {
        anyhow::bail!("Node.js is required. Install Node.js first.");
    }

    // Step 2: Verify remote URL connectivity
    println!("{}", style("Checking remote API connectivity...").cyan());
    verify_remote_url(remote_url)?;

    // Step 3: Resolve MCP binary
    let mcp_binary = resolve_mcp_binary(remote_url)?;

    // Step 4: Resolve platforms
    let platforms = resolve_platforms(&args.platform, project_dir)?;
    println!(
        "{}",
        style(format!(
            "Installing for platforms: {}",
            platform_list(&platforms)
        ))
        .cyan()
    );

    // Step 5: Sync agents, skills, hooks
    println!();
    println!("{}", style("Syncing agents...").cyan());
    sync_templates(project_dir, &platforms, args.force, args.local_only)?;

    // Step 6: Generate MCP configs (with remote URL)
    println!();
    println!("{}", style("Generating MCP configs...").cyan());

    let api_url = remote_url.to_string();
    let api_key = args.api_key.ok_or_else(|| {
        anyhow::anyhow!(
            "--api-key is required for remote mode.\n\
            Usage: masday install --remote <url> --api-key <key>"
        )
    })?;

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

    // Step 7: Update global settings
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

// ── Helpers ──────────────────────────────────────────────────────────────────

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
                eprintln!(
                    "{}",
                    style("Unknown platform, using all platforms").yellow()
                );
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

    // ── detect_mode tests ────────────────────────────────────────────────────

    #[test]
    fn test_detect_mode_remote() {
        let temp_dir = TempDir::new().unwrap();
        let args = InstallArgs {
            remote: Some("http://example.com".to_string()),
            ..Default::default()
        };
        assert_eq!(detect_mode(&args, temp_dir.path()), InstallMode::Remote);
    }

    #[test]
    fn test_detect_mode_standalone_explicit() {
        let temp_dir = TempDir::new().unwrap();
        let args = InstallArgs {
            standalone: true,
            ..Default::default()
        };
        assert_eq!(detect_mode(&args, temp_dir.path()), InstallMode::Standalone);
    }

    #[test]
    fn test_detect_mode_local_explicit() {
        let temp_dir = TempDir::new().unwrap();
        let args = InstallArgs {
            local_mode: true,
            ..Default::default()
        };
        assert_eq!(detect_mode(&args, temp_dir.path()), InstallMode::Local);
    }

    #[test]
    fn test_detect_mode_auto_local_when_cargo_toml() {
        let temp_dir = TempDir::new().unwrap();
        std::fs::write(temp_dir.path().join("Cargo.toml"), "[workspace]").unwrap();
        let args = InstallArgs::default();
        assert_eq!(detect_mode(&args, temp_dir.path()), InstallMode::Local);
    }

    #[test]
    fn test_detect_mode_auto_standalone_when_no_cargo() {
        let temp_dir = TempDir::new().unwrap();
        // No Cargo.toml in temp dir
        let args = InstallArgs::default();
        assert_eq!(detect_mode(&args, temp_dir.path()), InstallMode::Standalone);
    }

    #[test]
    fn test_detect_mode_remote_overrides_standalone() {
        let temp_dir = TempDir::new().unwrap();
        let args = InstallArgs {
            remote: Some("http://example.com".to_string()),
            standalone: true,
            ..Default::default()
        };
        // --remote takes highest priority
        assert_eq!(detect_mode(&args, temp_dir.path()), InstallMode::Remote);
    }

    // ── Mutual exclusion tests ───────────────────────────────────────────────

    #[test]
    fn test_mutual_exclusion_remote_standalone() {
        let temp_dir = TempDir::new().unwrap();
        let args = InstallArgs {
            remote: Some("http://example.com".to_string()),
            standalone: true,
            ..Default::default()
        };
        let result = run(args, temp_dir.path());
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("--remote and --standalone"));
    }

    #[test]
    fn test_mutual_exclusion_remote_local() {
        let temp_dir = TempDir::new().unwrap();
        let args = InstallArgs {
            remote: Some("http://example.com".to_string()),
            local_mode: true,
            ..Default::default()
        };
        let result = run(args, temp_dir.path());
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("--remote and --local"));
    }

    #[test]
    fn test_mutual_exclusion_standalone_local() {
        let temp_dir = TempDir::new().unwrap();
        let args = InstallArgs {
            standalone: true,
            local_mode: true,
            ..Default::default()
        };
        let result = run(args, temp_dir.path());
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("--standalone and --local"));
    }

    // ── Existing tests ───────────────────────────────────────────────────────

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
        assert!(!args.standalone);
        assert!(!args.local_mode);
        assert!(!args.skip_build);
        assert!(!args.local_only);
        assert!(!args.force);
    }

    // ── Standalone install integration tests ─────────────────────────────────

    #[test]
    fn test_standalone_install_creates_agents() {
        let temp_dir = TempDir::new().unwrap();
        let project_dir = temp_dir.path();

        let args = InstallArgs {
            standalone: true,
            force: true,
            local_only: true, // Avoid writing to global dirs in tests
            ..Default::default()
        };

        let result = run(args, project_dir);
        assert!(result.is_ok(), "Standalone install should succeed");

        // Check that agents were extracted
        let agents_dir = project_dir.join(".claude/agents");
        if agents_dir.exists() {
            let agent_count = std::fs::read_dir(&agents_dir)
                .unwrap()
                .filter(|e| {
                    e.as_ref()
                        .map(|f| f.file_name().to_string_lossy().ends_with(".md"))
                        .unwrap_or(false)
                })
                .count();
            assert!(agent_count > 0, "Should have extracted at least one agent");
        }
    }

    #[test]
    fn test_standalone_install_no_env() {
        let temp_dir = TempDir::new().unwrap();
        let project_dir = temp_dir.path();

        let args = InstallArgs {
            standalone: true,
            force: true,
            local_only: true,
            ..Default::default()
        };

        run(args, project_dir).unwrap();

        // Standalone mode should NOT create .env
        assert!(
            !project_dir.join(".env").exists(),
            "Standalone mode should not create .env file"
        );
    }

    #[test]
    fn test_standalone_install_no_mcp_config() {
        let temp_dir = TempDir::new().unwrap();
        let project_dir = temp_dir.path();

        let args = InstallArgs {
            standalone: true,
            force: true,
            local_only: true,
            ..Default::default()
        };

        run(args, project_dir).unwrap();

        // Standalone mode should NOT create MCP config
        assert!(
            !project_dir.join(".mcp.json").exists(),
            "Standalone mode should not create .mcp.json"
        );
    }
}
