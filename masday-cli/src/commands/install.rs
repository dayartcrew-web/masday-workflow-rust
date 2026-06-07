//! Install command implementation (DEPRECATED)
//!
//! This command is deprecated. Use `masday quickstart` instead.

use anyhow::Result;
use console::style;
use std::path::Path;

/// Install mode enum
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstallMode {
    /// Build from source (requires Rust toolchain + Cargo.toml)
    Local,
    /// Connect to remote API server
    Remote,
    /// Extract embedded templates only (no build, no API server)
    Standalone,
}

impl InstallMode {
    /// Display name for mode
    pub fn as_str(&self) -> &'static str {
        match self {
            InstallMode::Local => "local",
            InstallMode::Remote => "remote",
            InstallMode::Standalone => "standalone",
        }
    }
}

impl std::str::FromStr for InstallMode {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "local" => Ok(InstallMode::Local),
            "remote" => Ok(InstallMode::Remote),
            "standalone" => Ok(InstallMode::Standalone),
            _ => Err(format!(
                "Invalid mode: '{}'. Must be local, remote, or standalone",
                s
            )),
        }
    }
}

/// Arguments for the install command
#[derive(Debug, Clone, Default)]
pub struct InstallArgs {
    /// Install mode (None = defaults to standalone)
    pub mode: Option<InstallMode>,
    /// Remote API URL (implies remote mode)
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
    /// Skip hook installation
    pub no_hooks: bool,
    /// Skip MCP server registration
    pub no_mcp: bool,
}

/// Resolve which install mode to use based on args.
fn resolve_mode(args: &InstallArgs) -> InstallMode {
    // Explicit --mode flag takes highest priority
    if let Some(mode) = args.mode {
        return mode;
    }

    // --remote flag implies remote mode
    if args.remote.is_some() {
        return InstallMode::Remote;
    }

    // Default: standalone mode (no build, no API server)
    InstallMode::Standalone
}

/// Run the install command
pub fn run(args: InstallArgs, _project_dir: &Path) -> Result<()> {
    // Deprecation warning
    eprintln!();
    eprintln!(
        "{}",
        style("⚠ masday install is deprecated").yellow().bold()
    );
    eprintln!("{}", style("  Use 'masday quickstart' instead.").yellow());
    eprintln!();

    let mode = resolve_mode(&args);
    match mode {
        InstallMode::Local => run_local_install(args),
        InstallMode::Remote => run_remote_install(args),
        InstallMode::Standalone => run_standalone_install(args),
    }
}

// ── Standalone mode ──────────────────────────────────────────────────────────

/// Standalone mode installation — thin wrapper with deprecation notice
fn run_standalone_install(args: InstallArgs) -> Result<()> {
    println!();
    println!(
        "{}",
        style("⚠ Standalone mode via 'masday install' is deprecated")
            .yellow()
            .bold()
    );
    println!();
    println!(
        "{}",
        style("Use 'masday quickstart --mode standalone' instead.").cyan()
    );
    println!();

    let cmd = build_quickstart_command("standalone", &args);
    print_quickstart_command(&cmd);

    Ok(())
}

// ── Local mode ───────────────────────────────────────────────────────────────

/// Local mode installation — redirect to quickstart
fn run_local_install(args: InstallArgs) -> Result<()> {
    println!();
    println!(
        "{}",
        style("⚠ Local mode via 'masday install' is deprecated")
            .yellow()
            .bold()
    );
    println!();
    println!(
        "{}",
        style("Use 'masday quickstart --mode local' instead.").cyan()
    );
    println!();

    // Show quickstart help
    println!("Quickstart provides:");
    println!("  • Database setup (Docker prompt or --database-url)");
    println!("  • Template extraction (agents, skills, hooks)");
    println!("  • MCP server configuration");
    println!("  • One-command setup with auto-prompts");
    println!();

    let cmd = build_quickstart_command("local", &args);
    print_quickstart_command(&cmd);

    Ok(())
}

// ── Remote mode ──────────────────────────────────────────────────────────────

/// Remote mode installation — thin wrapper with deprecation notice
fn run_remote_install(args: InstallArgs) -> Result<()> {
    println!();
    println!(
        "{}",
        style("⚠ Remote mode via 'masday install' is deprecated")
            .yellow()
            .bold()
    );
    println!();
    println!(
        "{}",
        style("Use 'masday quickstart --mode remote' instead.").cyan()
    );
    println!();

    let cmd = build_quickstart_command("remote", &args);
    print_quickstart_command(&cmd);

    Ok(())
}

// ── Helper functions ─────────────────────────────────────────────────────────────

/// Build quickstart command line based on mode and args
fn build_quickstart_command(mode: &str, args: &InstallArgs) -> Vec<String> {
    let mut cmd = vec![
        "masday".to_string(),
        "quickstart".to_string(),
        "--mode".to_string(),
        mode.to_string(),
    ];

    // Remote mode flags
    if let Some(ref remote) = args.remote {
        cmd.extend(["--remote".to_string(), remote.clone()]);
    }

    if let Some(ref api_key) = args.api_key {
        cmd.extend(["--api-key".to_string(), api_key.clone()]);
    }

    // Common flags
    if args.local_only {
        cmd.push("--local-only".to_string());
    }

    if args.force {
        cmd.push("--force".to_string());
    }

    if args.no_hooks {
        cmd.push("--no-hooks".to_string());
    }

    if args.no_mcp {
        cmd.push("--no-mcp".to_string());
    }

    if let Some(ref platform) = args.platform {
        cmd.extend(["--platform".to_string(), platform.clone()]);
    }

    cmd
}

/// Print the quickstart command with styling
fn print_quickstart_command(cmd: &[String]) {
    println!("{}", style("Run this command:").cyan().bold());
    println!("  {}", style(cmd.join(" ")).green());
    println!();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_mode_explicit_local() {
        let args = InstallArgs {
            mode: Some(InstallMode::Local),
            ..Default::default()
        };
        assert_eq!(resolve_mode(&args), InstallMode::Local);
    }

    #[test]
    fn test_resolve_mode_explicit_remote() {
        let args = InstallArgs {
            mode: Some(InstallMode::Remote),
            ..Default::default()
        };
        assert_eq!(resolve_mode(&args), InstallMode::Remote);
    }

    #[test]
    fn test_resolve_mode_explicit_standalone() {
        let args = InstallArgs {
            mode: Some(InstallMode::Standalone),
            ..Default::default()
        };
        assert_eq!(resolve_mode(&args), InstallMode::Standalone);
    }

    #[test]
    fn test_resolve_mode_remote_flag_implies_remote() {
        let args = InstallArgs {
            remote: Some("http://example.com".to_string()),
            ..Default::default()
        };
        assert_eq!(resolve_mode(&args), InstallMode::Remote);
    }

    #[test]
    fn test_resolve_mode_defaults_to_standalone() {
        let args = InstallArgs::default();
        assert_eq!(resolve_mode(&args), InstallMode::Standalone);
    }

    #[test]
    fn test_install_mode_from_str() {
        assert_eq!("local".parse::<InstallMode>().unwrap(), InstallMode::Local);
        assert_eq!(
            "remote".parse::<InstallMode>().unwrap(),
            InstallMode::Remote
        );
        assert_eq!(
            "standalone".parse::<InstallMode>().unwrap(),
            InstallMode::Standalone
        );
        assert!("invalid".parse::<InstallMode>().is_err());
    }

    // ── Deprecation message tests ─────────────────────────────────────────────

    #[test]
    fn test_install_mode_as_str() {
        assert_eq!(InstallMode::Local.as_str(), "local");
        assert_eq!(InstallMode::Remote.as_str(), "remote");
        assert_eq!(InstallMode::Standalone.as_str(), "standalone");
    }

    #[test]
    fn test_install_args_default() {
        let args = InstallArgs::default();
        assert!(args.mode.is_none());
        assert!(args.remote.is_none());
        assert!(args.api_key.is_none());
        assert!(args.platform.is_none());
        assert!(!args.skip_build);
        assert!(!args.local_only);
        assert!(!args.force);
        assert!(!args.no_hooks);
        assert!(!args.no_mcp);
    }

    #[test]
    fn test_install_args_with_mode() {
        let args = InstallArgs {
            mode: Some(InstallMode::Local),
            ..Default::default()
        };
        assert_eq!(args.mode, Some(InstallMode::Local));
    }

    #[test]
    fn test_install_args_with_remote() {
        let args = InstallArgs {
            remote: Some("https://example.com".to_string()),
            ..Default::default()
        };
        assert_eq!(args.remote, Some("https://example.com".to_string()));
        // Remote flag should imply Remote mode via resolve_mode
        assert_eq!(resolve_mode(&args), InstallMode::Remote);
    }

    #[test]
    fn test_install_args_with_api_key() {
        let args = InstallArgs {
            api_key: Some("secret-key".to_string()),
            ..Default::default()
        };
        assert_eq!(args.api_key, Some("secret-key".to_string()));
    }

    #[test]
    fn test_install_args_with_platform() {
        let args = InstallArgs {
            platform: Some("claude-code".to_string()),
            ..Default::default()
        };
        assert_eq!(args.platform, Some("claude-code".to_string()));
    }

    #[test]
    fn test_install_args_with_skip_build() {
        let args = InstallArgs {
            skip_build: true,
            ..Default::default()
        };
        assert!(args.skip_build);
    }

    #[test]
    fn test_install_args_with_local_only() {
        let args = InstallArgs {
            local_only: true,
            ..Default::default()
        };
        assert!(args.local_only);
    }

    #[test]
    fn test_install_args_with_force() {
        let args = InstallArgs {
            force: true,
            ..Default::default()
        };
        assert!(args.force);
    }

    #[test]
    fn test_install_args_with_no_hooks() {
        let args = InstallArgs {
            no_hooks: true,
            ..Default::default()
        };
        assert!(args.no_hooks);
    }

    #[test]
    fn test_install_args_with_no_mcp() {
        let args = InstallArgs {
            no_mcp: true,
            ..Default::default()
        };
        assert!(args.no_mcp);
    }

    #[test]
    fn test_install_mode_case_insensitive() {
        assert_eq!("LOCAL".parse::<InstallMode>().unwrap(), InstallMode::Local);
        assert_eq!("Remote".parse::<InstallMode>().unwrap(), InstallMode::Remote);
        assert_eq!("STANDALONE".parse::<InstallMode>().unwrap(), InstallMode::Standalone);
    }
}
