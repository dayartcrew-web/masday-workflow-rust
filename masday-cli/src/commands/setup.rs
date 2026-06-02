//! Interactive setup wizard for first-time Masday configuration.
//!
//! Guides users through:
//! - Mode selection (local vs remote)
//! - PostgreSQL setup (Docker or existing)
//! - Embedding model selection
//! - Platform selection (Claude Code, Gemini, etc.)
//! - Config persistence and agent/skill/hook sync

use anyhow::{bail, Result};
use console::style;
use indicatif::{ProgressBar, ProgressStyle};
use std::path::Path;

use crate::config::MasdayConfig;
use crate::docker;

/// Run the interactive setup wizard
pub fn run(project_dir: &Path) -> Result<()> {
    println!();
    println!(
        "{}",
        style("╔══════════════════════════════════════╗").cyan()
    );
    println!(
        "{}",
        style("║     ⚡ Masday Setup Wizard           ║").cyan()
    );
    println!(
        "{}",
        style("╚══════════════════════════════════════╝").cyan()
    );
    println!();

    // Check if config already exists
    if MasdayConfig::exists() {
        let overwrite = inquire::Confirm::new("Configuration already exists. Overwrite?")
            .with_default(false)
            .prompt()?;
        if !overwrite {
            println!("Setup cancelled.");
            return Ok(());
        }
    }

    // Step 1: Mode selection
    let mode = inquire::Select::new(
        "How would you like to run Masday?",
        vec![
            "Local (everything on this machine)",
            "Remote (connect to existing server)",
        ],
    )
    .with_help_message("↑↓ to move, Enter to select")
    .prompt()?;

    if mode.starts_with("Local") {
        run_local_setup(project_dir)?;
    } else {
        run_remote_setup(project_dir)?;
    }

    Ok(())
}

fn run_local_setup(project_dir: &Path) -> Result<()> {
    println!();
    println!("{}", style("── Local Mode Setup ──").cyan().bold());

    // Step 2a: PostgreSQL
    let pg_choice = inquire::Select::new(
        "PostgreSQL setup:",
        vec![
            "Start Docker container (recommended)",
            "Use existing PostgreSQL",
        ],
    )
    .prompt()?;

    let database_url = if pg_choice.starts_with("Start Docker") {
        if !docker::is_docker_available() {
            bail!("Docker is not installed. Install Docker Desktop first, or choose 'Use existing PostgreSQL'.");
        }

        // Start PostgreSQL container
        let pb = spinner("Starting PostgreSQL container...");
        docker::start_postgres("masday", "masdaypass", "masday_workflow")?;
        pb.finish_with_message(format!("{} PostgreSQL started", style("✓").green()));

        // Wait for ready
        let pb = spinner("Waiting for PostgreSQL to be ready...");
        docker::wait_for_postgres("localhost", 54341, 30)?;
        pb.finish_with_message(format!("{} PostgreSQL ready on port 54341", style("✓").green()));

        // Start Redis too
        let pb = spinner("Starting Redis container...");
        docker::start_redis()?;
        pb.finish_with_message(format!("{} Redis ready on port 63791", style("✓").green()));

        // Run migrations
        let pb = spinner("Running database migrations...");
        let db_url = docker::default_database_url();
        std::env::set_var("DATABASE_URL", &db_url);

        // Create a runtime for async migrations
        let rt = tokio::runtime::Runtime::new()?;
        let pool = rt.block_on(masday_db::pool::init_pool_with_retry(5))
            .map_err(|e| anyhow::anyhow!("{}", e))?;
        rt.block_on(masday_db::run_migrations(&pool))
            .map_err(|e| anyhow::anyhow!("{}", e))?;
        pb.finish_with_message(format!("{} Migrations complete", style("✓").green()));

        Some(db_url)
    } else {
        // Prompt for existing connection
        let url = inquire::Text::new("Database URL:")
            .with_default("postgresql://postgres:postgres@localhost:5432/masday_workflow")
            .with_help_message("Full PostgreSQL connection URL")
            .prompt()?;
        Some(url)
    };

    // Step 2b: Embedding model
    println!();
    let embed_choice = inquire::Select::new(
        "Select embedding model:",
        vec![
            "all-MiniLM-L6-v2 (384 dims, fast, ~80MB)",
            "bge-small-en-v1.5 (384 dims, balanced)",
            "bge-base-en-v1.5 (768 dims, accurate)",
            "Skip embeddings",
        ],
    )
    .with_help_message("Local ONNX model — no external service needed")
    .prompt()?;

    let (embed_provider, embed_model, embed_dims) = match embed_choice {
        s if s.starts_with("all-MiniLM") => {
            (Some("local"), Some("all-MiniLM-L6-v2"), Some(384usize))
        }
        s if s.starts_with("bge-small") => {
            (Some("local"), Some("bge-small-en-v1.5"), Some(384usize))
        }
        s if s.starts_with("bge-base") => {
            (Some("local"), Some("bge-base-en-v1.5"), Some(768usize))
        }
        _ => (None, None, None),
    };

    // Step 3: Platform selection
    println!();
    let platforms = inquire::MultiSelect::new(
        "Select your AI platforms (space to toggle):",
        vec![
            "Claude Code",
            "Gemini CLI",
            "VS Code Copilot",
            "OpenCode",
        ],
    )
    .with_default(&[0])
    .with_help_message("Space to toggle, Enter to confirm")
    .prompt()?;

    if platforms.is_empty() {
        bail!("At least one platform must be selected.");
    }

    let platform_names: Vec<String> = platforms
        .iter()
        .map(|p| match *p {
            "Claude Code" => "claude-code",
            "Gemini CLI" => "gemini",
            "VS Code Copilot" => "vscode",
            "OpenCode" => "opencode",
            _ => "claude-code",
        })
        .map(String::from)
        .collect();

    // Step 4: Port
    let port_str = inquire::Text::new("API port:")
        .with_default("30101")
        .prompt()?;
    let port: u16 = port_str.parse().unwrap_or(30101);

    // Step 5: Save config
    let config = MasdayConfig {
        mode: "local".to_string(),
        api_url: format!("http://localhost:{}", port),
        api_key: "local-dev".to_string(),
        database_url,
        embedding_provider: embed_provider.map(String::from),
        embedding_model: embed_model.map(String::from),
        embedding_dimensions: embed_dims,
        port,
        platforms: platform_names.clone(),
    };
    config.save()?;

    // Step 6: Run install to sync agents/skills/hooks
    println!();
    let pb = spinner("Installing agents, skills, and hooks...");
    let install_args = crate::commands::install::InstallArgs {
        remote: None,
        api_key: None,
        platform: None,
        standalone: false,
        local_mode: false,
        skip_build: true,
        local_only: false,
        force: true,
    };
    crate::commands::install::run(install_args, project_dir)?;
    pb.finish_with_message(format!("{} Templates synced", style("✓").green()));

    print_success(&config);
    Ok(())
}

fn run_remote_setup(project_dir: &Path) -> Result<()> {
    println!();
    println!("{}", style("── Remote Mode Setup ──").cyan().bold());

    let api_url = inquire::Text::new("Server URL:")
        .with_default("https://masday.example.com")
        .with_help_message("HTTPS recommended")
        .prompt()?;

    // Validate URL
    if !api_url.starts_with("http://") && !api_url.starts_with("https://") {
        bail!("URL must start with http:// or https://");
    }

    let api_key = inquire::Password::new("API Key:")
        .with_help_message("Your Masday API key")
        .prompt()?;

    // Verify connectivity
    let pb = spinner("Verifying connection...");
    let health_url = format!("{}/api/health", api_url.trim_end_matches('/'));
    match reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .unwrap()
        .get(&health_url)
        .header("Authorization", format!("Bearer {}", api_key))
        .send()
    {
        Ok(resp) if resp.status().is_success() => {
            pb.finish_with_message(format!("{} Server reachable", style("✓").green()));
        }
        Ok(resp) => {
            pb.finish_with_message(format!("{} Server returned {}", style("⚠").yellow(), resp.status()));
            println!(
                "  {} Server responded but not healthy. Continue anyway?",
                style("⚠").yellow()
            );
            let cont = inquire::Confirm::new("Continue?")
                .with_default(true)
                .prompt()?;
            if !cont {
                bail!("Setup cancelled.");
            }
        }
        Err(e) => {
            pb.finish_with_message(format!("{} Connection failed", style("✗").red()));
            println!("  Error: {}", e);
            let cont = inquire::Confirm::new("Continue anyway?")
                .with_default(false)
                .prompt()?;
            if !cont {
                bail!("Setup cancelled.");
            }
        }
    }

    // Platform selection
    println!();
    let platforms = inquire::MultiSelect::new(
        "Select your AI platforms (space to toggle):",
        vec![
            "Claude Code",
            "Gemini CLI",
            "VS Code Copilot",
            "OpenCode",
        ],
    )
    .with_default(&[0])
    .prompt()?;

    let platform_names: Vec<String> = platforms
        .iter()
        .map(|p| match *p {
            "Claude Code" => "claude-code",
            "Gemini CLI" => "gemini",
            "VS Code Copilot" => "vscode",
            "OpenCode" => "opencode",
            _ => "claude-code",
        })
        .map(String::from)
        .collect();

    // Save config
    let config = MasdayConfig {
        mode: "remote".to_string(),
        api_url: api_url.trim_end_matches('/').to_string(),
        api_key,
        database_url: None,
        embedding_provider: None,
        embedding_model: None,
        embedding_dimensions: None,
        port: 30101,
        platforms: platform_names.clone(),
    };
    config.save()?;

    // Run install in remote mode
    println!();
    let pb = spinner("Installing agents, skills, and hooks...");
    let install_args = crate::commands::install::InstallArgs {
        remote: Some(config.api_url.clone()),
        api_key: Some(config.api_key.clone()),
        platform: None,
        standalone: false,
        local_mode: false,
        skip_build: true,
        local_only: false,
        force: true,
    };
    crate::commands::install::run(install_args, project_dir)?;
    pb.finish_with_message(format!("{} Templates synced", style("✓").green()));

    print_success(&config);
    Ok(())
}

fn print_success(config: &MasdayConfig) {
    println!();
    println!("{}", style("╔══════════════════════════════════════╗").green());
    println!("{}", style("║     ✓ Masday is ready!              ║").green());
    println!("{}", style("╚══════════════════════════════════════╝").green());
    println!();

    if config.mode == "local" {
        println!("  Dashboard: http://localhost:{}", config.port);
        println!("  API:       http://localhost:{}/api", config.port);
    } else {
        println!("  Server:    {}", config.api_url);
    }
    println!(
        "  MCP:       stdio (configured for {})",
        config.platforms.join(", ")
    );
    println!();
    if config.mode == "local" {
        println!(
            "  Run '{}' to start the dashboard",
            style("masday serve").cyan()
        );
    }
    println!(
        "  Run '{}' to check health",
        style("masday status").cyan()
    );
    println!(
        "  Config saved to: {}",
        MasdayConfig::config_path().display()
    );
    println!();
}

fn spinner(message: &str) -> ProgressBar {
    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.cyan} {msg}")
            .unwrap(),
    );
    pb.set_message(message.to_string());
    pb.enable_steady_tick(std::time::Duration::from_millis(80));
    pb
}
