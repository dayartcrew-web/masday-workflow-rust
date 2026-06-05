//! Status command — health check for all Masday services.

use anyhow::Result;
use console::style;
use masday_core::constants::ports;
use serde::Serialize;
use std::collections::HashMap;
use std::process::Command;

use crate::config::MasdayConfig;

/// Health status for a component
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum HealthStatus {
    Healthy,
    Degraded,
    Unhealthy,
    NotConfigured,
    Unknown,
}

/// Component health information
#[derive(Debug, Clone, Serialize)]
pub struct ComponentHealth {
    status: HealthStatus,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    details: Option<HashMap<String, String>>,
}

/// Overall system health report
#[derive(Debug, Clone, Serialize)]
pub struct HealthReport {
    masday_version: String,
    mode: String,
    platform: String,
    config_path: String,
    config_valid: bool,
    api: ComponentHealth,
    database: ComponentHealth,
    redis: ComponentHealth,
    mcp: ComponentHealth,
    embedding: ComponentHealth,
    agents_count: usize,
    skills_count: usize,
    hooks_count: usize,
}

impl HealthReport {
    /// Calculate overall health status
    pub fn overall_status(&self) -> (i32, String) {
        let core_healthy = matches!(self.api.status, HealthStatus::Healthy)
            && matches!(self.database.status, HealthStatus::Healthy);

        if !core_healthy {
            return (2, "critical".to_string());
        }

        let has_issues = !matches!(self.redis.status, HealthStatus::Healthy)
            || !matches!(self.mcp.status, HealthStatus::Healthy)
            || !matches!(self.embedding.status, HealthStatus::Healthy);

        if has_issues {
            return (1, "degraded".to_string());
        }

        (0, "healthy".to_string())
    }
}

/// Check health of all Masday services
pub async fn run(json: bool, verbose: bool) -> Result<()> {
    let config = MasdayConfig::load_or_err()?;
    let report = build_health_report(&config, verbose).await?;

    if json {
        output_json(&report)?;
    } else {
        output_table(&report, verbose)?;
    }

    let (exit_code, _status_str) = report.overall_status();
    if !json {
        if exit_code == 0 {
            println!("\n  Status: {}", style("✓ All healthy").green());
        } else if exit_code == 1 {
            println!("\n  Status: {}", style("⚠ Partial degradation").yellow());
        } else {
            println!("\n  Status: {}", style("✗ Critical failure").red());
        }
    }

    std::process::exit(exit_code);
}

/// Build comprehensive health report
async fn build_health_report(config: &MasdayConfig, verbose: bool) -> Result<HealthReport> {
    let version = env!("CARGO_PKG_VERSION").to_string();

    // API health check
    let api_health = check_api_health(config, verbose).await;

    // Database health check
    let db_health = check_database_health(config, verbose).await;

    // Redis health check
    let redis_health = check_redis_health(config, verbose);

    // MCP health check
    let mcp_health = check_mcp_health(verbose);

    // Embedding health check
    let embedding_health = check_embedding_health(config, verbose);

    // Count agents, skills, hooks
    let (agents_count, skills_count, hooks_count) = count_assets()?;

    let config_path = MasdayConfig::config_path().display().to_string();
    let config_valid = config_path.contains("config.toml");

    let platform = detect_platforms();

    Ok(HealthReport {
        masday_version: version,
        mode: config.mode.clone(),
        platform,
        config_path,
        config_valid,
        api: api_health,
        database: db_health,
        redis: redis_health,
        mcp: mcp_health,
        embedding: embedding_health,
        agents_count,
        skills_count,
        hooks_count,
    })
}

/// Check API server health
async fn check_api_health(config: &MasdayConfig, verbose: bool) -> ComponentHealth {
    let health_url = format!("{}/api/health", config.api_url);
    let start_time = std::time::Instant::now();

    match reqwest::get(&health_url).await {
        Ok(resp) if resp.status().is_success() => {
            let latency = start_time.elapsed().as_millis();
            let mut details = HashMap::new();
            details.insert("url".to_string(), config.api_url.clone());
            if verbose {
                details.insert("latency_ms".to_string(), latency.to_string());
                details.insert("status_code".to_string(), resp.status().as_u16().to_string());
            }

            ComponentHealth {
                status: HealthStatus::Healthy,
                message: format!("{} healthy", style("✓").green()),
                details: if verbose { Some(details) } else { None },
            }
        }
        Ok(resp) => {
            let mut details = HashMap::new();
            details.insert("url".to_string(), config.api_url.clone());
            details.insert("status_code".to_string(), resp.status().as_u16().to_string());

            ComponentHealth {
                status: HealthStatus::Degraded,
                message: format!("{} unhealthy ({})", style("⚠").yellow(), resp.status()),
                details: if verbose { Some(details) } else { None },
            }
        }
        Err(e) => {
            let mut details = HashMap::new();
            details.insert("url".to_string(), config.api_url.clone());
            details.insert("error".to_string(), e.to_string());

            ComponentHealth {
                status: HealthStatus::Unhealthy,
                message: format!("{} not running", style("✗").red()),
                details: if verbose { Some(details) } else { None },
            }
        }
    }
}

/// Check database health
async fn check_database_health(config: &MasdayConfig, verbose: bool) -> ComponentHealth {
    if config.mode != "local" {
        return ComponentHealth {
            status: HealthStatus::NotConfigured,
            message: "— not in local mode".to_string(),
            details: None,
        };
    }

    if let Some(ref db_url) = config.database_url {
        check_postgres_external(db_url, verbose).await
    } else {
        // Try to connect to PostgreSQL on configured port
        let db_port = config.db_port;

        // Build connection string from defaults
        let default_url = format!(
            "postgresql://USER:PASS@127.0.0.1:{}/masday",
            db_port
        );

        // First try actual DB connection
        match try_postgres_connect(&default_url).await {
            Some(health) => health,
            None => {
                // If full connection fails, try just TCP port check
                if let Ok(stream) = tokio::net::TcpStream::connect(format!("127.0.0.1:{}", db_port)).await {
                    drop(stream);
                    let mut details = HashMap::new();
                    details.insert("port".to_string(), db_port.to_string());
                    if verbose {
                        details.insert("type".to_string(), "native".to_string());
                        details.insert("hint".to_string(), "port open but cannot connect — check credentials/database_url".to_string());
                    }
                    ComponentHealth {
                        status: HealthStatus::Degraded,
                        message: format!("{} port {} open but not accessible", style("⚠").yellow(), db_port),
                        details: if verbose { Some(details) } else { None },
                    }
                } else if crate::docker::is_docker_available() {
                    check_postgres_docker(verbose)
                } else {
                    ComponentHealth {
                        status: HealthStatus::NotConfigured,
                        message: "— no database configured".to_string(),
                        details: None,
                    }
                }
            }
        }
    }
}

/// Try connecting to PostgreSQL with a connection string.
/// Returns Some(health) on success or definitive failure, None if can't connect at all.
async fn try_postgres_connect(db_url: &str) -> Option<ComponentHealth> {
    std::env::set_var("DATABASE_URL", db_url);
    let start_time = std::time::Instant::now();

    match masday_db::pool::init_pool_with_retry(2).await {
        Ok(pool) => {
            match masday_db::pool::health_check(&pool).await {
                Ok(_) => {
                    let latency = start_time.elapsed().as_millis();
                    let mut details = HashMap::new();
                    details.insert("latency_ms".to_string(), latency.to_string());
                    details.insert("type".to_string(), "native".to_string());

                    Some(ComponentHealth {
                        status: HealthStatus::Healthy,
                        message: format!("{} connected", style("✓").green()),
                        details: Some(details),
                    })
                }
                Err(e) => {
                    let mut details = HashMap::new();
                    details.insert("error".to_string(), e.to_string());

                    Some(ComponentHealth {
                        status: HealthStatus::Degraded,
                        message: format!("{} health check failed", style("⚠").yellow()),
                        details: Some(details),
                    })
                }
            }
        }
        Err(_) => None, // Can't connect at all — caller should try TCP fallback
    }
}

/// Check external PostgreSQL database
async fn check_postgres_external(db_url: &str, verbose: bool) -> ComponentHealth {
    std::env::set_var("DATABASE_URL", db_url);
    let start_time = std::time::Instant::now();

    match masday_db::pool::init_pool_with_retry(2).await {
        Ok(pool) => {
            match masday_db::pool::health_check(&pool).await {
                Ok(_) => {
                    let safe_url = redact_db_url(db_url);
                    let latency = start_time.elapsed().as_millis();

                    let mut details = HashMap::new();
                    details.insert("url".to_string(), safe_url.clone());
                    if verbose {
                        details.insert("latency_ms".to_string(), latency.to_string());
                        details.insert("type".to_string(), "external".to_string());
                    }

                    ComponentHealth {
                        status: HealthStatus::Healthy,
                        message: format!("{} connected", style("✓").green()),
                        details: if verbose { Some(details) } else { None },
                    }
                }
                Err(e) => {
                    let mut details = HashMap::new();
                    details.insert("url".to_string(), redact_db_url(db_url));
                    details.insert("error".to_string(), e.to_string());

                    ComponentHealth {
                        status: HealthStatus::Degraded,
                        message: format!("{} health check failed", style("⚠").yellow()),
                        details: if verbose { Some(details) } else { None },
                    }
                }
            }
        }
        Err(e) => {
            let mut details = HashMap::new();
            details.insert("url".to_string(), redact_db_url(db_url));
            details.insert("error".to_string(), e.to_string());

            ComponentHealth {
                status: HealthStatus::Unhealthy,
                message: format!("{} unreachable", style("✗").red()),
                details: if verbose { Some(details) } else { None },
            }
        }
    }
}

/// Check Docker PostgreSQL container
fn check_postgres_docker(verbose: bool) -> ComponentHealth {
    if crate::docker::is_container_running("masday-postgres") {
        let mut details = HashMap::new();
        details.insert("port".to_string(), ports::postgres_port().to_string());
        if verbose {
            details.insert("type".to_string(), "docker".to_string());
            details.insert("container".to_string(), "masday-postgres".to_string());
        }

        ComponentHealth {
            status: HealthStatus::Healthy,
            message: format!("{} running", style("✓").green()),
            details: if verbose { Some(details) } else { None },
        }
    } else {
        let mut details = HashMap::new();
        if verbose {
            details.insert("hint".to_string(), "run 'masday db start'".to_string());
        }

        ComponentHealth {
            status: HealthStatus::Unhealthy,
            message: format!("{} not running", style("✗").red()),
            details: if verbose { Some(details) } else { None },
        }
    }
}

/// Check Redis health
fn check_redis_health(config: &MasdayConfig, verbose: bool) -> ComponentHealth {
    if config.mode != "local" {
        return ComponentHealth {
            status: HealthStatus::NotConfigured,
            message: "— not in local mode".to_string(),
            details: None,
        };
    }

    let redis_port = config.redis_port;

    // 1. Try direct TCP connection + PING
    if let Ok(stream) = std::net::TcpStream::connect(format!("127.0.0.1:{}", redis_port)) {
        let ping_ok = redis_ping_check(&stream);
        let mut details = HashMap::new();
        details.insert("port".to_string(), redis_port.to_string());
        if verbose {
            details.insert("type".to_string(), if ping_ok { "native" } else { "tcp-open" }.to_string());
        }

        return ComponentHealth {
            status: if ping_ok { HealthStatus::Healthy } else { HealthStatus::Degraded },
            message: if ping_ok {
                format!("{} connected (port {})", style("✓").green(), redis_port)
            } else {
                format!("{} port open but not responding to PING", style("⚠").yellow())
            },
            details: if verbose { Some(details) } else { None },
        };
    }

    // 2. Check Docker container
    if crate::docker::is_docker_available() && crate::docker::is_container_running("masday-redis") {
        let mut details = HashMap::new();
        details.insert("port".to_string(), redis_port.to_string());
        if verbose {
            details.insert("type".to_string(), "docker".to_string());
            details.insert("container".to_string(), "masday-redis".to_string());
        }

        return ComponentHealth {
            status: HealthStatus::Healthy,
            message: format!("{} running (Docker)", style("✓").green()),
            details: if verbose { Some(details) } else { None },
        };
    }

    // 3. Not available
    let mut details = HashMap::new();
    if verbose {
        details.insert("hint".to_string(), "run 'masday db start' or install Redis".to_string());
    }

    ComponentHealth {
        status: HealthStatus::Unhealthy,
        message: format!("{} not running", style("✗").red()),
        details: if verbose { Some(details) } else { None },
    }
}

/// Send Redis PING command and check for PONG response
fn redis_ping_check(mut stream: &std::net::TcpStream) -> bool {
    use std::io::{Read, Write};
    let ping_cmd = "*1\r\n$4\r\nPING\r\n".to_string();
    if stream.write_all(ping_cmd.as_bytes()).is_err() {
        return false;
    }
    let mut buf = [0u8; 64];
    match stream.read(&mut buf) {
        Ok(n) if n > 0 => {
            let resp = String::from_utf8_lossy(&buf[..n]);
            resp.contains("PONG")
        }
        _ => false,
    }
}

/// Check MCP server health
fn check_mcp_health(verbose: bool) -> ComponentHealth {
    let home = home::home_dir().unwrap_or_else(|| std::path::PathBuf::from("."));

    // Try multiple MCP binary locations
    let mcp_paths = vec![
        home.join(".masday").join("bin").join("masday-mcp"),
        std::path::PathBuf::from("/home/vibe-dev/masday-workflow-rust/target/release/masday-mcp"),
    ];

    let mcp_binary = mcp_paths.iter().find(|p| p.exists()).cloned();
    let mcp_exists = mcp_binary.is_some();

    if mcp_exists {
        // Count tools from MCP binary
        let tool_count = count_mcp_tools(mcp_binary.as_ref().unwrap());

        // Check if MCP is registered in Claude Code settings
        let registered = check_mcp_registration();

        let mut details = HashMap::new();
        if verbose {
            details.insert("binary_path".to_string(), mcp_binary.as_ref().unwrap().display().to_string());
            if let Some(count) = tool_count {
                details.insert("tool_count".to_string(), count.to_string());
            }
            details.insert("registered".to_string(), registered.to_string());
        }

        let message = match (tool_count, registered) {
            (Some(count), true) => format!("{} {} tools, registered", style("✓").green(), count),
            (Some(count), false) => format!("{} {} tools, not registered", style("⚠").yellow(), count),
            (None, true) => format!("{} registered", style("✓").green()),
            (None, false) => format!("{} binary found, not registered", style("⚠").yellow()),
        };

        let status = if registered { HealthStatus::Healthy } else { HealthStatus::Degraded };

        ComponentHealth {
            status,
            message,
            details: if verbose { Some(details) } else { None },
        }
    } else {
        let mut details = HashMap::new();
        if verbose {
            details.insert("expected_path".to_string(), mcp_paths[0].display().to_string());
            details.insert("hint".to_string(), "run 'masday install'".to_string());
        }

        ComponentHealth {
            status: HealthStatus::Degraded,
            message: format!("{} not installed", style("⚠").yellow()),
            details: if verbose { Some(details) } else { None },
        }
    }
}

/// Check if MCP server is registered in platform settings
fn check_mcp_registration() -> bool {
    let home = home::home_dir().unwrap_or_else(|| std::path::PathBuf::from("."));

    // Check ~/.claude.json (Claude Code MCP registration)
    let claude_json = home.join(".claude.json");
    if claude_json.exists() {
        if let Ok(content) = std::fs::read_to_string(&claude_json) {
            // Try structured parse first
            if let Ok(val) = serde_json::from_str::<serde_json::Value>(&content) {
                if val.get("mcpServers").and_then(|m| m.get("masday")).is_some() {
                    return true;
                }
            }
            // Fallback: string search
            if content.contains("\"masday\"") && content.contains("mcpServers") {
                return true;
            }
        }
    }

    // Check ~/.claude/settings.json (legacy)
    let claude_settings = home.join(".claude").join("settings.json");
    if claude_settings.exists() {
        if let Ok(content) = std::fs::read_to_string(&claude_settings) {
            if content.contains("masday-mcp") || content.contains("masday") {
                return true;
            }
        }
    }

    // Check project-level .claude/settings.json
    let project_settings = std::path::Path::new(".claude").join("settings.json");
    if project_settings.exists() {
        if let Ok(content) = std::fs::read_to_string(&project_settings) {
            if content.contains("masday-mcp") || content.contains("masday") {
                return true;
            }
        }
    }

    // Check ~/.gemini/ for Gemini CLI
    let gemini_settings = home.join(".gemini").join("settings.json");
    if gemini_settings.exists() {
        if let Ok(content) = std::fs::read_to_string(&gemini_settings) {
            if content.contains("masday") {
                return true;
            }
        }
    }

    false
}

/// Count MCP tools by parsing --help output
fn count_mcp_tools(mcp_binary: &std::path::Path) -> Option<usize> {
    let output = Command::new(mcp_binary)
        .arg("--help")
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let help_text = String::from_utf8(output.stdout).ok()?;
    // Count lines containing "Available MCP tools:" or similar
    // This is a fallback heuristic
    Some(help_text.lines().filter(|line| line.contains("tool")).count())
}

/// Check embedding service health
fn check_embedding_health(config: &MasdayConfig, verbose: bool) -> ComponentHealth {
    if let Some(ref provider) = config.embedding_provider {
        if !provider.is_empty() {
            let mut details = HashMap::new();
            details.insert("provider".to_string(), provider.clone());
            if let Some(ref model) = config.embedding_model {
                details.insert("model".to_string(), model.clone());
            }
            if let Some(dims) = config.embedding_dimensions {
                details.insert("dimensions".to_string(), dims.to_string());
            }
            if verbose {
                let home = home::home_dir().unwrap_or_else(|| std::path::PathBuf::from("."));
                details.insert(
                    "cache_dir".to_string(),
                    home.join(".masday").join("embed-cache").display().to_string(),
                );
            }

            ComponentHealth {
                status: HealthStatus::Healthy,
                message: format!("{} ready", style("✓").green()),
                details: if verbose { Some(details) } else { None },
            }
        } else {
            ComponentHealth {
                status: HealthStatus::NotConfigured,
                message: "— not configured".to_string(),
                details: None,
            }
        }
    } else {
        ComponentHealth {
            status: HealthStatus::NotConfigured,
            message: "— not configured".to_string(),
            details: None,
        }
    }
}

/// Count installed agents, skills, and hooks
fn count_assets() -> Result<(usize, usize, usize)> {
    let home = home::home_dir().unwrap_or_else(|| std::path::PathBuf::from("."));
    let masday_home = home.join(".masday");
    let claude_home = home.join(".claude");
    let gemini_home = home.join(".gemini");

    // Count agents from all platforms
    let mut agents_count = 0usize;
    let mut skills_count = 0usize;
    let mut hooks_count = 0usize;

    // ~/.masday/ (primary)
    count_dir_entries(&masday_home.join("agents"), &mut agents_count);
    count_dir_entries(&masday_home.join("skills"), &mut skills_count);
    count_dir_entries(&masday_home.join("hooks"), &mut hooks_count);

    // ~/.claude/ (Claude Code platform)
    count_dir_entries(&claude_home.join("agents"), &mut agents_count);
    count_dir_entries(&claude_home.join("skills"), &mut skills_count);
    count_dir_entries(&claude_home.join("hooks"), &mut hooks_count);

    // Deduplicate — prefer the larger count (most recent sync)
    let agents = agents_count.max(0);
    let skills = skills_count.max(0);
    let hooks = hooks_count.max(0);

    Ok((agents, skills, hooks))
}

fn count_dir_entries(dir: &std::path::Path, count: &mut usize) {
    if dir.exists() {
        if let Ok(entries) = std::fs::read_dir(dir) {
            *count = (*count).max(entries.filter_map(|e| e.ok()).count());
        }
    }
}

/// Detect actually installed platforms
fn detect_platforms() -> String {
    let home = home::home_dir().unwrap_or_else(|| std::path::PathBuf::from("."));
    let mut platforms = Vec::new();

    // Claude Code — ~/.claude/agents/ or ~/.claude/settings.json
    if home.join(".claude").join("agents").exists()
        || home.join(".claude").join("settings.json").exists()
        || home.join(".claude.json").exists()
    {
        platforms.push("claude-code");
    }

    // Gemini CLI — ~/.gemini/
    if home.join(".gemini").exists() {
        platforms.push("gemini");
    }

    // VS Code Copilot — .vscode/extensions
    if home.join(".vscode").join("extensions").exists() {
        platforms.push("vscode");
    }

    // OpenCode — ~/.opencode/
    if home.join(".opencode").exists() {
        platforms.push("opencode");
    }

    if platforms.is_empty() {
        "none".to_string()
    } else {
        platforms.join(", ")
    }
}

/// Output health report as JSON
fn output_json(report: &HealthReport) -> Result<()> {
    let json = serde_json::to_string_pretty(report)?;
    println!("{}", json);
    Ok(())
}

/// Table width constant
const TABLE_WIDTH: usize = 42;

/// Output health report as formatted table
fn output_table(report: &HealthReport, verbose: bool) -> Result<()> {
    format_header(report);
    format_section(report, verbose);
    format_footer(report);
    Ok(())
}

/// Format table header with version and basic info
fn format_header(report: &HealthReport) {
    let v = "│";

    println!("╭{}╮", "─".repeat(TABLE_WIDTH));
    println!("{}  Masday v{}{}{}", v, report.masday_version, " ".repeat(TABLE_WIDTH.saturating_sub(14 + report.masday_version.len())), v);
    println!("{}{}{}", v, " ".repeat(TABLE_WIDTH), v);
    println!("{}  Mode:       {}{}{}", v, report.mode, " ".repeat(TABLE_WIDTH.saturating_sub(15 + report.mode.len())), v);
    println!("{}  Platform:   {}{}{}", v, report.platform, " ".repeat(TABLE_WIDTH.saturating_sub(16 + report.platform.len())), v);

    // Config status
    let config_status = if report.config_valid {
        format!("{} ✓", style("valid").green())
    } else {
        format!("{} ✗", style("invalid").red())
    };
    let config_display = format!("~/.masday/config.toml {}", config_status);
    println!("{}  Config:     {}{}{}", v, config_display, " ".repeat(TABLE_WIDTH.saturating_sub(16 + config_display.chars().count())), v);
    println!("{}{}{}", v, " ".repeat(TABLE_WIDTH), v);
}

/// Format main section with component statuses
fn format_section(report: &HealthReport, verbose: bool) {
    let v = "│";

    // API status
    let api_msg = if let Some(ref details) = report.api.details {
        let url = details.get("url").map(|s| s.as_str()).unwrap_or("");
        format!("{} {}", report.api.message, url)
    } else {
        report.api.message.clone()
    };
    println!("{}  API:        {}{}{}", v, api_msg, " ".repeat(TABLE_WIDTH.saturating_sub(16 + api_msg.chars().count())), v);

    // Database status
    let default_url = "N/A".to_string();
    let db_msg = if report.database.details.as_ref().and_then(|d| d.get("url")).is_some() {
        let url = report.database.details.as_ref().and_then(|d| d.get("url")).unwrap_or(&default_url);
        format!("{} {}", report.database.message, url)
    } else if report.database.details.as_ref().and_then(|d| d.get("port")).is_some() {
        let default_port = "N/A".to_string();
        let port = report.database.details.as_ref().and_then(|d| d.get("port")).unwrap_or(&default_port);
        format!("{} (Docker, port {})", report.database.message, port)
    } else {
        report.database.message.clone()
    };
    println!("{}  Database:   {}{}{}", v, db_msg, " ".repeat(TABLE_WIDTH.saturating_sub(16 + db_msg.chars().count())), v);

    // Redis status
    let default_port = "N/A".to_string();
    let redis_msg = if report.redis.details.as_ref().and_then(|d| d.get("port")).is_some() {
        let port = report.redis.details.as_ref().and_then(|d| d.get("port")).unwrap_or(&default_port);
        format!("{} (Docker, port {})", report.redis.message, port)
    } else {
        report.redis.message.clone()
    };
    println!("{}  Redis:      {}{}{}", v, redis_msg, " ".repeat(TABLE_WIDTH.saturating_sub(16 + redis_msg.chars().count())), v);

    // MCP status (dynamic tool count already in message)
    let mcp_msg = report.mcp.message.clone();
    println!("{}  MCP:        {}{}{}", v, mcp_msg, " ".repeat(TABLE_WIDTH.saturating_sub(16 + mcp_msg.chars().count())), v);
    println!("{}{}{}", v, " ".repeat(TABLE_WIDTH), v);

    // Embedding status
    let default_provider = "N/A".to_string();
    let embed_msg = if report.embedding.details.as_ref().and_then(|d| d.get("provider")).is_some() {
        let provider = report.embedding.details.as_ref().and_then(|d| d.get("provider")).unwrap_or(&default_provider);
        format!("{} ({})", report.embedding.message, provider)
    } else {
        report.embedding.message.clone()
    };
    println!("{}  Embedding:  {}{}{}", v, embed_msg, " ".repeat(TABLE_WIDTH.saturating_sub(16 + embed_msg.chars().count())), v);

    if verbose {
        if let Some(ref details) = report.embedding.details {
            if let Some(model) = details.get("model") {
                println!("{}  Model:      {}{}{}", v, model, " ".repeat(TABLE_WIDTH.saturating_sub(16 + model.len())), v);
            }
            if let Some(cache) = details.get("cache_dir") {
                let cache_short = cache.replace("/home/", "~/").replace("/Users/", "~/");
                println!("{}  Cache:      {}{}{}", v, cache_short, " ".repeat(TABLE_WIDTH.saturating_sub(16 + cache_short.len())), v);
            }
        }
    }
    println!("{}{}{}", v, " ".repeat(TABLE_WIDTH), v);
}

/// Format footer with asset counts
fn format_footer(report: &HealthReport) {
    let v = "│";

    // Asset counts
    println!("{}  Agents:     {} synced{}{}", v, report.agents_count, " ".repeat(TABLE_WIDTH.saturating_sub(16 + format!("{} synced", report.agents_count).len())), v);
    println!("{}  Skills:     {} synced{}{}", v, report.skills_count, " ".repeat(TABLE_WIDTH.saturating_sub(16 + format!("{} synced", report.skills_count).len())), v);
    println!("{}  Hooks:      {} installed{}{}", v, report.hooks_count, " ".repeat(TABLE_WIDTH.saturating_sub(16 + format!("{} installed", report.hooks_count).len())), v);

    // Bottom border
    println!("╰{}╯", "─".repeat(TABLE_WIDTH));
}

/// Redact password from a database URL for safe display.
/// `postgresql://user:secret@host/db` → `postgresql://user:***@host/db`
fn redact_db_url(url: &str) -> String {
    if let Some(scheme_end) = url.find("://") {
        let prefix = &url[..scheme_end]; // "postgresql"
        let rest = &url[scheme_end + 3..]; // "user:pass@host/db"
        if let Some(at_pos) = rest.find('@') {
            let user_part = &rest[..at_pos]; // "user:pass"
            let host_part = &rest[at_pos..]; // "@host/db"
            if let Some(colon_pos) = user_part.find(':') {
                let user = &user_part[..colon_pos];
                return format!("{}://{}:***{}", prefix, user, host_part);
            }
        }
    }
    url.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_health_report_overall_status_healthy() {
        let report = HealthReport {
            masday_version: "0.1.0".to_string(),
            mode: "local".to_string(),
            platform: "linux".to_string(),
            config_path: "/tmp/config.toml".to_string(),
            config_valid: true,
            api: ComponentHealth {
                status: HealthStatus::Healthy,
                message: "✓ healthy".to_string(),
                details: None,
            },
            database: ComponentHealth {
                status: HealthStatus::Healthy,
                message: "✓ connected".to_string(),
                details: None,
            },
            redis: ComponentHealth {
                status: HealthStatus::Healthy,
                message: "✓ running".to_string(),
                details: None,
            },
            mcp: ComponentHealth {
                status: HealthStatus::Healthy,
                message: "✓ registered".to_string(),
                details: None,
            },
            embedding: ComponentHealth {
                status: HealthStatus::Healthy,
                message: "✓ ready".to_string(),
                details: None,
            },
            agents_count: 5,
            skills_count: 10,
            hooks_count: 3,
        };

        let (exit_code, status_str) = report.overall_status();
        assert_eq!(exit_code, 0);
        assert_eq!(status_str, "healthy");
    }

    #[test]
    fn test_health_report_overall_status_degraded() {
        let report = HealthReport {
            masday_version: "0.1.0".to_string(),
            mode: "local".to_string(),
            platform: "linux".to_string(),
            config_path: "/tmp/config.toml".to_string(),
            config_valid: true,
            api: ComponentHealth {
                status: HealthStatus::Healthy,
                message: "✓ healthy".to_string(),
                details: None,
            },
            database: ComponentHealth {
                status: HealthStatus::Healthy,
                message: "✓ connected".to_string(),
                details: None,
            },
            redis: ComponentHealth {
                status: HealthStatus::Degraded, // Redis degraded
                message: "⚠ not running".to_string(),
                details: None,
            },
            mcp: ComponentHealth {
                status: HealthStatus::Healthy,
                message: "✓ registered".to_string(),
                details: None,
            },
            embedding: ComponentHealth {
                status: HealthStatus::Healthy,
                message: "✓ ready".to_string(),
                details: None,
            },
            agents_count: 5,
            skills_count: 10,
            hooks_count: 3,
        };

        let (exit_code, status_str) = report.overall_status();
        assert_eq!(exit_code, 1);
        assert_eq!(status_str, "degraded");
    }

    #[test]
    fn test_health_report_overall_status_critical() {
        let report = HealthReport {
            masday_version: "0.1.0".to_string(),
            mode: "local".to_string(),
            platform: "linux".to_string(),
            config_path: "/tmp/config.toml".to_string(),
            config_valid: true,
            api: ComponentHealth {
                status: HealthStatus::Unhealthy, // API down
                message: "✗ not running".to_string(),
                details: None,
            },
            database: ComponentHealth {
                status: HealthStatus::Healthy,
                message: "✓ connected".to_string(),
                details: None,
            },
            redis: ComponentHealth {
                status: HealthStatus::Healthy,
                message: "✓ running".to_string(),
                details: None,
            },
            mcp: ComponentHealth {
                status: HealthStatus::Healthy,
                message: "✓ registered".to_string(),
                details: None,
            },
            embedding: ComponentHealth {
                status: HealthStatus::Healthy,
                message: "✓ ready".to_string(),
                details: None,
            },
            agents_count: 5,
            skills_count: 10,
            hooks_count: 3,
        };

        let (exit_code, status_str) = report.overall_status();
        assert_eq!(exit_code, 2);
        assert_eq!(status_str, "critical");
    }

    #[test]
    fn test_redact_db_url() {
        assert_eq!(
            redact_db_url("postgresql://user:secret@localhost/db"),
            "postgresql://user:***@localhost/db"
        );
        assert_eq!(
            redact_db_url("postgresql://user@localhost/db"),
            "postgresql://user@localhost/db"
        );
        assert_eq!(
            redact_db_url("invalid-url"),
            "invalid-url"
        );
    }

    #[test]
    fn test_output_json_valid() {
        let report = HealthReport {
            masday_version: "0.1.0".to_string(),
            mode: "local".to_string(),
            platform: "linux".to_string(),
            config_path: "/tmp/config.toml".to_string(),
            config_valid: true,
            api: ComponentHealth {
                status: HealthStatus::Healthy,
                message: "✓ healthy".to_string(),
                details: None,
            },
            database: ComponentHealth {
                status: HealthStatus::Healthy,
                message: "✓ connected".to_string(),
                details: None,
            },
            redis: ComponentHealth {
                status: HealthStatus::Healthy,
                message: "✓ running".to_string(),
                details: None,
            },
            mcp: ComponentHealth {
                status: HealthStatus::Healthy,
                message: "✓ registered".to_string(),
                details: None,
            },
            embedding: ComponentHealth {
                status: HealthStatus::Healthy,
                message: "✓ ready".to_string(),
                details: None,
            },
            agents_count: 5,
            skills_count: 10,
            hooks_count: 3,
        };

        let result = output_json(&report);
        assert!(result.is_ok());
    }

    #[test]
    fn test_output_table_formatting() {
        let report = HealthReport {
            masday_version: "0.1.0".to_string(),
            mode: "local".to_string(),
            platform: "linux".to_string(),
            config_path: "/tmp/config.toml".to_string(),
            config_valid: true,
            api: ComponentHealth {
                status: HealthStatus::Healthy,
                message: "✓ healthy".to_string(),
                details: None,
            },
            database: ComponentHealth {
                status: HealthStatus::Healthy,
                message: "✓ connected".to_string(),
                details: None,
            },
            redis: ComponentHealth {
                status: HealthStatus::Healthy,
                message: "✓ running".to_string(),
                details: None,
            },
            mcp: ComponentHealth {
                status: HealthStatus::Healthy,
                message: "✓ 91 tools".to_string(),
                details: None,
            },
            embedding: ComponentHealth {
                status: HealthStatus::Healthy,
                message: "✓ ready".to_string(),
                details: None,
            },
            agents_count: 5,
            skills_count: 10,
            hooks_count: 3,
        };

        let result = output_table(&report, false);
        assert!(result.is_ok());
    }
}
