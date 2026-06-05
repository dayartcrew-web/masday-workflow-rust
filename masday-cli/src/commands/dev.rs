//! Development-only commands for building and managing Masday from source.
//!
//! This module is only compiled when the `dev-mode` feature is active.
//! Production builds will show a "not available" message instead.

use anyhow::Result;
use console::style;
use std::path::Path;

use crate::installer;

/// Arguments for the dev command group
#[derive(Debug, Clone)]
pub enum DevAction {
    /// Build all crates from source
    Build,
    /// Run local install (build + sync + MCP config)
    Install,
    /// Start API server from built binary
    Serve {
        /// Port to listen on (overrides config)
        port: Option<u16>,
    },
}

/// Run a dev subcommand
pub async fn run(action: DevAction, project_dir: &Path) -> Result<()> {
    match action {
        DevAction::Build => run_dev_build(project_dir),
        DevAction::Install => run_dev_install(project_dir),
        DevAction::Serve { port } => run_dev_serve(port).await,
    }
}

/// Build all crates from source using cargo
fn run_dev_build(project_dir: &Path) -> Result<()> {
    println!();
    println!("{}", style("Building Masday from source...").cyan().bold());
    println!();

    installer::build_crates(project_dir)?;

    println!();
    println!(
        "{}",
        style("Build complete! Binary at target/release/masday").green()
    );
    println!();
    println!("Next steps:");
    println!(
        "  {}  Run local install (sync + MCP config)",
        style("masday dev install").cyan()
    );
    println!(
        "  {}  Start the API server",
        style("masday dev serve").cyan()
    );

    Ok(())
}

/// Run local install: build + find binary + sync templates + MCP config
fn run_dev_install(project_dir: &Path) -> Result<()> {
    println!();
    println!(
        "{}",
        style("Installing Masday (development mode)...")
            .cyan()
            .bold()
    );
    println!();

    // Step 1: Build
    println!("{}", style("Building...").cyan());
    installer::build_crates(project_dir)?;
    println!("  {}", style("✓ Build complete").green());

    // Step 2: Find binary
    let mcp_binary = installer::find_mcp_binary(project_dir)?;
    println!(
        "  {} Found binary: {}",
        style("✓").green(),
        mcp_binary.display()
    );

    // Step 3: Run install with Local mode
    let install_args = crate::commands::install::InstallArgs {
        mode: Some(crate::commands::install::InstallMode::Local),
        skip_build: true, // Already built above
        force: true,
        ..Default::default()
    };
    crate::commands::install::run(install_args, project_dir)?;

    Ok(())
}

/// Start the API server from the built binary
async fn run_dev_serve(port: Option<u16>) -> Result<()> {
    println!();
    println!(
        "{}",
        style("Starting API server (development mode)...")
            .cyan()
            .bold()
    );
    println!();

    // Delegate to the serve command
    // The serve command handles config loading, DB pool, etc.
    crate::commands::serve::run(port).await?;

    Ok(())
}
