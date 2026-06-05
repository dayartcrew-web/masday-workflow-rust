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
pub fn run(json: bool) -> Result<()> {
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
    check_api_connectivity(&mut report);
    check_mcp_binary(&mut report);
    check_embedding(&mut report);
    check_platforms(&mut report);
    check_disk_space(&mut report);
    check_update_available(&mut report);

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
        message: format!(
            "v{} ({})",
            report.version, report.build_origin
        ),
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
    let in_path = path_var
        .split(path_sep)
        .any(|p| Path::new(p) == bin_dir);

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

fn check_api_connectivity(report: &mut DoctorReport) {
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

    match reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()
    {
        Ok(client) => match client.get(&health_url).send() {
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
    let binary_name = if cfg!(windows) { "masday.exe" } else { "masday" };
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
            detail: Some(format!(
                "Checked: {} and current exe",
                bin_path.display()
            )),
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
                    config
                        .embedding_provider
                        .as_deref()
                        .unwrap_or("unknown"),
                    config
                        .embedding_model
                        .as_deref()
                        .unwrap_or("unknown")
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

fn check_update_available(report: &mut DoctorReport) {
    // Reuse update module's version check
    match crate::commands::update::fetch_latest_version() {
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
    println!("{}", style("  ───────────────────────────────────────").dim());

    for check in &report.checks {
        let icon = match check.status {
            CheckStatus::Ok => style("✓").green(),
            CheckStatus::Warn => style("⚠").yellow(),
            CheckStatus::Fail => style("✗").red(),
            CheckStatus::Skip => style("—").dim(),
        };

        println!("  {} {}: {}", icon, style(&check.name).bold(), check.message);

        if let Some(ref detail) = check.detail {
            println!("    {}", style(detail).dim());
        }
    }

    println!();
    println!("{}", style("  ───────────────────────────────────────").dim());

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
