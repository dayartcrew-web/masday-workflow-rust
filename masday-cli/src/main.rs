//! CLI entry point for masday

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "masday")]
#[command(about = "Masday workflow orchestration — all-in-one binary", long_about = None)]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Interactive setup wizard (first-time configuration)
    Setup,

    /// Start API server + dashboard
    Serve {
        /// Port to listen on (overrides config)
        #[arg(long)]
        port: Option<u16>,
    },

    /// Start MCP server (stdio — used by AI platforms)
    Mcp,

    /// Check health of all services
    Status,

    /// Manage PostgreSQL via Docker
    Db {
        #[command(subcommand)]
        action: DbAction,
    },

    /// Install masday into the current project
    Install {
        /// Remote API server URL (skips local build, connects to remote)
        #[arg(long)]
        remote: Option<String>,

        /// API key for remote server
        #[arg(long)]
        api_key: Option<String>,

        /// Install only for specific platform (claude-code, gemini, vscode, opencode)
        #[arg(long)]
        platform: Option<String>,

        /// Standalone mode — extract templates only, no build or API server
        #[arg(long)]
        standalone: bool,

        /// Force local mode — cargo build from source (requires Rust toolchain)
        #[arg(long)]
        local_mode: bool,

        /// Skip building Rust crates (use existing binaries)
        #[arg(long)]
        skip_build: bool,

        /// Skip global directory installation
        #[arg(long)]
        local_only: bool,

        /// Force overwrite existing configs
        #[arg(long)]
        force: bool,
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

    /// Update masday (re-install with force)
    Update,
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
    let cli = Cli::parse();
    let project_dir = std::env::current_dir()?;

    match cli.command {
        Commands::Setup => {
            masday_cli::commands::setup::run(&project_dir)?;
        }
        Commands::Serve { port } => {
            masday_cli::commands::serve::run(port).await?;
        }
        Commands::Mcp => {
            masday_cli::commands::mcp_cmd::run().await?;
        }
        Commands::Status => {
            masday_cli::commands::status::run().await?;
        }
        Commands::Db { action } => match action {
            DbAction::Start => masday_cli::commands::db::start()?,
            DbAction::Stop => masday_cli::commands::db::stop()?,
            DbAction::Reset => masday_cli::commands::db::reset()?,
        },
        Commands::Install {
            remote,
            api_key,
            platform,
            standalone,
            local_mode,
            skip_build,
            local_only,
            force,
        } => {
            let args = masday_cli::commands::install::InstallArgs {
                remote,
                api_key,
                platform,
                standalone,
                local_mode,
                skip_build,
                local_only,
                force,
            };
            masday_cli::commands::install::run(args, &project_dir)?;
        }
        Commands::Uninstall { global, platform } => {
            let args = masday_cli::commands::uninstall::UninstallArgs { global, platform };
            masday_cli::commands::uninstall::run(args, &project_dir)?;
        }
        Commands::Update => {
            masday_cli::commands::update::run(&project_dir)?;
        }
    }

    Ok(())
}
