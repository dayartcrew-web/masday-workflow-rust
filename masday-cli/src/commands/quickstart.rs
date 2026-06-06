//! Quickstart command — one command to set up everything.
//!
//! Interactive wizard that handles:
//! - Mode selection (local/remote/standalone)
//! - DB setup (Docker or existing or remote URL)
//! - API key configuration
//! - Platform detection + MCP server registration
//! - Agent/skill/hook sync

use anyhow::{bail, Result};
use console::style;
use indicatif::{ProgressBar, ProgressStyle};
use std::path::Path;

// InstallArgs removed — quickstart uses sync functions directly
use crate::config::MasdayConfig;
use crate::docker;
use crate::installer::{
    all_platforms, generate_mcp_config, install_global_hooks, install_project_hooks,
    register_hooks_in_settings, sync_agents_to_global, sync_agents_to_project,
    sync_skills_to_global, sync_skills_to_project, McpConfig, Platform,
};

/// Run the quickstart wizard.
pub async fn run(project_dir: &Path) -> Result<()> {
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
    let mode_options = vec![
        "Local — everything on this machine (Docker)",
        "Remote — connect to existing API server",
        "Standalone — agents & skills only (no DB, no API)",
    ];

    let mode_choice = inquire::Select::new("How will you run Masday?", mode_options)
        .with_help_message("↑↓ to move, Enter to select")
        .prompt()?;

    println!();

    match mode_choice {
        s if s.starts_with("Local") => {
            run_local_mode(project_dir, &detected_platforms, has_docker).await?
        }
        s if s.starts_with("Remote") => run_remote_mode(project_dir, &detected_platforms)?,
        s if s.starts_with("Standalone") => run_standalone_mode(project_dir, &detected_platforms)?,
        _ => bail!("Invalid selection"),
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
) -> Result<()> {
    println!("{}", style("── Local Mode ──").cyan().bold());
    println!();

    // ── DB Setup ──────────────────────────────────────────────────────────
    let database_url = if has_docker {
        let db_choice = inquire::Select::new(
            "PostgreSQL setup:",
            vec![
                "Start Docker container (recommended)",
                "Use existing PostgreSQL (provide URL)",
            ],
        )
        .prompt()?;

        if db_choice.starts_with("Start Docker") {
            let (pg_user, pg_pass, pg_db) = ask_docker_credentials()?;
            Some(start_docker_infrastructure(&pg_user, &pg_pass, &pg_db).await?)
        } else {
            ask_database_url().await?
        }
    } else {
        println!(
            "{}",
            style("Docker not found. Provide an existing PostgreSQL URL:").yellow()
        );
        ask_database_url().await?
    };

    // ── Embedding (optional) ──────────────────────────────────────────────
    let (embed_provider, embed_model, embed_dims) = ask_embedding_model()?;

    // ── Platforms ─────────────────────────────────────────────────────────
    let platforms = ask_platforms(detected_platforms)?;

    // ── Save config ───────────────────────────────────────────────────────
    let api_url = format!(
        "http://localhost:{}",
        masday_core::constants::ports::api_port()
    );
    let config = MasdayConfig {
        mode: "local".to_string(),
        api_url: api_url.clone(),
        api_key: "***".to_string(),
        database_url: database_url.clone(),
        embedding_provider: embed_provider,
        embedding_model: embed_model,
        embedding_dimensions: embed_dims,
        embedding_base_url: None,
        embedding_api_key: None,
        api_port: masday_core::constants::ports::API_PORT,
        db_port: masday_core::constants::ports::POSTGRES_PORT,
        redis_port: masday_core::constants::ports::REDIS_PORT,
        dashboard_port: masday_core::constants::ports::API_PORT,
        platforms: platform_names(&platforms),
    };
    config.save()?;
    config.set_env_vars();
    println!("  {} Config saved", style("✓").green());
    println!();

    // ── Sync templates ────────────────────────────────────────────────────
    sync_templates(project_dir, &platforms)?;

    // ── Register MCP servers ──────────────────────────────────────────────
    register_mcp_servers(
        project_dir,
        &platforms,
        &api_url,
        "***",
        database_url.as_deref(),
    )?;

    // ── Summary ───────────────────────────────────────────────────────────
    print_local_summary(&config);
    Ok(())
}

// ═══════════════════════════════════════════════════════════════════════════
// REMOTE MODE
// ═══════════════════════════════════════════════════════════════════════════

fn run_remote_mode(project_dir: &Path, detected_platforms: &[Platform]) -> Result<()> {
    println!("{}", style("── Remote Mode ──").cyan().bold());
    println!();

    // ── API Server URL ────────────────────────────────────────────────────
    let api_url = inquire::Text::new("API server URL:")
        .with_default("https://masday.example.com")
        .with_help_message("HTTPS recommended. e.g. https://masday.yourcompany.com")
        .prompt()?;

    let api_url = api_url.trim_end_matches('/').to_string();
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
    let platforms = ask_platforms(detected_platforms)?;

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
    config.save()?;
    config.set_env_vars();
    println!("  {} Config saved", style("✓").green());
    println!();

    // ── Sync templates ────────────────────────────────────────────────────
    sync_templates(project_dir, &platforms)?;

    // ── Register MCP servers ──────────────────────────────────────────────
    register_mcp_servers(
        project_dir,
        &platforms,
        &api_url,
        &api_key,
        database_url.as_deref(),
    )?;

    // ── Summary ───────────────────────────────────────────────────────────
    print_remote_summary(&config);
    Ok(())
}

// ═══════════════════════════════════════════════════════════════════════════
// STANDALONE MODE
// ═══════════════════════════════════════════════════════════════════════════

fn run_standalone_mode(project_dir: &Path, detected_platforms: &[Platform]) -> Result<()> {
    println!("{}", style("── Standalone Mode ──").cyan().bold());
    println!();

    let platforms = ask_platforms(detected_platforms)?;

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
    config.save()?;
    println!("  {} Config saved", style("✓").green());
    println!();

    // ── Sync templates ────────────────────────────────────────────────────
    sync_templates(project_dir, &platforms)?;

    // ── Register MCP servers (stdio, no API) ──────────────────────────────
    register_mcp_servers(project_dir, &platforms, "", "", None)?;

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

/// Start Docker containers + run migrations
async fn start_docker_infrastructure(pg_user: &str, pg_pass: &str, pg_db: &str) -> Result<String> {
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

    // Redis
    if docker::is_container_running("masday-redis") {
        println!("  {} Redis already running", style("✓").green());
    } else {
        let pb = spinner("Starting Redis...");
        docker::start_redis()?;
        pb.finish_with_message(format!("  {} Redis ready", style("✓").green()));
    }

    // Migrations — build URL from the same credentials used for container
    let db_url = format!(
        "postgresql://{}:{}@localhost:{}/{}",
        pg_user,
        pg_pass,
        masday_core::constants::ports::postgres_port(),
        pg_db
    );
    std::env::set_var("DATABASE_URL", &db_url);

    let pb = spinner("Running migrations...");
    let pool = masday_db::pool::init_pool_with_retry(5)
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))?;
    masday_db::run_migrations(&pool)
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))?;
    pb.finish_with_message(format!("  {} Database migrated", style("✓").green()));

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

/// Ask for embedding model preference ()
fn ask_embedding_model() -> Result<(Option<String>, Option<String>, Option<usize>)> {
    let embed_choice = inquire::Select::new(
        "Embedding model:",
        vec![
            "all-MiniLM-L6-v2 (384 dims, fast, ~80MB)",
            "bge-small-en-v1.5 (384 dims, balanced)",
            "bge-base-en-v1.5 (768 dims, accurate)",
            "Skip embeddings",
        ],
    )
    .with_help_message("Local ONNX model — no external service needed")
    .prompt()?;

    let result = match embed_choice {
        s if s.starts_with("all-MiniLM") => (
            Some("local".into()),
            Some("all-MiniLM-L6-v2".into()),
            Some(384),
        ),
        s if s.starts_with("bge-small") => (
            Some("local".into()),
            Some("bge-small-en-v1.5".into()),
            Some(384),
        ),
        s if s.starts_with("bge-base") => (
            Some("local".into()),
            Some("bge-base-en-v1.5".into()),
            Some(768),
        ),
        _ => (None, None, None),
    };

    Ok(result)
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
fn sync_templates(project_dir: &Path, platforms: &[Platform]) -> Result<()> {
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
    }

    // Project hooks (e.g. .claude/hooks/ in project dir)
    let project_report = install_project_hooks(project_dir)?;
    println!(
        "  {} project hooks installed",
        style(project_report.copied).green()
    );
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
fn register_mcp_servers(
    project_dir: &Path,
    platforms: &[Platform],
    api_url: &str,
    api_key: &str,
    database_url: Option<&str>,
) -> Result<()> {
    println!("{}", style("Registering MCP servers...").cyan());

    // Find the masday binary itself to use as MCP command
    let mcp_binary = std::env::current_exe().unwrap_or_else(|_| "masday".into());

    for platform in platforms {
        let config = McpConfig {
            mcp_binary_path: mcp_binary.clone(),
            api_url: api_url.to_string(),
            api_key: api_key.to_string(),
            database_url: database_url.map(|s| s.to_string()),
        };

        generate_mcp_config(platform, project_dir, &config)?;
        println!("  {} {}", style("✓").green(), platform.name());
    }

    println!();
    Ok(())
}

/// Detect platforms from home directory config files
fn detect_active_platforms_from_home() -> Vec<Platform> {
    let mut platforms = Vec::new();

    if let Some(home) = home::home_dir() {
        if home.join(".claude").exists() {
            platforms.push(Platform::ClaudeCode);
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
    }

    platforms
}

/// Get platform name strings
fn platform_names(platforms: &[Platform]) -> Vec<String> {
    platforms.iter().map(|p| p.name().to_string()).collect()
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
    println!("  Database:  localhost:{}", config.db_port);
    println!("  Redis:     localhost:{}", config.redis_port);
    println!("  Platforms: {}", config.platforms.join(", "));
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
    println!("  Platforms: {}", config.platforms.join(", "));
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
}
