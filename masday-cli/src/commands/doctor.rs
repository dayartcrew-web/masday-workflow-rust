//! Doctor command — comprehensive health diagnostics for Masday installation.
//!
//! Checks binary, config, connectivity, database, MCP, embedding, platforms,
//! and provides actionable feedback for troubleshooting.

use anyhow::Result;
use console::style;
use serde::Serialize;
use std::path::Path;

use crate::build_origin;
use crate::config::MasdayConfig;

/// Individual check result
#[derive(Debug, Serialize)]
pub struct CheckResult {
    pub name: String,
    pub status: CheckStatus,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CheckStatus {
    Ok,
    Warn,
    Fail,
    Skip,
}

/// Full doctor report
#[derive(Debug, Serialize)]
pub struct DoctorReport {
    pub version: String,
    pub build_origin: &'static str,
    pub binary_path: String,
    pub checks: Vec<CheckResult>,
}

impl DoctorReport {
    /// Count checks by status
    pub fn count(&self, status: CheckStatus) -> usize {
        self.checks.iter().filter(|c| c.status == status).count()
    }

    /// Overall health status
    pub fn overall(&self) -> &'static str {
        if self.count(CheckStatus::Fail) > 0 {
            "UNHEALTHY"
        } else if self.count(CheckStatus::Warn) > 0 {
            "DEGRADED"
        } else {
            "HEALTHY"
        }
    }
}

/// Run the doctor command
pub async fn run(json: bool, fix: bool) -> Result<()> {
    let mut report = DoctorReport {
        version: env!("CARGO_PKG_VERSION").to_string(),
        build_origin: build_origin::BUILD_ORIGIN,
        binary_path: std::env::current_exe()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|_| "unknown".to_string()),
        checks: Vec::new(),
    };

    // Run all checks
    check_binary(&mut report);
    check_config(&mut report);
    check_masday_home(&mut report);
    check_path(&mut report);
    check_postgres_container(&mut report);
    check_postgres_connectivity(&mut report).await;
    check_credential_consistency(&mut report);
    check_api_connectivity(&mut report).await;
    check_mcp_binary(&mut report);
    check_embedding(&mut report);
    check_platforms(&mut report);
    check_disk_space(&mut report);
    check_update_available(&mut report).await;
    check_sqlite_database(&mut report);
    check_redis_connectivity(&mut report).await;
    check_stale_workflows(&mut report).await;
    check_mcp_config(&mut report);

    // Apply fixes if requested
    if fix {
        let fixes_applied = apply_fixes(&report).await?;
        if !fixes_applied.is_empty() {
            println!();
            println!("{}", style("Fixes applied:").cyan().bold());
            for fix in &fixes_applied {
                println!("  {}", style(fix).green());
            }
        }
    }

    if json {
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        print_report(&report);
    }

    // Exit with error code if unhealthy
    if report.count(CheckStatus::Fail) > 0 {
        std::process::exit(1);
    }

    Ok(())
}

// ── Individual checks ────────────────────────────────────────────────────────

fn check_binary(report: &mut DoctorReport) {
    report.checks.push(CheckResult {
        name: "Binary".into(),
        status: CheckStatus::Ok,
        message: format!("v{} ({})", report.version, report.build_origin),
        detail: Some(report.binary_path.clone()),
    });
}

fn check_config(report: &mut DoctorReport) {
    let config_path = MasdayConfig::config_path();

    if !config_path.exists() {
        report.checks.push(CheckResult {
            name: "Config".into(),
            status: CheckStatus::Warn,
            message: "No config file found".into(),
            detail: Some(format!("Expected: {}", config_path.display())),
        });
        return;
    }

    match MasdayConfig::load_or_err() {
        Ok(config) => {
            let mode = &config.mode;
            report.checks.push(CheckResult {
                name: "Config".into(),
                status: CheckStatus::Ok,
                message: format!("Valid (mode: {})", mode),
                detail: Some(config_path.display().to_string()),
            });
        }
        Err(e) => {
            report.checks.push(CheckResult {
                name: "Config".into(),
                status: CheckStatus::Fail,
                message: format!("Invalid config: {}", e),
                detail: Some(config_path.display().to_string()),
            });
        }
    }
}

fn check_masday_home(report: &mut DoctorReport) {
    let home = MasdayConfig::masday_home();
    if home.exists() {
        report.checks.push(CheckResult {
            name: "Home Dir".into(),
            status: CheckStatus::Ok,
            message: format!("Exists at {}", home.display()),
            detail: None,
        });
    } else {
        report.checks.push(CheckResult {
            name: "Home Dir".into(),
            status: CheckStatus::Fail,
            message: format!("~/.masday/ not found at {}", home.display()),
            detail: Some("Run 'masday quickstart' to create it".into()),
        });
    }
}

fn check_path(report: &mut DoctorReport) {
    let home = MasdayConfig::masday_home();
    let bin_dir = home.join("bin");
    let path_var = std::env::var("PATH").unwrap_or_default();
    let path_sep = if cfg!(windows) { ';' } else { ':' };
    let in_path = path_var.split(path_sep).any(|p| Path::new(p) == bin_dir);

    report.checks.push(CheckResult {
        name: "PATH".into(),
        status: if in_path {
            CheckStatus::Ok
        } else {
            CheckStatus::Warn
        },
        message: if in_path {
            "~/.masday/bin in PATH".into()
        } else {
            "~/.masday/bin NOT in PATH".into()
        },
        detail: if in_path {
            None
        } else {
            Some("Add: export PATH=\"$PATH:$HOME/.masday/bin\"".into())
        },
    });
}

fn check_postgres_container(report: &mut DoctorReport) {
    let config = match MasdayConfig::load_or_err() {
        Ok(c) if c.mode == "local" => c,
        _ => {
            report.checks.push(CheckResult {
                name: "PostgreSQL".into(),
                status: CheckStatus::Skip,
                message: "Not in local mode".into(),
                detail: None,
            });
            return;
        }
    };

    // If database_url is explicitly set, skip Docker check
    if config.database_url.is_some() {
        report.checks.push(CheckResult {
            name: "PostgreSQL".into(),
            status: CheckStatus::Ok,
            message: "Custom database_url configured".into(),
            detail: config.database_url.as_ref().map(|u| redact_url(u)),
        });
        return;
    }

    if !crate::docker::is_docker_available() {
        report.checks.push(CheckResult {
            name: "PostgreSQL (Docker)".into(),
            status: CheckStatus::Warn,
            message: "Docker not available".into(),
            detail: Some("Install Docker to use managed PostgreSQL".into()),
        });
        return;
    }

    if crate::docker::is_container_running("masday-postgres") {
        report.checks.push(CheckResult {
            name: "PostgreSQL (Docker)".into(),
            status: CheckStatus::Ok,
            message: "Container running".into(),
            detail: None,
        });
    } else {
        report.checks.push(CheckResult {
            name: "PostgreSQL (Docker)".into(),
            status: CheckStatus::Fail,
            message: "Container not running".into(),
            detail: Some("Run 'masday db start'".into()),
        });
    }
}

async fn check_postgres_connectivity(report: &mut DoctorReport) {
    let config = match MasdayConfig::load_or_err() {
        Ok(c) if c.mode == "local" => c,
        _ => return, // Already handled in check_postgres_container
    };

    let db_url = config
        .database_url
        .clone()
        .unwrap_or_else(crate::docker::default_database_url);

    std::env::set_var("DATABASE_URL", &db_url);

    match masday_db::pool::init_pool_with_retry(2).await {
        Ok(pool) => match masday_db::pool::health_check(&pool).await {
            Ok(_) => {
                let table_count = count_tables(&pool).await;
                report.checks.push(CheckResult {
                    name: "PostgreSQL (Connectivity)".into(),
                    status: CheckStatus::Ok,
                    message: format!("Connected ({} tables)", table_count),
                    detail: None,
                });

                if table_count == 0 {
                    report.checks.push(CheckResult {
                        name: "PostgreSQL (Migrations)".into(),
                        status: CheckStatus::Warn,
                        message: "Database has 0 tables — migrations may not have run".into(),
                        detail: Some("Run 'masday db reset' or 'masday quickstart'".into()),
                    });
                }
            }
            Err(e) => {
                report.checks.push(CheckResult {
                    name: "PostgreSQL (Connectivity)".into(),
                    status: CheckStatus::Fail,
                    message: format!("Health check failed: {}", e),
                    detail: Some(format!("URL: {}", redact_url(&db_url))),
                });
            }
        },
        Err(e) => {
            report.checks.push(CheckResult {
                name: "PostgreSQL (Connectivity)".into(),
                status: CheckStatus::Fail,
                message: format!("Cannot connect: {}", e),
                detail: Some(format!(
                    "URL: {} — check credentials in config.toml",
                    redact_url(&db_url)
                )),
            });
        }
    }
}

fn check_credential_consistency(report: &mut DoctorReport) {
    let config = match MasdayConfig::load_or_err() {
        Ok(c) if c.mode == "local" => c,
        _ => return,
    };

    if let Some(ref db_url) = config.database_url {
        // URL should contain user:password@host pattern
        if let Some(scheme_end) = db_url.find("://") {
            let rest = &db_url[scheme_end + 3..];
            if !rest.contains('@') {
                report.checks.push(CheckResult {
                    name: "PostgreSQL (Credentials)".into(),
                    status: CheckStatus::Warn,
                    message: "database_url may be missing credentials".into(),
                    detail: Some("Expected: postgresql://user:password@host:port/dbname".into()),
                });
            }
        }
    }
}

/// Count tables in the public schema
async fn count_tables(pool: &masday_db::pool::DbPool) -> usize {
    let client = match pool.get().await {
        Ok(c) => c,
        Err(_) => return 0,
    };
    match client
        .query_one(
            "SELECT count(*) FROM information_schema.tables WHERE table_schema = 'public'",
            &[],
        )
        .await
    {
        Ok(row) => row.get::<_, i64>(0) as usize,
        Err(_) => 0,
    }
}

/// Redact password from URL for safe display
fn redact_url(url: &str) -> String {
    if let Some(scheme_end) = url.find("://") {
        let prefix = &url[..scheme_end];
        let rest = &url[scheme_end + 3..];
        if let Some(at_pos) = rest.find('@') {
            let user_part = &rest[..at_pos];
            let host_part = &rest[at_pos..];
            if let Some(colon_pos) = user_part.find(':') {
                let user = &user_part[..colon_pos];
                return format!("{}://{}:***{}", prefix, user, host_part);
            }
        }
    }
    url.to_string()
}

async fn check_api_connectivity(report: &mut DoctorReport) {
    let config = match MasdayConfig::load_or_err() {
        Ok(c) => c,
        Err(_) => {
            report.checks.push(CheckResult {
                name: "API".into(),
                status: CheckStatus::Skip,
                message: "No config to check".into(),
                detail: None,
            });
            return;
        }
    };

    let health_url = format!("{}/api/health", config.api_url.trim_end_matches('/'));

    match reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()
    {
        Ok(client) => match client.get(&health_url).send().await {
            Ok(resp) if resp.status().is_success() => {
                report.checks.push(CheckResult {
                    name: "API".into(),
                    status: CheckStatus::Ok,
                    message: format!("Connected ({})", config.api_url),
                    detail: None,
                });
            }
            Ok(resp) => {
                report.checks.push(CheckResult {
                    name: "API".into(),
                    status: CheckStatus::Warn,
                    message: format!("HTTP {}", resp.status()),
                    detail: Some(config.api_url.clone()),
                });
            }
            Err(e) => {
                report.checks.push(CheckResult {
                    name: "API".into(),
                    status: CheckStatus::Fail,
                    message: format!("Connection failed: {}", e),
                    detail: Some(config.api_url.clone()),
                });
            }
        },
        Err(e) => {
            report.checks.push(CheckResult {
                name: "API".into(),
                status: CheckStatus::Fail,
                message: format!("HTTP client error: {}", e),
                detail: None,
            });
        }
    }
}

fn check_mcp_binary(report: &mut DoctorReport) {
    let binary_name = if cfg!(windows) {
        "masday.exe"
    } else {
        "masday"
    };
    let home = MasdayConfig::masday_home();
    let bin_path = home.join("bin").join(binary_name);

    // Check current exe first, then ~/.masday/bin/
    let current = std::env::current_exe().ok();

    if let Some(ref exe) = current {
        if exe.exists() {
            report.checks.push(CheckResult {
                name: "MCP Binary".into(),
                status: CheckStatus::Ok,
                message: "Found".into(),
                detail: Some(exe.display().to_string()),
            });
            return;
        }
    }

    if bin_path.exists() {
        report.checks.push(CheckResult {
            name: "MCP Binary".into(),
            status: CheckStatus::Ok,
            message: "Found".into(),
            detail: Some(bin_path.display().to_string()),
        });
    } else {
        report.checks.push(CheckResult {
            name: "MCP Binary".into(),
            status: CheckStatus::Fail,
            message: "Not found".into(),
            detail: Some(format!("Checked: {} and current exe", bin_path.display())),
        });
    }
}

fn check_embedding(report: &mut DoctorReport) {
    let config = match MasdayConfig::load_or_err() {
        Ok(c) => c,
        Err(_) => {
            report.checks.push(CheckResult {
                name: "Embedding".into(),
                status: CheckStatus::Skip,
                message: "No config to check".into(),
                detail: None,
            });
            return;
        }
    };

    match config.embedding_provider.as_deref() {
        Some("local") | Some("ollama") | Some("openai") => {
            report.checks.push(CheckResult {
                name: "Embedding".into(),
                status: CheckStatus::Ok,
                message: format!(
                    "Provider: {} ({})",
                    config.embedding_provider.as_deref().unwrap_or("unknown"),
                    config.embedding_model.as_deref().unwrap_or("unknown")
                ),
                detail: None,
            });
        }
        _ => {
            report.checks.push(CheckResult {
                name: "Embedding".into(),
                status: CheckStatus::Warn,
                message: "Not configured".into(),
                detail: Some("Run 'masday embed setup' to configure".into()),
            });
        }
    }
}

fn check_platforms(report: &mut DoctorReport) {
    let config = match MasdayConfig::load_or_err() {
        Ok(c) => c,
        Err(_) => {
            report.checks.push(CheckResult {
                name: "Platforms".into(),
                status: CheckStatus::Skip,
                message: "No config to check".into(),
                detail: None,
            });
            return;
        }
    };

    if config.platforms.is_empty() {
        report.checks.push(CheckResult {
            name: "Platforms".into(),
            status: CheckStatus::Warn,
            message: "No platforms configured".into(),
            detail: Some("Run 'masday install' to configure".into()),
        });
    } else {
        report.checks.push(CheckResult {
            name: "Platforms".into(),
            status: CheckStatus::Ok,
            message: config.platforms.join(", "),
            detail: None,
        });
    }

    // Check Claude Code MCP registration specifically
    if let Some(home) = home::home_dir() {
        let claude_mcp = home.join(".claude.json");
        let has_mcp = claude_mcp.exists()
            && std::fs::read_to_string(&claude_mcp)
                .map(|s| s.contains("masday"))
                .unwrap_or(false);

        report.checks.push(CheckResult {
            name: "Claude MCP".into(),
            status: if has_mcp {
                CheckStatus::Ok
            } else {
                CheckStatus::Warn
            },
            message: if has_mcp {
                "Registered in .claude.json".into()
            } else {
                "Not registered in .claude.json".into()
            },
            detail: None,
        });
    }
}

fn check_disk_space(report: &mut DoctorReport) {
    let home = MasdayConfig::masday_home();
    if home.exists() {
        // Get directory size (best effort)
        let size: u64 = std::fs::read_dir(&home)
            .map(|entries| {
                entries
                    .filter_map(|e| e.ok())
                    .filter_map(|e| e.metadata().ok().map(|m| m.len()))
                    .sum()
            })
            .unwrap_or(0);

        let size_mb = size as f64 / 1024.0 / 1024.0;
        report.checks.push(CheckResult {
            name: "Disk Usage".into(),
            status: CheckStatus::Ok,
            message: format!("~{:.1} MB in ~/.masday/", size_mb),
            detail: None,
        });
    }
}

async fn check_update_available(report: &mut DoctorReport) {
    // Use async version to avoid nested runtime panic
    match crate::commands::update::fetch_latest_version_async().await {
        Ok(latest) => {
            let current = env!("CARGO_PKG_VERSION");
            let current_clean = current.trim_start_matches('v');
            let latest_clean = latest.trim_start_matches('v');

            if current_clean == latest_clean {
                report.checks.push(CheckResult {
                    name: "Updates".into(),
                    status: CheckStatus::Ok,
                    message: format!("Up to date (v{})", current_clean),
                    detail: None,
                });
            } else {
                report.checks.push(CheckResult {
                    name: "Updates".into(),
                    status: CheckStatus::Warn,
                    message: format!("v{} available (current: v{})", latest_clean, current_clean),
                    detail: Some("Run 'masday update' to upgrade".into()),
                });
            }
        }
        Err(_) => {
            report.checks.push(CheckResult {
                name: "Updates".into(),
                status: CheckStatus::Skip,
                message: "Could not check for updates".into(),
                detail: None,
            });
        }
    }
}

// ── Display ──────────────────────────────────────────────────────────────────

fn print_report(report: &DoctorReport) {
    println!();
    println!(
        "{}",
        style("╔══════════════════════════════════════════╗").cyan()
    );
    println!(
        "{}",
        style("║        ⚕️  Masday Doctor Report          ║").cyan()
    );
    println!(
        "{}",
        style("╚══════════════════════════════════════════╝").cyan()
    );
    println!();
    println!(
        "  Version: {} ({})",
        style(&report.version).green(),
        style(report.build_origin).dim()
    );
    println!("  Binary:  {}", style(&report.binary_path).dim());
    println!();

    println!("{}", style("  Checks:").cyan().bold());
    println!(
        "{}",
        style("  ───────────────────────────────────────").dim()
    );

    for check in &report.checks {
        let icon = match check.status {
            CheckStatus::Ok => style("✓").green(),
            CheckStatus::Warn => style("⚠").yellow(),
            CheckStatus::Fail => style("✗").red(),
            CheckStatus::Skip => style("—").dim(),
        };

        println!(
            "  {} {}: {}",
            icon,
            style(&check.name).bold(),
            check.message
        );

        if let Some(ref detail) = check.detail {
            println!("    {}", style(detail).dim());
        }
    }

    println!();
    println!(
        "{}",
        style("  ───────────────────────────────────────").dim()
    );

    let overall = report.overall();
    let overall_style = match overall {
        "HEALTHY" => style(overall).green().bold(),
        "DEGRADED" => style(overall).yellow().bold(),
        _ => style(overall).red().bold(),
    };

    println!("  Overall: {}", overall_style);

    let ok = report.count(CheckStatus::Ok);
    let warn = report.count(CheckStatus::Warn);
    let fail = report.count(CheckStatus::Fail);
    let skip = report.count(CheckStatus::Skip);

    println!(
        "  {} ok, {} warnings, {} failures, {} skipped",
        style(ok).green(),
        style(warn).yellow(),
        style(fail).red(),
        style(skip).dim()
    );
    println!();
}

// ── Additional checks ───────────────────────────────────────────────────────────

fn check_sqlite_database(report: &mut DoctorReport) {
    let config = match MasdayConfig::load_or_err() {
        Ok(c) => c,
        Err(_) => {
            report.checks.push(CheckResult {
                name: "SQLite Database".into(),
                status: CheckStatus::Skip,
                message: "No config to check".into(),
                detail: None,
            });
            return;
        }
    };

    if config.mode != "local" && config.mode != "standalone" {
        report.checks.push(CheckResult {
            name: "SQLite Database".into(),
            status: CheckStatus::Skip,
            message: "Not in local/standalone mode".into(),
            detail: None,
        });
        return;
    }

    let home = MasdayConfig::masday_home();
    let db_path = home.join("data.db");

    if !db_path.exists() {
        report.checks.push(CheckResult {
            name: "SQLite Database".into(),
            status: CheckStatus::Fail,
            message: "Database file not found".into(),
            detail: Some(format!("Expected: {}", db_path.display())),
        });
        return;
    }

    match rusqlite::Connection::open(&db_path) {
        Ok(conn) => {
            let table_count: Result<usize, _> = conn
                .query_row(
                    "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%'",
                    [],
                    |row| row.get(0),
                );

            match table_count {
                Ok(count) => {
                    if count >= 16 {
                        report.checks.push(CheckResult {
                            name: "SQLite Database".into(),
                            status: CheckStatus::Ok,
                            message: format!("OK ({} tables)", count),
                            detail: Some(db_path.display().to_string()),
                        });
                    } else {
                        report.checks.push(CheckResult {
                            name: "SQLite Database".into(),
                            status: CheckStatus::Warn,
                            message: format!("Incomplete schema ({} tables, expected 16+)", count),
                            detail: Some(db_path.display().to_string()),
                        });
                    }
                }
                Err(e) => {
                    report.checks.push(CheckResult {
                        name: "SQLite Database".into(),
                        status: CheckStatus::Fail,
                        message: format!("Query failed: {}", e),
                        detail: Some(db_path.display().to_string()),
                    });
                }
            }
        }
        Err(e) => {
            report.checks.push(CheckResult {
                name: "SQLite Database".into(),
                status: CheckStatus::Fail,
                message: format!("Cannot open: {}", e),
                detail: Some(db_path.display().to_string()),
            });
        }
    }
}

async fn check_redis_connectivity(report: &mut DoctorReport) {
    let config = match MasdayConfig::load_or_err() {
        Ok(c) => c,
        Err(_) => {
            report.checks.push(CheckResult {
                name: "Redis".into(),
                status: CheckStatus::Skip,
                message: "No config to check".into(),
                detail: None,
            });
            return;
        }
    };

    if config.mode != "local" {
        report.checks.push(CheckResult {
            name: "Redis".into(),
            status: CheckStatus::Skip,
            message: "Not in local mode".into(),
            detail: None,
        });
        return;
    }

    if let Some(ref redis_url) = config.redis_url {
        match redis::Client::open(redis_url.as_str()) {
            Ok(client) => {
                match client.get_multiplexed_async_connection().await {
                    Ok(mut conn) => {
                        match redis::cmd("PING").query_async::<String>(&mut conn).await {
                            Ok(_) => {
                                report.checks.push(CheckResult {
                                    name: "Redis".into(),
                                    status: CheckStatus::Ok,
                                    message: "Connected".into(),
                                    detail: None,
                                });
                            }
                            Err(e) => {
                                report.checks.push(CheckResult {
                                    name: "Redis".into(),
                                    status: CheckStatus::Fail,
                                    message: format!("Ping failed: {}", e),
                                    detail: Some(redis_url.clone()),
                                });
                            }
                        }
                    }
                    Err(e) => {
                        report.checks.push(CheckResult {
                            name: "Redis".into(),
                            status: CheckStatus::Fail,
                            message: format!("Cannot connect: {}", e),
                            detail: Some(redis_url.clone()),
                        });
                    }
                }
            }
            Err(e) => {
                report.checks.push(CheckResult {
                    name: "Redis".into(),
                    status: CheckStatus::Fail,
                    message: format!("Invalid URL: {}", e),
                    detail: Some(redis_url.clone()),
                });
            }
        }
    } else {
        let port = 63791;
        if is_port_open(port) {
            report.checks.push(CheckResult {
                name: "Redis".into(),
                status: CheckStatus::Ok,
                message: format!("Port {} open", port),
                detail: None,
            });
        } else {
            report.checks.push(CheckResult {
                name: "Redis".into(),
                status: CheckStatus::Skip,
                message: "Port not open (Redis not configured)".into(),
                detail: None,
            });
        }
    }
}

async fn check_stale_workflows(report: &mut DoctorReport) {
    let config = match MasdayConfig::load_or_err() {
        Ok(c) => c,
        Err(_) => {
            report.checks.push(CheckResult {
                name: "Stale Workflows".into(),
                status: CheckStatus::Skip,
                message: "No config to check".into(),
                detail: None,
            });
            return;
        }
    };

    if config.mode != "local" && config.mode != "standalone" {
        report.checks.push(CheckResult {
            name: "Stale Workflows".into(),
            status: CheckStatus::Skip,
            message: "Not in local/standalone mode".into(),
            detail: None,
        });
        return;
    }

    let home = MasdayConfig::masday_home();
    let db_path = home.join("data.db");

    if !db_path.exists() {
        report.checks.push(CheckResult {
            name: "Stale Workflows".into(),
            status: CheckStatus::Skip,
            message: "Database not found".into(),
            detail: None,
        });
        return;
    }

    match rusqlite::Connection::open(&db_path) {
        Ok(conn) => {
            let stale_count: Result<i64, _> = conn
                .query_row(
                    "SELECT COUNT(*) FROM workflows
                     WHERE status IN ('RUNNING', 'EXECUTE')
                     AND datetime(updated_at) < datetime('now', '-30 minutes')",
                    [],
                    |row| row.get(0),
                );

            match stale_count {
                Ok(0) => {
                    report.checks.push(CheckResult {
                        name: "Stale Workflows".into(),
                        status: CheckStatus::Ok,
                        message: "No stuck workflows".into(),
                        detail: None,
                    });
                }
                Ok(count) => {
                    report.checks.push(CheckResult {
                        name: "Stale Workflows".into(),
                        status: CheckStatus::Warn,
                        message: format!("{} workflows stuck", count),
                        detail: Some("Use --fix to reset them to PAUSED".into()),
                    });
                }
                Err(_) => {
                    report.checks.push(CheckResult {
                        name: "Stale Workflows".into(),
                        status: CheckStatus::Skip,
                        message: "Could not check".into(),
                        detail: None,
                    });
                }
            }
        }
        Err(_) => {
            report.checks.push(CheckResult {
                name: "Stale Workflows".into(),
                status: CheckStatus::Skip,
                message: "Cannot open database".into(),
                detail: None,
            });
        }
    }
}

fn check_mcp_config(report: &mut DoctorReport) {
    let home = match home::home_dir() {
        Some(h) => h,
        None => {
            report.checks.push(CheckResult {
                name: "MCP Config".into(),
                status: CheckStatus::Skip,
                message: "Cannot determine home directory".into(),
                detail: None,
            });
            return;
        }
    };

    let global_mcp = home.join(".claude").join(".mcp.json");
    let project_mcp = std::env::current_dir()
        .ok()
        .map(|p| p.join(".mcp.json"));

    let (config_path, is_global) = if project_mcp.as_ref().map(|p| p.exists()).unwrap_or(false) {
        (project_mcp.unwrap(), false)
    } else if global_mcp.exists() {
        (global_mcp, true)
    } else {
        report.checks.push(CheckResult {
            name: "MCP Config".into(),
            status: CheckStatus::Warn,
            message: "No MCP config found".into(),
            detail: Some("Expected .mcp.json or ~/.claude/.mcp.json".into()),
        });
        return;
    };

    match std::fs::read_to_string(&config_path) {
        Ok(content) => {
            match serde_json::from_str::<serde_json::Value>(&content) {
                Ok(json) => {
                    let has_masday = json
                        .get("mcpServers")
                        .and_then(|s| s.as_object())
                        .and_then(|s| s.get("masday"))
                        .is_some();

                    if has_masday {
                        let binary_path = json
                            .get("mcpServers")
                            .and_then(|s| s.get("masday"))
                            .and_then(|m| m.get("command"))
                            .and_then(|c| c.as_str());

                        if let Some(path) = binary_path {
                            if std::path::Path::new(path).exists() {
                                report.checks.push(CheckResult {
                                    name: "MCP Config".into(),
                                    status: CheckStatus::Ok,
                                    message: "Valid config found".into(),
                                    detail: Some(if is_global {
                                        "~/.claude/.mcp.json".into()
                                    } else {
                                        ".mcp.json".into()
                                    }),
                                });
                            } else {
                                report.checks.push(CheckResult {
                                    name: "MCP Config".into(),
                                    status: CheckStatus::Warn,
                                    message: "Binary path invalid".into(),
                                    detail: Some(format!("Path not found: {}", path)),
                                });
                            }
                        } else {
                            report.checks.push(CheckResult {
                                name: "MCP Config".into(),
                                status: CheckStatus::Warn,
                                message: "Missing command field".into(),
                                detail: Some(config_path.display().to_string()),
                            });
                        }
                    } else {
                        report.checks.push(CheckResult {
                            name: "MCP Config".into(),
                            status: CheckStatus::Warn,
                            message: "Masday server not configured".into(),
                            detail: Some(config_path.display().to_string()),
                        });
                    }
                }
                Err(e) => {
                    report.checks.push(CheckResult {
                        name: "MCP Config".into(),
                        status: CheckStatus::Fail,
                        message: format!("Invalid JSON: {}", e),
                        detail: Some(config_path.display().to_string()),
                    });
                }
            }
        }
        Err(e) => {
            report.checks.push(CheckResult {
                name: "MCP Config".into(),
                status: CheckStatus::Fail,
                message: format!("Cannot read: {}", e),
                detail: Some(config_path.display().to_string()),
            });
        }
    }
}

async fn apply_fixes(report: &DoctorReport) -> Result<Vec<String>> {
    let mut fixes = Vec::new();

    for check in &report.checks {
        if check.name == "Home Dir" && check.status == CheckStatus::Fail {
            if let Err(e) = create_masday_directory() {
                eprintln!("Failed to create ~/.masday/: {}", e);
            } else {
                fixes.push("Created ~/.masday/ directory structure".into());
            }
        }
    }

    for check in &report.checks {
        if check.name == "SQLite Database" && check.status == CheckStatus::Fail {
            if let Err(e) = create_sqlite_database().await {
                eprintln!("Failed to create SQLite database: {}", e);
            } else {
                fixes.push("Created ~/.masday/data.db with schema".into());
            }
        }
    }

    for check in &report.checks {
        if check.name == "MCP Config" && check.status == CheckStatus::Warn
            && check.message.contains("Binary path invalid")
        {
            if let Err(e) = fix_mcp_config_binary_path() {
                eprintln!("Failed to fix MCP config: {}", e);
            } else {
                fixes.push("Updated MCP config binary path".into());
            }
        }
    }

    for check in &report.checks {
        if check.name == "Stale Workflows" && check.status == CheckStatus::Warn {
            if let Err(e) = reset_stale_workflows().await {
                eprintln!("Failed to reset stale workflows: {}", e);
            } else {
                fixes.push("Reset stale workflows to PAUSED".into());
            }
        }
    }

    Ok(fixes)
}

fn create_masday_directory() -> Result<()> {
    let home = MasdayConfig::masday_home();
    std::fs::create_dir_all(&home)?;
    std::fs::create_dir_all(home.join("bin"))?;
    std::fs::create_dir_all(home.join("context"))?;
    std::fs::create_dir_all(home.join("state"))?;
    std::fs::create_dir_all(home.join("research"))?;
    std::fs::create_dir_all(home.join("notes"))?;
    std::fs::create_dir_all(home.join("plans"))?;
    Ok(())
}

async fn create_sqlite_database() -> Result<()> {
    let home = MasdayConfig::masday_home();
    let db_path = home.join("data.db");

    if let Some(parent) = db_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let conn = rusqlite::Connection::open(&db_path)?;
    conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")?;
    conn.execute_batch(masday_mcp::sqlite_schema::SCHEMA)?;

    eprintln!("Created SQLite database at {}", db_path.display());
    Ok(())
}

fn fix_mcp_config_binary_path() -> Result<()> {
    let home = home::home_dir().ok_or_else(|| anyhow::anyhow!("Cannot determine home directory"))?;
    let global_mcp = home.join(".claude").join(".mcp.json");

    let masday_home = MasdayConfig::masday_home();
    let binary_name = if cfg!(windows) { "masday-mcp.exe" } else { "masday-mcp" };
    let correct_path = masday_home.join("bin").join(binary_name);

    if !global_mcp.exists() {
        anyhow::bail!("Global MCP config not found at {}", global_mcp.display());
    }

    let content = std::fs::read_to_string(&global_mcp)?;
    let mut json: serde_json::Value = serde_json::from_str(&content)?;

    if let Some(mcp_servers) = json.get_mut("mcpServers").and_then(|s| s.as_object_mut()) {
        if let Some(masday) = mcp_servers.get_mut("masday").and_then(|m| m.as_object_mut()) {
            masday.insert("command".into(), serde_json::json!(correct_path.display().to_string()));
            masday.insert("args".into(), serde_json::json!(["stdio"]));

            let updated = serde_json::to_string_pretty(&json)?;
            std::fs::write(&global_mcp, updated)?;
            eprintln!("Updated MCP config with path: {}", correct_path.display());
            return Ok(());
        }
    }

    anyhow::bail!("Masday server not found in MCP config");
}

async fn reset_stale_workflows() -> Result<()> {
    let home = MasdayConfig::masday_home();
    let db_path = home.join("data.db");

    if !db_path.exists() {
        anyhow::bail!("Database not found at {}", db_path.display());
    }

    let conn = rusqlite::Connection::open(&db_path)?;

    let updated = conn.execute(
        "UPDATE workflows
         SET status = 'PAUSED', updated_at = datetime('now')
         WHERE status IN ('RUNNING', 'EXECUTE')
         AND datetime(updated_at) < datetime('now', '-30 minutes')",
        [],
    )?;

    if updated > 0 {
        eprintln!("Reset {} stale workflows to PAUSED", updated);
    }

    Ok(())
}

fn is_port_open(port: u16) -> bool {
    use std::net::{TcpStream, SocketAddr};
    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    TcpStream::connect_timeout(&addr, std::time::Duration::from_millis(100)).is_ok()
}
