//! Status command — health check for all Masday services.

use anyhow::Result;
use console::style;
use masday_core::constants::ports;

use crate::config::MasdayConfig;

/// Check health of all Masday services
pub async fn run() -> Result<()> {
    let config = MasdayConfig::load_or_err()?;

    println!("{}", style("Masday Status").cyan().bold());
    println!();

    // Check API health
    let health_url = format!("{}/api/health", config.api_url);
    match reqwest::get(&health_url).await {
        Ok(resp) if resp.status().is_success() => {
            println!(
                "  API:        {} {}",
                style("✓ healthy").green(),
                config.api_url
            );
        }
        Ok(resp) => {
            println!(
                "  API:        {} ({})",
                style("⚠ unhealthy").yellow(),
                resp.status()
            );
        }
        Err(e) => {
            println!("  API:        {}", style("✗ not running").red());
            println!("              {}", e);
            if config.mode == "local" {
                println!(
                    "              Run '{}' to start",
                    style("masday serve").cyan()
                );
            }
        }
    }

    // Check database connectivity
    if config.mode == "local" {
        if let Some(ref db_url) = config.database_url {
            // External database URL (e.g. Supabase) — test connectivity directly
            print!("  PostgreSQL: ");
            std::env::set_var("DATABASE_URL", db_url);
            match masday_db::pool::init_pool_with_retry(2).await {
                Ok(pool) => match masday_db::pool::health_check(&pool).await {
                    Ok(_) => {
                        let safe_url = redact_db_url(db_url);
                        println!("{} ({})", style("✓ connected").green(), safe_url);
                    }
                    Err(e) => println!("{} health check failed: {}", style("✗").red(), e),
                },
                Err(e) => {
                    let safe_url = redact_db_url(db_url);
                    println!("{} ({})", style("✗ unreachable").red(), safe_url);
                    println!("              {}", e);
                }
            }
        } else if crate::docker::is_docker_available() {
            if crate::docker::is_container_running("masday-postgres") {
                println!(
                    "  PostgreSQL: {} (Docker, port {})",
                    style("✓ running").green(),
                    ports::postgres_port()
                );
            } else {
                println!("  PostgreSQL: {}", style("✗ not running").red());
                println!(
                    "              Run '{}' to start",
                    style("masday db start").cyan()
                );
            }
        } else {
            println!(
                "  PostgreSQL: {} (no database_url configured)",
                style("✗").red()
            );
        }

        // Redis check (only Docker-based)
        if crate::docker::is_docker_available() {
            if crate::docker::is_container_running("masday-redis") {
                println!(
                    "  Redis:      {} (Docker, port {})",
                    style("✓ running").green(),
                    ports::redis_port()
                );
            } else {
                println!("  Redis:      {}", style("✗ not running").yellow());
            }
        }

        // Check embedding
        if let Some(ref provider) = config.embedding_provider {
            if !provider.is_empty() {
                println!(
                    "  Embedding:  {} ({})",
                    style("✓ configured").green(),
                    provider
                );
            } else {
                println!("  Embedding:  {}", style("— not configured").yellow());
            }
        } else {
            println!("  Embedding:  {}", style("— not configured").yellow());
        }
    }

    // Show config info
    println!();
    println!("  Mode:       {}", config.mode);
    println!("  Platforms:  {}", config.platforms.join(", "));
    println!("  Config:     {}", MasdayConfig::config_path().display());

    Ok(())
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
