//! CLI entry point for masday

use clap::{Parser, Subcommand};

use masday_cli::commands::config::ConfigSubcommand;
use masday_cli::commands::embed::EmbedAction;

/// Auto-install masday binary to ~/.masday/bin/ if running from elsewhere.
/// This handles the case where user downloads masday.exe to Desktop/Downloads
/// and runs it directly — it copies itself to the proper location.
fn self_install_if_needed() {
    let home = match home::home_dir() {
        Some(h) => h,
        None => return,
    };

    let install_dir = home.join(".masday").join("bin");
    let dest = install_dir.join(if cfg!(windows) {
        "masday.exe"
    } else {
        "masday"
    });

    // Already installed in the right place — skip
    if let Ok(current) = std::env::current_exe() {
        if current == dest {
            return;
        }

        // If running from ~/.masday/bin/ (even with different name), skip
        if let Some(parent) = current.parent() {
            if parent == install_dir {
                return;
            }
        }

        // Copy self to ~/.masday/bin/
        let current_path = current;
        let should_install = !dest.exists()
            || std::fs::metadata(&current_path)
                .and_then(|m| m.modified())
                .ok()
                .is_none_or(|current_mtime| {
                    std::fs::metadata(&dest)
                        .and_then(|m| m.modified())
                        .ok()
                        .is_none_or(|dest_mtime| current_mtime > dest_mtime)
                });

        if should_install {
            let _ = std::fs::create_dir_all(&install_dir);

            match std::fs::copy(&current_path, &dest) {
                Ok(_) => {
                    #[cfg(unix)]
                    {
                        let _ = std::fs::set_permissions(
                            &dest,
                            std::os::unix::fs::PermissionsExt::from_mode(0o755),
                        );
                    }

                    // Add ~/.masday/bin to PATH if not already there
                    add_to_path(&install_dir);

                    eprintln!("✓ Installed to {}", dest.display());

                    // Check if current PATH includes install dir
                    let path_var = std::env::var("PATH").unwrap_or_default();
                    let path_sep = if cfg!(windows) { ';' } else { ':' };
                    let in_path = path_var
                        .split(path_sep)
                        .any(|p| std::path::Path::new(p) == install_dir);

                    if !in_path {
                        eprintln!(
                            "  Add to PATH: export PATH=\"$PATH:{}\"",
                            install_dir.display()
                        );
                        eprintln!("  Or restart your terminal.");
                    }
                    eprintln!();
                }
                Err(e) => {
                    eprintln!("⚠ Could not install to {}: {}", dest.display(), e);
                    eprintln!("  Continuing from current location.");
                }
            }
        }
    }
}

/// Add directory to shell PATH in .bashrc/.zshrc (Linux/macOS)
fn add_to_path(dir: &std::path::Path) {
    #[cfg(unix)]
    {
        let path_line = format!("export PATH=\"$PATH:{}\"", dir.display());
        let home = match home::home_dir() {
            Some(h) => h,
            None => return,
        };
        for rc_file in &[".bashrc", ".zshrc", ".profile"] {
            let rc_path = home.join(rc_file);
            if rc_path.exists() {
                if let Ok(content) = std::fs::read_to_string(&rc_path) {
                    if content.contains(dir.to_str().unwrap_or("")) {
                        continue;
                    }
                    if let Ok(mut f) = std::fs::OpenOptions::new().append(true).open(&rc_path) {
                        use std::io::Write;
                        let _ = writeln!(f);
                        let _ = writeln!(f, "# Masday CLI");
                        let _ = writeln!(f, "{}", path_line);
                    }
                }
            }
        }
    }
    #[cfg(windows)]
    {
        let _ = dir; // suppress unused warning
                     // Windows PATH is handled by install.sh / PowerShell script
    }
}

/// Build version string with build origin: "0.3.19 (production)" or "0.3.19 (development)"
const FULL_VERSION: &str = concat!(env!("CARGO_PKG_VERSION"), " (production)");

#[derive(Parser)]
#[command(name = "masday")]
#[command(about = "Masday workflow orchestration — all-in-one binary", long_about = None)]
#[command(version = FULL_VERSION)]
struct Cli {
    /// Enable verbose output
    #[arg(short = 'v', long = "verbose", global = true)]
    verbose: bool,

    /// Suppress non-error output
    #[arg(short = 'q', long = "quiet", global = true)]
    quiet: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Interactive setup wizard (first-time configuration)
    Setup,

    /// One-command setup — db + migrate + install + ready
    Quickstart,

    /// Start API server + dashboard
    Serve {
        /// Port to listen on (overrides config)
        #[arg(long)]
        port: Option<u16>,
    },

    /// Start MCP server (stdio — used by AI platforms)
    Mcp,

    /// Check health of all services
    Status {
        /// Output as JSON (for scripting)
        #[arg(long = "json")]
        json: bool,

        /// Show detailed component info
        #[arg(long = "verbose")]
        verbose: bool,
    },

    /// Diagnose installation and configuration issues
    Doctor {
        /// Output as JSON (for scripting)
        #[arg(long = "json")]
        json: bool,
    },

    /// View and manage configuration
    Config {
        #[command(subcommand)]
        action: ConfigSubcommand,
    },

    /// Manage PostgreSQL via Docker
    Db {
        #[command(subcommand)]
        action: DbAction,
    },

    /// Install masday into the current project
    Install {
        /// Install mode: local | remote | standalone
        #[arg(long)]
        mode: Option<String>,

        /// Remote API server URL (skips local build, connects to remote)
        #[arg(long)]
        remote: Option<String>,

        /// API key for remote server
        #[arg(long)]
        api_key: Option<String>,

        /// Install only for specific platform (claude-code, gemini, vscode, opencode)
        #[arg(long)]
        platform: Option<String>,

        /// Skip building Rust crates (dev-mode only, use existing binaries)
        /// Skip cargo build step
        #[arg(long)]
        skip_build: bool,

        /// Skip global directory installation
        #[arg(long)]
        local_only: bool,

        /// Force overwrite existing configs
        #[arg(long)]
        force: bool,

        /// Skip hook installation
        #[arg(long)]
        no_hooks: bool,

        /// Skip MCP server registration
        #[arg(long)]
        no_mcp: bool,
    },

    /// Remove masday from the current project
    Uninstall {
        /// Remove from global directories too
        #[arg(long)]
        global: bool,

        /// Remove only for specific platform
        #[arg(long)]
        platform: Option<String>,
    },

    /// Manage local embedding runtime (ONNX Runtime + models)
    Embed {
        #[command(subcommand)]
        action: EmbedAction,
    },

    /// Update masday (re-install with force)
    Update {
        /// Check for available update without applying
        #[arg(long)]
        check: bool,

        /// Update to specific version (default: latest)
        #[arg(long)]
        version: Option<String>,

        /// Only update agents/skills/hooks, not the binary
        #[arg(long)]
        skip_binary: bool,

        /// Don't overwrite config.toml
        #[arg(long)]
        skip_config: bool,

        /// Show what would be updated without changing anything
        #[arg(long)]
        dry_run: bool,

        /// Force re-install even if already up-to-date
        #[arg(long)]
        force: bool,
    },

    /// Development commands (build from source, local install, serve)
    Dev {
        #[command(subcommand)]
        action: DevAction,
    },
}

/// Dev subcommands (dev-mode only)
#[derive(Subcommand)]
enum DevAction {
    /// Build all crates from source
    Build,

    /// Build + install + sync templates + MCP config
    Install,

    /// Start API server from built binary
    Serve {
        /// Port to listen on (overrides config)
        #[arg(long)]
        port: Option<u16>,
    },
}

#[derive(Subcommand)]
enum DbAction {
    /// Start PostgreSQL and Redis containers
    Start,

    /// Stop all containers
    Stop,

    /// Delete data and recreate containers
    Reset,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Auto-install binary to ~/.masday/bin/ if running from elsewhere
    self_install_if_needed();

    let cli = Cli::parse();
    let project_dir = std::env::current_dir()?;

    match cli.command {
        Commands::Setup => {
            masday_cli::commands::setup::run(&project_dir).await?;
        }
        Commands::Quickstart => {
            masday_cli::commands::quickstart::run(&project_dir).await?;
        }
        Commands::Serve { port } => {
            masday_cli::commands::serve::run(port).await?;
        }
        Commands::Mcp => {
            masday_cli::commands::mcp_cmd::run().await?;
        }
        Commands::Status { json, verbose } => {
            masday_cli::commands::status::run(json, verbose).await?;
        }
        Commands::Doctor { json } => {
            masday_cli::commands::doctor::run(json).await?;
        }
        Commands::Config { action } => {
            masday_cli::commands::config::run(action).await?;
        }
        Commands::Db { action } => match action {
            DbAction::Start => masday_cli::commands::db::start()?,
            DbAction::Stop => masday_cli::commands::db::stop()?,
            DbAction::Reset => masday_cli::commands::db::reset().await?,
        },
        Commands::Install {
            mode,
            remote,
            api_key,
            platform,
            skip_build,
            local_only,
            force,
            no_hooks,
            no_mcp,
        } => {
            let install_mode = mode.as_ref().and_then(|m| m.parse().ok());
            let args = masday_cli::commands::install::InstallArgs {
                mode: install_mode,
                remote,
                api_key,
                platform,
                skip_build,
                local_only,
                force,
                no_hooks,
                no_mcp,
            };
            masday_cli::commands::install::run(args, &project_dir)?;
        }
        Commands::Uninstall { global, platform } => {
            let args = masday_cli::commands::uninstall::UninstallArgs { global, platform };
            masday_cli::commands::uninstall::run(args, &project_dir)?;
        }
        Commands::Update {
            check,
            version,
            skip_binary,
            skip_config,
            dry_run,
            force,
        } => {
            let args = masday_cli::commands::update::UpdateArgs {
                check,
                version,
                skip_binary,
                skip_config,
                dry_run,
                force,
            };
            masday_cli::commands::update::run(args, &project_dir)?;
        }
        Commands::Embed { action } => {
            masday_cli::commands::embed::run(action)?;
        }
        Commands::Dev { action } => {
            let dev_action = match action {
                DevAction::Build => masday_cli::commands::dev::DevAction::Build,
                DevAction::Install => masday_cli::commands::dev::DevAction::Install,
                DevAction::Serve { port } => masday_cli::commands::dev::DevAction::Serve { port },
            };
            masday_cli::commands::dev::run(dev_action, &project_dir).await?;
        }
    }

    Ok(())
}
