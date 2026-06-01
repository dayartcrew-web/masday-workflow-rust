//! CLI entry point for masday

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "masday")]
#[command(about = "Masday workflow orchestration CLI", long_about = None)]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
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

    /// Run database migrations
    DbMigrate,

    /// Start the API server
    Serve,

    /// Show workflow status
    Status {
        /// Workflow ID
        id: Option<String>,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let project_dir = std::env::current_dir()?;

    match cli.command {
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
        Commands::DbMigrate => {
            println!("Running database migrations...");
            // TODO: implement via masday-db crate
        }
        Commands::Serve => {
            println!("Starting API server...");
            // TODO: delegate to masday-api
        }
        Commands::Status { id } => {
            match id {
                Some(id) => println!("Workflow status for: {}", id),
                None => println!("Listing all workflows..."),
            }
            // TODO: implement via masday-api client
        }
    }

    Ok(())
}
