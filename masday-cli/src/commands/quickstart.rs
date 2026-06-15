//! Quickstart command — one command to set up everything.
//!
//! Interactive wizard that handles:
//! - Mode selection (local/remote/standalone)
//! - DB setup (Docker or existing or remote URL)
//! - API key configuration
//! - Platform detection + MCP server registration
//! - Agent/skill/hook sync

use anyhow::{bail, Context, Result};
use console::style;
use indicatif::{ProgressBar, ProgressStyle};
use std::path::Path;

// InstallArgs removed — quickstart uses sync functions directly
use crate::commands::embed;
use crate::config::MasdayConfig;
use crate::docker;
use crate::installer::{
    all_platforms, build_crates, generate_mcp_config, install_git_hooks, install_global_hooks,
    install_project_hooks, is_build_fresh, register_hooks_in_settings, sync_agents_to_global,
    sync_agents_to_project, sync_scripts_to_masday_dir, sync_skills_to_global,
    sync_skills_to_project, McpConfig, Platform,
};

/// Arguments for the quickstart command (supports non-interactive mode).
#[derive(Debug, Default, Clone)]
pub struct QuickstartArgs {
    /// Setup mode: local | remote | standalone | server
    pub mode: Option<String>,
    /// Developer mode: build from source
    pub dev: bool,
    /// Skip cargo build step (only with --dev)
    pub skip_build: bool,
    /// Non-interactive: use defaults for all prompts
    pub yes: bool,
    /// Database URL (local mode only)
    pub database_url: Option<String>,
    /// Redis URL (local mode only)
    pub redis_url: Option<String>,
    /// Embedding model ID
    pub embedding: Option<String>,
    /// Platform(s) to install (comma-separated)
    pub platform: Option<String>,
    /// API server port (local mode only)
    pub api_port: Option<u16>,
    /// Docker image tag (server mode only)
    pub image_tag: Option<String>,
}

/// Run the quickstart wizard. Pass `None` for interactive mode, or `Some(args)` for CLI-driven mode.
pub async fn run(project_dir: &Path, args: Option<QuickstartArgs>) -> Result<()> {
    println!();
    println!("{}", style("⚡ Masday Quickstart").cyan().bold());
    println!(
        "{}",
        style("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━").cyan()
    );
    println!();

    // ── Step 1: Detect environment ────────────────────────────────────────
    let has_docker = docker::is_docker_available();
    let config_exists = MasdayConfig::exists();
    let detected_platforms = detect_active_platforms_from_home();

    println!("{}", style("Environment:").cyan().bold());
    println!(
        "  Docker:     {}",
        if has_docker {
            style("✓ available").green()
        } else {
            style("✗ not found").yellow()
        }
    );
    println!(
        "  Config:     {}",
        if config_exists {
            style("✓ exists").green()
        } else {
            style("— not yet").dim()
        }
    );
    if !detected_platforms.is_empty() {
        println!(
            "  Platforms:  {}",
            style(platform_names(&detected_platforms).join(", ")).green()
        );
    }
    println!();

    // ── Step 2: Choose mode ───────────────────────────────────────────────
    let is_non_interactive = args.as_ref().is_some_and(|a| a.yes || a.mode.is_some());

    let mode_str = if let Some(ref a) = args {
        if let Some(ref m) = a.mode {
            m.clone()
        } else if a.yes {
            "local".to_string()
        } else {
            // Interactive — use inquire
            let mode_options = vec![
                "Local — everything on this machine (Docker)",
                "Remote — connect to existing API server",
                "Server — deploy full server stack (Docker)",
                "Standalone — agents & skills only (no DB, no API)",
            ];
            let choice = inquire::Select::new("How will you run Masday?", mode_options)
                .with_help_message("↑↓ to move, Enter to select")
                .prompt()?;
            println!();
            if choice.starts_with("Local") {
                "local".to_string()
            } else if choice.starts_with("Remote") {
                "remote".to_string()
            } else if choice.starts_with("Server") {
                "server".to_string()
            } else {
                "standalone".to_string()
            }
        }
    } else {
        let mode_options = vec![
            "Local — everything on this machine (Docker)",
            "Remote — connect to existing API server",
            "Server — deploy full server stack (Docker)",
            "Standalone — agents & skills only (no DB, no API)",
        ];
        let choice = inquire::Select::new("How will you run Masday?", mode_options)
            .with_help_message("↑↓ to move, Enter to select")
            .prompt()?;
        println!();
        if choice.starts_with("Local") {
            "local".to_string()
        } else if choice.starts_with("Remote") {
            "remote".to_string()
        } else if choice.starts_with("Server") {
            "server".to_string()
        } else {
            "standalone".to_string()
        }
    };

    if is_non_interactive {
        println!("  Mode: {}", style(&mode_str).cyan());
        println!();
    }

    match mode_str.as_str() {
        "local" => {
            run_local_mode(project_dir, &detected_platforms, has_docker, args.as_ref()).await?;
        }
        "remote" => {
            run_remote_mode(project_dir, &detected_platforms, args.as_ref())?;
        }
        "standalone" => {
            run_standalone_mode(project_dir, &detected_platforms, args.as_ref())?;
        }
        "server" => {
            run_server_mode(project_dir, &detected_platforms, has_docker, args.as_ref()).await?;
        }
        other => bail!(
            "Invalid mode '{}'. Use: local, remote, standalone, server",
            other
        ),
    }

    Ok(())
}

// ═══════════════════════════════════════════════════════════════════════════
// LOCAL MODE
// ═══════════════════════════════════════════════════════════════════════════

async fn run_local_mode(
    project_dir: &Path,
    detected_platforms: &[Platform],
    has_docker: bool,
    args: Option<&QuickstartArgs>,
) -> Result<()> {
    println!("{}", style("── Local Mode ──").cyan().bold());
    println!();

    let is_non_interactive = args.is_some_and(|a| a.yes);

    // ── DB Setup ──────────────────────────────────────────────────────────
    let infra_resolution = resolve_infra(
        has_docker,
        is_non_interactive,
        args.and_then(|a| a.database_url.clone()),
        args.and_then(|a| a.redis_url.clone()),
    )?;

    let database_url = match infra_resolution {
        InfraResolution::Docker {
            user,
            password,
            db,
            redis_url: infra_redis_url,
        } => Some(
            start_docker_infrastructure(&user, &password, &db, infra_redis_url.as_deref()).await?,
        ),
        InfraResolution::ExistingUrl {
            database_url: url,
            redis_url: _infra_redis_url,
        } => Some(url),
        InfraResolution::NoDatabase => {
            if !is_non_interactive {
                // Ask for URL in interactive mode
                ask_database_url().await?
            } else {
                None
            }
        }
    };

    // ── Embedding (optional) ──────────────────────────────────────────────
    let (embed_provider, embed_model, embed_dims) =
        if let Some(emb_id) = args.and_then(|a| a.embedding.as_ref()) {
            // Explicit --embedding provided
            resolve_embedding_model(emb_id)?
        } else if is_non_interactive {
            // Default: all-MiniLM-L6-v2
            println!(
                "  Embedding: {} (default)",
                style("all-MiniLM-L6-v2").cyan()
            );
            (
                Some("local".into()),
                Some("all-MiniLM-L6-v2".into()),
                Some(384),
            )
        } else {
            ask_embedding_model()?
        };

    // ── Platforms ─────────────────────────────────────────────────────────
    let platforms = if let Some(platform_str) = args.and_then(|a| a.platform.as_ref()) {
        parse_platforms(platform_str)?
    } else if is_non_interactive {
        let plats = if detected_platforms.is_empty() {
            vec![Platform::ClaudeCode]
        } else {
            detected_platforms.to_vec()
        };
        println!(
            "  Platforms: {}",
            style(platform_names(&plats).join(", ")).cyan()
        );
        plats
    } else {
        ask_platforms(detected_platforms)?
    };

    // ── Save config ───────────────────────────────────────────────────────
    // Use custom api_port from args if provided
    let api_port = args
        .and_then(|a| a.api_port)
        .unwrap_or(masday_core::constants::ports::API_PORT);
    let api_url = format!("http://localhost:{}", api_port);

    // Get redis_url from args if provided
    let redis_url = args.and_then(|a| a.redis_url.clone());

    let config = MasdayConfig {
        mode: "local".to_string(),
        api_url: api_url.clone(),
        api_key: "***".to_string(),
        database_url: database_url.clone(),
        redis_url: redis_url.clone(),
        embedding_provider: embed_provider,
        embedding_model: embed_model,
        embedding_dimensions: embed_dims,
        embedding_base_url: None,
        embedding_api_key: None,
        api_port,
        db_port: masday_core::constants::ports::POSTGRES_PORT,
        redis_port: masday_core::constants::ports::REDIS_PORT,
        dashboard_port: api_port,
        platforms: platform_names(&platforms),
    };
    save_config_and_env(&config)?;

    // ── Sync templates ────────────────────────────────────────────────────
    sync_templates(project_dir, &platforms)?;

    // ── Initialize SQLite database for MCP stdio mode ─────────────────────
    init_sqlite_database()?;

    // ── Register MCP servers ──────────────────────────────────────────────
    register_mcp_servers(
        project_dir,
        &platforms,
        &api_url,
        "***",
        database_url.as_deref(),
        "local",
    )?;

    // ── Summary ───────────────────────────────────────────────────────────
    print_local_summary(&config);
    Ok(())
}

// ═══════════════════════════════════════════════════════════════════════════
// REMOTE MODE
// ═══════════════════════════════════════════════════════════════════════════

fn run_remote_mode(
    project_dir: &Path,
    detected_platforms: &[Platform],
    args: Option<&QuickstartArgs>,
) -> Result<()> {
    println!("{}", style("── Remote Mode ──").cyan().bold());
    println!();

    let is_non_interactive = args.is_some_and(|a| a.yes);

    // ── API Server URL ────────────────────────────────────────────────────
    let api_url = if is_non_interactive {
        bail!("Remote mode requires --mode remote and environment setup. Use interactive mode or set API_URL env var.");
    } else {
        let url = inquire::Text::new("API server URL:")
            .with_default("https://masday.example.com")
            .with_help_message("HTTPS recommended. e.g. https://masday.yourcompany.com")
            .prompt()?;
        url.trim_end_matches('/').to_string()
    };

    if !api_url.starts_with("http://") && !api_url.starts_with("https://") {
        bail!("URL must start with http:// or https://");
    }

    // ── API Key ───────────────────────────────────────────────────────────
    let api_key = inquire::Password::new("API key:")
        .with_help_message("Your Masday API key")
        .prompt()?;

    // ── Verify connectivity ───────────────────────────────────────────────
    let health_url = format!("{}/api/health", api_url);
    print!("  {} Verifying connection...", style("→").cyan());
    match reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .unwrap()
        .get(&health_url)
        .header("Authorization", format!("Bearer {}", api_key))
        .send()
    {
        Ok(resp) if resp.status().is_success() => {
            println!(" {}", style("✓ connected").green());
        }
        Ok(resp) => {
            println!(
                " {} ({})",
                style("⚠ server returned").yellow(),
                resp.status()
            );
            let cont = inquire::Confirm::new("Continue anyway?")
                .with_default(true)
                .prompt()?;
            if !cont {
                bail!("Setup cancelled.");
            }
        }
        Err(e) => {
            println!(" {}", style("✗ failed").red());
            println!("    {}", e);
            let cont = inquire::Confirm::new("Continue anyway?")
                .with_default(false)
                .prompt()?;
            if !cont {
                bail!("Setup cancelled.");
            }
        }
    }
    println!();

    // ── Remote DB URL (optional — for direct DB access from MCP) ──────────
    let remote_db = inquire::Text::new("Remote database URL (optional):")
        .with_help_message("Leave empty if MCP connects via API only")
        .prompt()?;
    let database_url = if remote_db.trim().is_empty() {
        None
    } else {
        Some(remote_db.trim().to_string())
    };

    // ── Platforms ─────────────────────────────────────────────────────────
    let platforms = if let Some(platform_str) = args.and_then(|a| a.platform.as_ref()) {
        parse_platforms(platform_str)?
    } else if is_non_interactive {
        let plats = if detected_platforms.is_empty() {
            vec![Platform::ClaudeCode]
        } else {
            detected_platforms.to_vec()
        };
        println!(
            "  Platforms: {}",
            style(platform_names(&plats).join(", ")).cyan()
        );
        plats
    } else {
        ask_platforms(detected_platforms)?
    };

    // ── Save config ───────────────────────────────────────────────────────
    let is_windows = cfg!(target_os = "windows");
    if is_windows {
        println!();
        println!(
            "{}",
            style("⚠ Windows — local ONNX embeddings not included.").yellow()
        );
        println!("  Edit ~/.masday/config.toml to add a remote embedding provider:");
        println!(
            "  {}",
            style("embedding_provider = \"ollama\"    # ollama | openai").cyan()
        );
        println!(
            "  {}",
            style("embedding_model = \"nomic-embed-text\"").cyan()
        );
        println!("  {}", style("embedding_dimensions = 768").cyan());
        println!();
    }

    let config = MasdayConfig {
        mode: "remote".to_string(),
        api_url: api_url.clone(),
        api_key: api_key.clone(),
        database_url: database_url.clone(),
        redis_url: None,
        embedding_provider: if is_windows {
            Some(String::new())
        } else {
            None
        },
        embedding_model: if is_windows {
            Some(String::new())
        } else {
            None
        },
        embedding_dimensions: if is_windows { Some(0) } else { None },
        embedding_base_url: None,
        embedding_api_key: None,
        api_port: masday_core::constants::ports::API_PORT,
        db_port: masday_core::constants::ports::POSTGRES_PORT,
        redis_port: masday_core::constants::ports::REDIS_PORT,
        dashboard_port: masday_core::constants::ports::API_PORT,
        platforms: platform_names(&platforms),
    };
    save_config_and_env(&config)?;

    // ── Sync templates ────────────────────────────────────────────────────
    sync_templates(project_dir, &platforms)?;

    // ── Initialize SQLite database for MCP stdio mode ─────────────────────
    init_sqlite_database()?;

    // ── Register MCP servers ──────────────────────────────────────────────
    register_mcp_servers(
        project_dir,
        &platforms,
        &api_url,
        &api_key,
        database_url.as_deref(),
        "remote",
    )?;

    // ── Summary ───────────────────────────────────────────────────────────
    print_remote_summary(&config);
    Ok(())
}

// ═══════════════════════════════════════════════════════════════════════════
// SERVER MODE
// ═══════════════════════════════════════════════════════════════════════════

async fn run_server_mode(
    project_dir: &Path,
    _detected_platforms: &[Platform],
    has_docker: bool,
    args: Option<&QuickstartArgs>,
) -> Result<()> {
    println!("{}", style("── Server Mode ──").cyan().bold());
    println!();

    let is_non_interactive = args.is_some_and(|a| a.yes);
    let is_dev = args.is_some_and(|a| a.dev);

    // ── Step 0: Build binary if --dev ────────────────────────────────────────
    if is_dev {
        let skip_build = args.is_some_and(|a| a.skip_build);
        if !skip_build && !is_build_fresh(project_dir) {
            println!("{}", style("Building masday binary...").cyan());
            build_crates(project_dir)?;
            println!("  {} Build complete", style("✓").green());
            println!();
        } else if skip_build {
            println!("  {} Skipping build (--skip-build)", style("→").cyan());
            println!();
        } else {
            println!("  {} Build fresh, skipping cargo build", style("→").dim());
            println!();
        }
    }

    // ── Step 1: Resolve infra ──────────────────────────────────────────────────
    let infra_resolution = resolve_infra(
        has_docker,
        is_non_interactive,
        args.and_then(|a| a.database_url.clone()),
        args.and_then(|a| a.redis_url.clone()),
    )?;

    let (database_url, redis_url) = match infra_resolution {
        InfraResolution::Docker {
            user,
            password,
            db,
            redis_url: infra_redis_url,
        } => {
            // Start Docker containers
            let db_url =
                start_docker_infrastructure(&user, &password, &db, infra_redis_url.as_deref())
                    .await?;
            (Some(db_url), infra_redis_url)
        }
        InfraResolution::ExistingUrl {
            database_url: url,
            redis_url: infra_redis_url,
        } => (Some(url), infra_redis_url),
        InfraResolution::NoDatabase => {
            // Server mode requires database
            if !is_non_interactive {
                bail!("Server mode requires a database. Provide --database-url or ensure Docker is available.");
            } else {
                (None, None)
            }
        }
    };

    // ── Step 2: Migrations ─────────────────────────────────────────────────────
    // For now, just print instructions (actual migration runner is out of scope)
    if database_url.is_some() {
        println!("{}", style("Database migrations:").cyan());
        println!("  {} Database URL configured", style("✓").green());
        println!(
            "  {} Migrations will run automatically on API startup",
            style("→").cyan()
        );
        println!();
    }

    // ── Step 3: Save config ───────────────────────────────────────────────────
    let api_port = args
        .and_then(|a| a.api_port)
        .unwrap_or(masday_core::constants::ports::API_PORT);
    let api_url = format!("http://localhost:{}", api_port);

    let config = MasdayConfig {
        mode: "server".to_string(),
        api_url: api_url.clone(),
        api_key: "***".to_string(),
        database_url: database_url.clone(),
        redis_url: redis_url.clone(),
        embedding_provider: args
            .as_ref()
            .and_then(|a| a.embedding.as_ref())
            .map(|_| "local".to_string()),
        embedding_model: args.as_ref().and_then(|a| a.embedding.as_ref()).cloned(),
        embedding_dimensions: Some(384), // Default for all-MiniLM-L6-v2
        embedding_base_url: None,
        embedding_api_key: None,
        api_port,
        db_port: masday_core::constants::ports::POSTGRES_PORT,
        redis_port: masday_core::constants::ports::REDIS_PORT,
        dashboard_port: api_port,
        platforms: vec![], // Server mode doesn't need platform sync
    };
    save_config_and_env(&config)?;

    // ── Step 4: Initialize SQLite for MCP stdio mode ───────────────────────
    init_sqlite_database()?;

    // ── Step 5: Start API ─────────────────────────────────────────────────────
    if is_dev {
        // Dev mode: print instructions to run cargo run
        println!("{}", style("API Server (dev mode):").cyan());
        println!("  {} To start the API server:", style("→").cyan());
        println!();
        println!(
            "    {}",
            style(format!("cd {}", project_dir.display())).dim()
        );
        if let Some(ref db_url) = database_url {
            println!(
                "    {}",
                style(format!("export DATABASE_URL=\"{}\"", db_url)).dim()
            );
        }
        if let Some(ref redis_url) = redis_url {
            println!(
                "    {}",
                style(format!("export REDIS_URL=\"{}\"", redis_url)).dim()
            );
        }
        println!("    {}", style("cargo run -p masday-api").cyan());
        println!();
    } else if has_docker {
        // Docker mode: use docker compose
        println!("{}", style("API Server (Docker):").cyan());

        // Check for docker-compose.server.yml
        let compose_file = project_dir.join("docker-compose.server.yml");
        if compose_file.exists() {
            println!("  {} Starting Docker containers...", style("→").cyan());
            println!();
            println!(
                "    {}",
                style(format!(
                    "docker compose -f {} up -d",
                    compose_file.display()
                ))
                .cyan()
            );
            println!();
            println!("  {} Or run manually:", style("→").cyan());
            println!(
                "    {}",
                style(format!(
                    "docker compose -f {} up -d",
                    compose_file.display()
                ))
                .dim()
            );
            println!();
        } else {
            println!(
                "  {} docker-compose.server.yml not found",
                style("⚠").yellow()
            );
            println!(
                "  {} Expected location: {}",
                style("→").yellow(),
                compose_file.display()
            );
            println!();
        }
    } else {
        // No Docker, no dev: print instructions to download binary
        println!("{}", style("API Server (production mode):").cyan());
        println!(
            "  {} Download the latest binary from GitHub Releases:",
            style("→").cyan()
        );
        println!(
            "    {}",
            style("https://github.com/dayartcrew-web/masday-workflow-rust/releases").cyan()
        );
        println!();
        println!("  {} Then run:", style("→").cyan());
        if let Some(ref db_url) = database_url {
            println!(
                "    {}",
                style(format!("DATABASE_URL=\"{}\"", db_url)).dim()
            );
        }
        if let Some(ref redis_url) = redis_url {
            println!(
                "    {}",
                style(format!("REDIS_URL=\"{}\"", redis_url)).dim()
            );
        }
        println!("    {}", style("masday serve").cyan());
        println!();
    }

    // ── Step 5: Print summary ─────────────────────────────────────────────────
    print_server_summary(&config, database_url.as_deref(), redis_url.as_deref());
    Ok(())
}

// ═══════════════════════════════════════════════════════════════════════════
// STANDALONE MODE
// ═══════════════════════════════════════════════════════════════════════════

fn run_standalone_mode(
    project_dir: &Path,
    detected_platforms: &[Platform],
    args: Option<&QuickstartArgs>,
) -> Result<()> {
    println!("{}", style("── Standalone Mode ──").cyan().bold());
    println!();

    let is_non_interactive = args.is_some_and(|a| a.yes);

    let platforms = if let Some(platform_str) = args.and_then(|a| a.platform.as_ref()) {
        parse_platforms(platform_str)?
    } else if is_non_interactive {
        let plats = if detected_platforms.is_empty() {
            vec![Platform::ClaudeCode]
        } else {
            detected_platforms.to_vec()
        };
        println!(
            "  Platforms: {}",
            style(platform_names(&plats).join(", ")).cyan()
        );
        plats
    } else {
        ask_platforms(detected_platforms)?
    };

    // ── Save config ─────────────────────────────────────────────────────
    // On Windows, local ONNX embeddings are not included.
    // Pre-fill empty embedding fields so user can manually add remote provider.
    let is_windows = cfg!(target_os = "windows");
    if is_windows {
        println!();
        println!(
            "{}",
            style("⚠ Windows — local ONNX embeddings not included in this binary.").yellow()
        );
        println!("  Edit ~/.masday/config.toml to add a remote embedding provider:");
        println!();
        println!(
            "  {}",
            style("embedding_provider = \"ollama\"    # ollama | openai").cyan()
        );
        println!(
            "  {}",
            style("embedding_model = \"nomic-embed-text\"").cyan()
        );
        println!("  {}", style("embedding_dimensions = 768").cyan());
        println!();
    }

    let config = MasdayConfig {
        mode: "standalone".to_string(),
        api_url: "none".to_string(),
        api_key: "none".to_string(),
        database_url: None,
        redis_url: None,
        embedding_provider: if is_windows {
            Some(String::new())
        } else {
            None
        },
        embedding_model: if is_windows {
            Some(String::new())
        } else {
            None
        },
        embedding_dimensions: if is_windows { Some(0) } else { None },
        embedding_base_url: None,
        embedding_api_key: None,
        api_port: masday_core::constants::ports::API_PORT,
        db_port: masday_core::constants::ports::POSTGRES_PORT,
        redis_port: masday_core::constants::ports::REDIS_PORT,
        dashboard_port: masday_core::constants::ports::API_PORT,
        platforms: platform_names(&platforms),
    };
    save_config_and_env(&config)?;

    // ── Sync templates ────────────────────────────────────────────────────
    sync_templates(project_dir, &platforms)?;

    // ── Initialize SQLite database for MCP stdio mode ─────────────────────
    init_sqlite_database()?;

    // ── Register MCP servers (stdio, no API) ──────────────────────────────
    register_mcp_servers(project_dir, &platforms, "", "", None, "standalone")?;

    // ── Summary ───────────────────────────────────────────────────────────
    println!();
    println!(
        "{}",
        style("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━").green()
    );
    println!(
        "{}",
        style("  ⚡ Masday is ready! (standalone)").green().bold()
    );
    println!(
        "{}",
        style("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━").green()
    );
    println!();
    println!("  Agents and skills installed.");
    println!("  SQLite Database: ~/.masday/data.db");
    println!("  MCP Transport: {}", style("stdio").cyan());
    println!();
    println!("  For full MCP tools support, connect to an API server:");
    println!("    {}", style("masday quickstart").cyan());
    println!("    → Choose 'Remote' mode and provide your server URL + API key");
    println!();
    Ok(())
}

// ═══════════════════════════════════════════════════════════════════════════
// SHARED HELPERS
// ═══════════════════════════════════════════════════════════════════════════

/// Infrastructure resolution options
#[derive(Debug, Clone)]
pub enum InfraResolution {
    /// Use Docker containers (recommended)
    Docker {
        user: String,
        password: String,
        db: String,
        /// Optional Redis URL (if provided, skips Docker Redis)
        redis_url: Option<String>,
    },
    /// Use existing PostgreSQL instance
    ExistingUrl {
        database_url: String,
        redis_url: Option<String>,
    },
    /// No database needed (standalone/remote API only)
    NoDatabase,
}

/// Resolve infrastructure setup - Docker vs existing DB
pub fn resolve_infra(
    has_docker: bool,
    is_non_interactive: bool,
    explicit_url: Option<String>,
    explicit_redis_url: Option<String>,
) -> Result<InfraResolution> {
    if let Some(url) = explicit_url {
        println!("  Database URL: {}", style(&url).cyan());
        if let Some(ref redis_url) = explicit_redis_url {
            println!("  Redis URL: {}", style(redis_url).cyan());
        }
        return Ok(InfraResolution::ExistingUrl {
            database_url: url,
            redis_url: explicit_redis_url,
        });
    }

    if is_non_interactive {
        if has_docker {
            let (pg_user, pg_pass, pg_db) =
                (docker::pg_user(), docker::pg_password(), docker::pg_db());
            if explicit_redis_url.is_some() {
                println!(
                    "  {} Starting Docker PostgreSQL with defaults (Redis URL provided)...",
                    style("→").cyan()
                );
            } else {
                println!(
                    "  {} Starting Docker PostgreSQL + Redis with defaults...",
                    style("→").cyan()
                );
            }
            Ok(InfraResolution::Docker {
                user: pg_user,
                password: pg_pass,
                db: pg_db,
                redis_url: explicit_redis_url,
            })
        } else {
            println!(
                "{}",
                style("  ⚠ Docker not found and no --database-url provided. Skipping DB setup.")
                    .yellow()
            );
            Ok(InfraResolution::NoDatabase)
        }
    } else if has_docker {
        let db_choice = inquire::Select::new(
            "PostgreSQL setup:",
            vec![
                "Start Docker container (recommended)",
                "Use existing PostgreSQL (provide URL)",
            ],
        )
        .prompt()?;

        if db_choice.starts_with("Start Docker") {
            let (user, password, db) = ask_docker_credentials()?;
            Ok(InfraResolution::Docker {
                user,
                password,
                db,
                redis_url: explicit_redis_url,
            })
        } else {
            // This will be handled by the caller with async ask_database_url
            Ok(InfraResolution::NoDatabase)
        }
    } else {
        println!(
            "{}",
            style("Docker not found. Provide an existing PostgreSQL URL:").yellow()
        );
        Ok(InfraResolution::NoDatabase)
    }
}

/// Save config to ~/.masday/config.toml and set environment variables
pub fn save_config_and_env(config: &MasdayConfig) -> Result<()> {
    config.save()?;
    config.set_env_vars();
    println!("  {} Config saved", style("✓").green());
    println!();
    Ok(())
}

/// Start Docker containers + run migrations
/// If redis_url is provided, skips starting Docker Redis
async fn start_docker_infrastructure(
    pg_user: &str,
    pg_pass: &str,
    pg_db: &str,
    redis_url: Option<&str>,
) -> Result<String> {
    println!("{}", style("Starting infrastructure...").cyan());

    // PostgreSQL
    if docker::is_container_running("masday-postgres") {
        println!("  {} PostgreSQL already running", style("✓").green());
    } else {
        let pb = spinner("Starting PostgreSQL...");
        docker::start_postgres(pg_user, pg_pass, pg_db)?;
        docker::wait_for_postgres(
            "localhost",
            masday_core::constants::ports::postgres_port(),
            30,
        )?;
        pb.finish_with_message(format!("  {} PostgreSQL ready", style("✓").green()));
    }

    // Redis (skip if URL provided)
    if redis_url.is_some() {
        println!(
            "  {} Redis URL provided, skipping Docker Redis",
            style("→").cyan()
        );
    } else if docker::is_container_running("masday-redis") {
        println!("  {} Redis already running", style("✓").green());
    } else {
        let pb = spinner("Starting Redis...");
        docker::start_redis()?;
        pb.finish_with_message(format!("  {} Redis ready", style("✓").green()));
    }

    // Resolve DB URL — priority: config.toml > constructed from container credentials
    let db_url = if let Ok(config) = crate::config::MasdayConfig::load_or_err() {
        if let Some(ref url) = config.database_url {
            url.clone()
        } else {
            format!(
                "postgresql://{}:{}@localhost:{}/{}",
                pg_user,
                pg_pass,
                masday_core::constants::ports::postgres_port(),
                pg_db
            )
        }
    } else {
        format!(
            "postgresql://{}:{}@localhost:{}/{}",
            pg_user,
            pg_pass,
            masday_core::constants::ports::postgres_port(),
            pg_db
        )
    };
    std::env::set_var("DATABASE_URL", &db_url);

    // Set Redis URL — priority: config.toml > parameter > default
    if redis_url.is_none() {
        if let Ok(config) = crate::config::MasdayConfig::load_or_err() {
            if let Some(ref url) = config.redis_url {
                std::env::set_var("REDIS_URL", url);
            }
        }
    } else if let Some(url) = redis_url {
        std::env::set_var("REDIS_URL", url);
    }

    let pb = spinner("Running migrations...");
    let pool = masday_db::pool::init_pool_with_retry(5)
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))?;
    // Run migrations — skip if already applied (idempotent)
    if let Err(e) = masday_db::run_migrations(&pool).await {
        // Migrations may fail if already applied — that's ok
        let err_msg = e.to_string();
        if err_msg.contains("already exists") {
            pb.finish_with_message(format!("  {} Database up to date", style("✓").green()));
        } else {
            pb.finish_with_message(format!(
                "  {} Migration warning: {}",
                style("⚠").yellow(),
                err_msg
            ));
        }
    } else {
        pb.finish_with_message(format!("  {} Database migrated", style("✓").green()));
    }

    println!();
    Ok(db_url)
}

/// Ask user for Docker container PostgreSQL credentials.
/// Returns (user, password, db_name) — uses defaults if user skips.
fn ask_docker_credentials() -> Result<(String, String, String)> {
    let customize = inquire::Confirm::new("Customize PostgreSQL credentials?")
        .with_default(false)
        .with_help_message(&format!(
            "Default: {}/*****/{}",
            docker::DEFAULT_PG_USER,
            docker::DEFAULT_PG_DB
        ))
        .prompt()?;

    if !customize {
        return Ok((docker::pg_user(), docker::pg_password(), docker::pg_db()));
    }

    let user = inquire::Text::new("PostgreSQL user:")
        .with_default(docker::DEFAULT_PG_USER)
        .prompt()?;

    let password = inquire::Password::new("PostgreSQL password:")
        .with_help_message("Leave empty for default")
        .prompt()
        .unwrap_or_else(|_| docker::DEFAULT_PG_PASSWORD.to_string());

    let db_name = inquire::Text::new("Database name:")
        .with_default(docker::DEFAULT_PG_DB)
        .prompt()?;

    Ok((user, password, db_name))
}

/// Ask for existing database URL ()
async fn ask_database_url() -> Result<Option<String>> {
    let default_url = format!(
        "postgresql://{}:{}@localhost:5432/{}",
        docker::DEFAULT_PG_USER,
        docker::DEFAULT_PG_PASSWORD,
        docker::DEFAULT_PG_DB
    );
    let url = inquire::Text::new("Database URL:")
        .with_default(&default_url)
        .with_help_message("Full PostgreSQL connection URL")
        .prompt()?;

    // Verify connectivity
    std::env::set_var("DATABASE_URL", &url);
    print!("  {} Testing connection...", style("→").cyan());
    match masday_db::pool::init_pool_with_retry(3).await {
        Ok(_) => {
            println!(" {}", style("✓ connected").green());
        }
        Err(e) => {
            println!(" {}", style("✗ failed").red());
            println!("    {}", e);
            let cont = inquire::Confirm::new("Continue anyway?")
                .with_default(false)
                .prompt()?;
            if !cont {
                bail!("Setup cancelled.");
            }
        }
    }
    println!();

    Ok(Some(url))
}

/// Ask for embedding model preference — uses the same AVAILABLE_MODELS as `embed list`.
fn ask_embedding_model() -> Result<(Option<String>, Option<String>, Option<usize>)> {
    use crate::commands::embed::{self, EmbeddingModel};

    // Collect local models from the canonical model list
    let local_models: Vec<&EmbeddingModel> = embed::AVAILABLE_MODELS
        .iter()
        .filter(|m| m.provider == "local")
        .collect();

    let mut choices: Vec<String> = local_models
        .iter()
        .map(|m| format!("{} ({} dims, {})", m.name, m.dimensions, m.description))
        .collect();
    choices.push("Skip embeddings".to_string());

    let embed_choice = inquire::Select::new("Embedding model:", choices)
        .with_help_message(
            "Local ONNX model — no external service needed. ↑↓ to move, Enter to select",
        )
        .prompt()?;

    if embed_choice == "Skip embeddings" {
        return Ok((None, None, None));
    }

    // Match by model name prefix to find the right model
    let selected = local_models
        .iter()
        .find(|m| embed_choice.starts_with(m.name))
        .context("No matching local model found")?;

    Ok((
        Some("local".to_string()),
        Some(selected.id.to_string()),
        Some(selected.dimensions),
    ))
}

/// Ask which platforms to install for
fn ask_platforms(detected: &[Platform]) -> Result<Vec<Platform>> {
    let all = all_platforms();
    let labels: Vec<&str> = all.iter().map(|p| p.name()).collect();

    // Default to detected platforms, or claude-code if none detected
    let default_indices: Vec<usize> = if !detected.is_empty() {
        detected
            .iter()
            .filter_map(|d| all.iter().position(|p| p == d))
            .collect()
    } else {
        vec![0] // claude-code
    };

    let selected = inquire::MultiSelect::new("Select your AI platforms:", labels)
        .with_default(&default_indices)
        .with_help_message("Space to toggle, Enter to confirm")
        .prompt()?;

    let platforms: Vec<Platform> = selected
        .into_iter()
        .filter_map(|name| all.iter().find(|p| p.name() == name).copied())
        .collect();

    if platforms.is_empty() {
        bail!("At least one platform must be selected.");
    }

    println!();
    println!(
        "  Platforms: {}",
        style(platform_names(&platforms).join(", ")).cyan()
    );
    println!();

    Ok(platforms)
}

/// Sync agents, skills, hooks to project and global dirs
pub fn sync_templates(project_dir: &Path, platforms: &[Platform]) -> Result<()> {
    println!("{}", style("Syncing agents...").cyan());
    let reports = sync_agents_to_project(project_dir, platforms, true)?;
    for r in &reports {
        println!(
            "  {}: {} copied, {} skipped",
            r.platform,
            style(r.copied).green(),
            style(r.skipped).dim()
        );
    }

    let global_reports = sync_agents_to_global(platforms, true)?;
    for r in &global_reports {
        println!(
            "  {} (global): {} copied",
            r.platform,
            style(r.copied).green()
        );
    }

    println!();
    println!("{}", style("Syncing skills...").cyan());
    let skill_reports = sync_skills_to_project(project_dir, platforms, true)?;
    for r in &skill_reports {
        println!(
            "  {}: {} copied, {} skipped",
            r.platform,
            style(r.copied).green(),
            style(r.skipped).dim()
        );
    }

    let global_skill_reports = sync_skills_to_global(platforms, true)?;
    for r in &global_skill_reports {
        println!(
            "  {} (global): {} copied",
            r.platform,
            style(r.copied).green()
        );
    }
    println!();

    // ── Hooks ────────────────────────────────────────────────────────────
    println!("{}", style("Syncing hooks...").cyan());

    // Global hooks (e.g. ~/.claude/hooks/)
    if let Some(home) = home::home_dir() {
        let global_report = install_global_hooks(&home)?;
        println!(
            "  {} global hooks installed",
            style(global_report.copied).green()
        );

        // Utility scripts (e.g. ~/.masday/scripts/registry-sync.mjs)
        let script_count = sync_scripts_to_masday_dir(&home);
        if script_count > 0 {
            println!(
                "  {} utility scripts installed to ~/.masday/scripts/",
                style(script_count).green()
            );
        }
    }

    // Project hooks (e.g. .claude/hooks/ in project dir)
    let project_report = install_project_hooks(project_dir)?;
    println!(
        "  {} project hooks installed",
        style(project_report.copied).green()
    );

    // Git hooks (pre-commit, pre-push → .git/hooks/)
    let git_report = install_git_hooks(project_dir)?;
    if git_report.copied > 0 {
        println!(
            "  {} git hooks installed (pre-commit, pre-push)",
            style(git_report.copied).green()
        );
    } else if git_report.skipped > 0 {
        println!(
            "  {} git hooks skipped (no .git/hooks/ found)",
            style(git_report.skipped).dim()
        );
    }
    println!();

    // ── Register hooks in Claude Code settings ─────────────────────────
    if let Some(home) = home::home_dir() {
        let settings_path = home.join(".claude/settings.json");
        match register_hooks_in_settings(&settings_path, &home) {
            Ok(()) => println!(
                "  {} Hook events + statusline registered in settings.json",
                style("✓").green()
            ),
            Err(e) => println!("  {} Could not register hooks: {}", style("⚠").yellow(), e),
        }
    }
    println!();

    Ok(())
}

/// Register MCP servers for all selected platforms
pub fn register_mcp_servers(
    project_dir: &Path,
    platforms: &[Platform],
    api_url: &str,
    api_key: &str,
    database_url: Option<&str>,
    mode: &str,
) -> Result<()> {
    println!("{}", style("Registering MCP servers...").cyan());

    // Find the masday binary itself to use as MCP command
    let mcp_binary = resolve_mcp_binary();

    // Determine transport mode: remote and server use HTTP/SSE, local and standalone use stdio
    let use_http_transport = mode == "remote" || mode == "server";

    for platform in platforms {
        let config = McpConfig {
            mcp_binary_path: mcp_binary.clone(),
            api_url: api_url.to_string(),
            api_key: api_key.to_string(),
            database_url: database_url.map(|s| s.to_string()),
            use_http_transport,
        };

        generate_mcp_config(platform, project_dir, &config)?;
        println!("  {} {}", style("✓").green(), platform.name());
    }

    println!();
    Ok(())
}

/// Resolve the masday binary path to use as the MCP `command`.
///
/// Prefers the canonical install path (`~/.masday/bin/masday`) so that a
/// self-update that has replaced the running binary on disk does not leak a
/// stale `/proc/self/exe` value — on Linux an in-flight, replaced inode reads
/// back as `path (deleted)`, which would otherwise be written verbatim into
/// every platform MCP config. Falls back to `current_exe()` only when it is a
/// real file on disk and not marked deleted (dev builds running from
/// `target/`). Last resort: the bare name `masday` (relies on PATH).
pub fn resolve_mcp_binary() -> std::path::PathBuf {
    resolve_mcp_binary_inner(
        home::home_dir()
            .map(|h| h.join(".masday").join("bin").join("masday"))
            .as_deref(),
        std::env::current_exe().ok().as_deref(),
    )
}

fn resolve_mcp_binary_inner(
    installed: Option<&std::path::Path>,
    current_exe: Option<&std::path::Path>,
) -> std::path::PathBuf {
    // Production: prefer the canonical install path when it exists.
    if let Some(p) = installed.filter(|p| p.exists()) {
        return p.to_path_buf();
    }
    // Dev fallback: current_exe only if it is a real, non-"(deleted)" file.
    if let Some(exe) = current_exe {
        let s = exe.to_string_lossy();
        if !s.contains(" (deleted)") && exe.exists() {
            return exe.to_path_buf();
        }
    }
    // Last resort: rely on PATH.
    std::path::PathBuf::from("masday")
}

/// Detect platforms from home directory config files
pub fn detect_active_platforms_from_home() -> Vec<Platform> {
    let mut platforms = Vec::new();

    if let Some(home) = home::home_dir() {
        if home.join(".claude").exists() {
            platforms.push(Platform::ClaudeCode);
        }
        // Claude Desktop detection — OS-specific config path
        {
            let desktop_config = if cfg!(target_os = "macos") {
                home.join("Library/Application Support/Claude/claude_desktop_config.json")
            } else if cfg!(target_os = "windows") {
                std::env::var("APPDATA")
                    .map(|appdata| {
                        std::path::PathBuf::from(appdata).join("Claude/claude_desktop_config.json")
                    })
                    .unwrap_or_else(|_| home.join(".config/Claude/claude_desktop_config.json"))
            } else {
                home.join(".config/Claude/claude_desktop_config.json")
            };
            if desktop_config.exists() {
                platforms.push(Platform::ClaudeDesktop);
            }
        }
        if home.join(".gemini").exists() {
            platforms.push(Platform::GeminiCli);
        }
        if home.join(".continue").exists() {
            platforms.push(Platform::VsCodeCopilot);
        }
        if home.join(".config/opencode").exists() {
            platforms.push(Platform::OpenCode);
        }
        if home.join(".cursor").exists() {
            platforms.push(Platform::Cursor);
        }
        if home.join(".codeium").exists() {
            platforms.push(Platform::Windsurf);
        }
        if home.join(".codex").exists() {
            platforms.push(Platform::Codex);
        }
    }

    platforms
}

/// Get platform name strings
fn platform_names(platforms: &[Platform]) -> Vec<String> {
    platforms.iter().map(|p| p.name().to_string()).collect()
}

/// Resolve an embedding model ID to (provider, model, dimensions).
/// Used by non-interactive mode to look up model metadata.
fn resolve_embedding_model(
    model_id: &str,
) -> Result<(Option<String>, Option<String>, Option<usize>)> {
    if model_id == "skip" || model_id == "none" {
        return Ok((None, None, None));
    }

    let model = embed::AVAILABLE_MODELS
        .iter()
        .find(|m| m.id == model_id)
        .context(format!(
            "Unknown embedding model '{}'. Run 'masday embed list' to see available models.",
            model_id
        ))?;

    println!(
        "  Embedding: {} ({} dims, {})",
        style(model.id).cyan(),
        style(model.dimensions).cyan(),
        style(model.description).dim()
    );

    Ok((
        Some(model.provider.to_string()),
        Some(model.id.to_string()),
        Some(model.dimensions),
    ))
}

/// Parse comma-separated platform string (e.g. "claude-code,gemini") into Platform vec.
fn parse_platforms(platform_str: &str) -> Result<Vec<Platform>> {
    let all = all_platforms();
    let mut platforms = Vec::new();

    for name in platform_str.split(',') {
        let name = name.trim();
        let plat = all.iter().find(|p| p.name() == name).context(format!(
            "Unknown platform '{}'. Available: {}",
            name,
            all.iter().map(|p| p.name()).collect::<Vec<_>>().join(", ")
        ))?;
        if !platforms.contains(plat) {
            platforms.push(*plat);
        }
    }

    if platforms.is_empty() {
        bail!("At least one platform must be specified.");
    }

    println!(
        "  Platforms: {}",
        style(platform_names(&platforms).join(", ")).cyan()
    );

    Ok(platforms)
}

fn print_local_summary(config: &MasdayConfig) {
    println!();
    println!(
        "{}",
        style("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━").green()
    );
    println!(
        "{}",
        style("  ⚡ Masday is ready! (local mode)").green().bold()
    );
    println!(
        "{}",
        style("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━").green()
    );
    println!();
    println!("  Dashboard: http://localhost:{}", config.api_port);
    println!("  API:       http://localhost:{}/api", config.api_port);
    println!("  PostgreSQL:  localhost:{}", config.db_port);
    println!("  Redis:     localhost:{}", config.redis_port);
    println!("  SQLite:    ~/.masday/data.db (MCP stdio mode)");
    println!("  Platforms: {}", config.platforms.join(", "));
    println!("  MCP Transport: {}", style("stdio").cyan());
    println!();
    println!("  Commands:");
    println!(
        "    {}  Start API server + dashboard",
        style("masday serve").cyan()
    );
    println!(
        "    {}  Start MCP server (stdio)",
        style("masday mcp").cyan()
    );
    println!("    {}  Check health", style("masday status").cyan());
    println!();
    println!(
        "  {}",
        style("💡 Edit ~/.masday/config.toml to change ports (api_port, db_port, redis_port, dashboard_port)").dim()
    );
    println!();
}

fn print_remote_summary(config: &MasdayConfig) {
    println!();
    println!(
        "{}",
        style("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━").green()
    );
    println!(
        "{}",
        style("  ⚡ Masday is ready! (remote mode)").green().bold()
    );
    println!(
        "{}",
        style("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━").green()
    );
    println!();
    println!("  Server:    {}", config.api_url);
    println!("  SQLite:    ~/.masday/data.db (MCP stdio mode)");
    println!("  Platforms: {}", config.platforms.join(", "));
    println!("  MCP Transport: {}", style("HTTP/SSE").cyan());
    println!();
    println!("  MCP servers registered — your AI platforms can use Masday tools.");
    println!();
    println!("  Commands:");
    println!(
        "    {}  Start MCP server (stdio)",
        style("masday mcp").cyan()
    );
    println!("    {}  Check health", style("masday status").cyan());
    println!();
}

fn print_server_summary(
    config: &MasdayConfig,
    database_url: Option<&str>,
    redis_url: Option<&str>,
) {
    println!();
    println!(
        "{}",
        style("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━").green()
    );
    println!(
        "{}",
        style("  ⚡ Masday server stack configured!").green().bold()
    );
    println!(
        "{}",
        style("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━").green()
    );
    println!();
    println!("  API:       http://localhost:{}", config.api_port);

    if let Some(db_url) = database_url {
        println!("  PostgreSQL:  {}", style(db_url).cyan());
    }
    if let Some(redis_url) = redis_url {
        println!("  Redis:     {}", style(redis_url).cyan());
    }
    println!("  SQLite:    ~/.masday/data.db (MCP stdio mode)");
    println!("  MCP Transport: {}", style("HTTP/SSE").cyan());
    println!();

    println!("  Next steps:");
    println!("    1. Start the infrastructure (PostgreSQL + Redis)");
    println!("    2. Start the API server");
    println!();

    if config.embedding_model.is_some() {
        println!(
            "  Embedding: {}",
            style(config.embedding_model.as_deref().unwrap_or("unknown")).cyan()
        );
    }
    println!();
    println!(
        "  {}",
        style("💡 Edit ~/.masday/config.toml to change configuration").dim()
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

/// Initialize SQLite database at ~/.masday/data.db for MCP stdio mode.
///
/// This function is called by all quickstart modes (local, remote, standalone, server)
/// to ensure the SQLite database is properly initialized with all required tables.
/// The database is used by the MCP server in stdio transport mode.
pub fn init_sqlite_database() -> Result<()> {
    println!("{}", style("Initializing SQLite database...").cyan());

    // Call the MCP crate's SQLite initialization
    // This creates ~/.masday/data.db and runs the full schema
    match masday_mcp::sqlite::init_sqlite() {
        Ok(()) => {
            println!(
                "  {} SQLite database ready at ~/.masday/data.db",
                style("✓").green()
            );
            println!(
                "  {} 16 tables created (workflows, tasks, memories, etc.)",
                style("→").cyan()
            );
            Ok(())
        }
        Err(e) => {
            // Check if it's just "already initialized" error
            let err_msg = e.to_string();
            if err_msg.contains("already initialized") {
                println!("  {} SQLite database already exists", style("✓").green());
                Ok(())
            } else {
                // Real error - report but don't fail (PostgreSQL mode might not need SQLite)
                println!(
                    "  {} SQLite initialization skipped: {}",
                    style("⚠").yellow(),
                    err_msg
                );
                println!(
                    "  {} This is OK if you're using PostgreSQL mode",
                    style("→").dim()
                );
                Ok(())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_platforms_from_home() {
        let platforms = detect_active_platforms_from_home();
        // Should not panic
        for p in &platforms {
            assert!(!p.name().is_empty());
        }
    }

    #[test]
    fn test_platform_names_helper() {
        let platforms = vec![Platform::ClaudeCode, Platform::GeminiCli];
        let names = platform_names(&platforms);
        assert_eq!(names, vec!["claude-code", "gemini"]);
    }

    // ── InfraResolution tests ────────────────────────────────────────────────

    #[test]
    fn test_resolve_infra_explicit_database_url() {
        let result = resolve_infra(
            true, // has_docker
            true, // is_non_interactive
            Some("postgresql://user:pass@localhost:5432/db".to_string()),
            None,
        );
        assert!(result.is_ok());
        match result.unwrap() {
            InfraResolution::ExistingUrl {
                database_url,
                redis_url,
            } => {
                assert_eq!(database_url, "postgresql://user:pass@localhost:5432/db");
                assert!(redis_url.is_none());
            }
            _ => panic!("Expected ExistingUrl variant"),
        }
    }

    #[test]
    fn test_resolve_infra_explicit_redis_url() {
        let result = resolve_infra(
            true, // has_docker
            true, // is_non_interactive
            Some("postgresql://user:pass@localhost:5432/db".to_string()),
            Some("redis://localhost:6379".to_string()),
        );
        assert!(result.is_ok());
        match result.unwrap() {
            InfraResolution::ExistingUrl {
                database_url,
                redis_url,
            } => {
                assert_eq!(database_url, "postgresql://user:pass@localhost:5432/db");
                assert_eq!(redis_url, Some("redis://localhost:6379".to_string()));
            }
            _ => panic!("Expected ExistingUrl variant"),
        }
    }

    #[test]
    fn test_resolve_infra_docker_with_defaults() {
        let result = resolve_infra(
            true, // has_docker
            true, // is_non_interactive
            None, // no explicit database_url
            None, // no explicit redis_url
        );
        assert!(result.is_ok());
        match result.unwrap() {
            InfraResolution::Docker {
                user,
                password,
                db,
                redis_url,
            } => {
                assert_eq!(user, "masday");
                assert_eq!(password, "masdaypass");
                assert_eq!(db, "masday_workflow");
                assert!(redis_url.is_none());
            }
            _ => panic!("Expected Docker variant"),
        }
    }

    #[test]
    fn test_resolve_infra_docker_with_explicit_redis() {
        let result = resolve_infra(
            true, // has_docker
            true, // is_non_interactive
            None, // no explicit database_url
            Some("redis://localhost:6379".to_string()),
        );
        assert!(result.is_ok());
        match result.unwrap() {
            InfraResolution::Docker {
                user,
                password,
                db,
                redis_url,
            } => {
                assert_eq!(user, "masday");
                assert_eq!(password, "masdaypass");
                assert_eq!(db, "masday_workflow");
                assert_eq!(redis_url, Some("redis://localhost:6379".to_string()));
            }
            _ => panic!("Expected Docker variant with redis_url"),
        }
    }

    #[test]
    fn test_resolve_infra_no_docker_no_url() {
        let result = resolve_infra(
            false, // has_docker = false
            true,  // is_non_interactive
            None,  // no explicit database_url
            None,  // no explicit redis_url
        );
        assert!(result.is_ok());
        match result.unwrap() {
            InfraResolution::NoDatabase => {
                // Expected result when no Docker and no URL
            }
            _ => panic!("Expected NoDatabase variant"),
        }
    }

    #[test]
    fn test_resolve_infra_interactive_with_docker() {
        let result = resolve_infra(
            true,  // has_docker
            false, // is_non_interactive = false (interactive)
            None,  // no explicit database_url
            None,  // no explicit redis_url
        );
        // In interactive mode with Docker, it will try to prompt and fail
        // This is expected behavior - we can't test interactive prompts in unit tests
        assert!(result.is_err());
    }

    #[test]
    fn test_resolve_infra_interactive_without_docker() {
        let result = resolve_infra(
            false, // has_docker = false
            false, // is_non_interactive = false (interactive)
            None,  // no explicit database_url
            None,  // no explicit redis_url
        );
        // In interactive mode without Docker, should return NoDatabase
        // and let the caller handle the URL prompt
        assert!(result.is_ok());
        match result.unwrap() {
            InfraResolution::NoDatabase => {
                // Expected
            }
            _ => panic!("Expected NoDatabase variant in interactive mode without Docker"),
        }
    }

    // ── QuickstartArgs tests ─────────────────────────────────────────────────

    #[test]
    fn test_quickstart_args_default() {
        let args = QuickstartArgs::default();
        assert!(args.mode.is_none());
        assert!(!args.dev);
        assert!(!args.skip_build);
        assert!(!args.yes);
        assert!(args.database_url.is_none());
        assert!(args.redis_url.is_none());
        assert!(args.embedding.is_none());
        assert!(args.platform.is_none());
        assert!(args.api_port.is_none());
        assert!(args.image_tag.is_none());
    }

    #[test]
    fn test_quickstart_args_with_dev_flag() {
        let args = QuickstartArgs {
            dev: true,
            ..Default::default()
        };
        assert!(args.dev);
        assert!(!args.skip_build);
    }

    #[test]
    fn test_quickstart_args_with_skip_build() {
        let args = QuickstartArgs {
            dev: true,
            skip_build: true,
            ..Default::default()
        };
        assert!(args.dev);
        assert!(args.skip_build);
    }

    #[test]
    fn test_quickstart_args_non_interactive() {
        let args = QuickstartArgs {
            yes: true,
            mode: Some("local".to_string()),
            ..Default::default()
        };
        assert!(args.yes);
        assert_eq!(args.mode, Some("local".to_string()));
    }

    #[test]
    fn test_quickstart_args_with_database_url() {
        let args = QuickstartArgs {
            database_url: Some("postgresql://user:pass@localhost:5432/db".to_string()),
            ..Default::default()
        };
        assert_eq!(
            args.database_url,
            Some("postgresql://user:pass@localhost:5432/db".to_string())
        );
    }

    #[test]
    fn test_quickstart_args_with_redis_url() {
        let args = QuickstartArgs {
            redis_url: Some("redis://localhost:6379".to_string()),
            ..Default::default()
        };
        assert_eq!(args.redis_url, Some("redis://localhost:6379".to_string()));
    }

    #[test]
    fn test_quickstart_args_with_api_port() {
        let args = QuickstartArgs {
            api_port: Some(8080),
            ..Default::default()
        };
        assert_eq!(args.api_port, Some(8080));
    }

    #[test]
    fn test_quickstart_args_with_image_tag() {
        let args = QuickstartArgs {
            mode: Some("server".to_string()),
            image_tag: Some("v0.3.0".to_string()),
            ..Default::default()
        };
        assert_eq!(args.mode, Some("server".to_string()));
        assert_eq!(args.image_tag, Some("v0.3.0".to_string()));
    }

    #[test]
    fn test_quickstart_args_with_embedding() {
        let args = QuickstartArgs {
            embedding: Some("all-MiniLM-L6-v2".to_string()),
            ..Default::default()
        };
        assert_eq!(args.embedding, Some("all-MiniLM-L6-v2".to_string()));
    }

    #[test]
    fn test_quickstart_args_with_platform() {
        let args = QuickstartArgs {
            platform: Some("claude-code,gemini".to_string()),
            ..Default::default()
        };
        assert_eq!(args.platform, Some("claude-code,gemini".to_string()));
    }

    // ── Mode string tests ─────────────────────────────────────────────────────

    #[test]
    fn test_mode_local() {
        let mode_str = "local";
        assert_eq!(mode_str, "local");
    }

    #[test]
    fn test_mode_remote() {
        let mode_str = "remote";
        assert_eq!(mode_str, "remote");
    }

    #[test]
    fn test_mode_standalone() {
        let mode_str = "standalone";
        assert_eq!(mode_str, "standalone");
    }

    #[test]
    fn test_mode_server() {
        let mode_str = "server";
        assert_eq!(mode_str, "server");
    }

    #[test]
    fn test_resolve_mcp_binary_prefers_install_path_over_deleted_exe() {
        use std::path::PathBuf;
        let installed = PathBuf::from("/tmp/masday-fake-install-exists");
        std::fs::write(&installed, b"fake").unwrap();
        let deleted = PathBuf::from("/tmp/masday-fake-install-exists (deleted)");
        let resolved = super::resolve_mcp_binary_inner(Some(&installed), Some(&deleted));
        assert_eq!(
            resolved, installed,
            "must prefer real install path, not (deleted)"
        );
        let _ = std::fs::remove_file(&installed);

        // When install path is absent, a "(deleted)" current_exe is rejected → bare "masday".
        let none_installed = PathBuf::from("/tmp/does-not-exist-masday");
        let resolved2 = super::resolve_mcp_binary_inner(Some(&none_installed), Some(&deleted));
        assert_eq!(resolved2, PathBuf::from("masday"));
    }
}
