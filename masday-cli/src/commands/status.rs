//! Status command — health check for all Masday services.

use anyhow::Result;
use console::style;

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
            println!("  API:        {} {}", style("✓ healthy").green(), config.api_url);
        }
        Ok(resp) => {
            println!("  API:        {} ({})", style("⚠ unhealthy").yellow(), resp.status());
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

    // Check Docker containers (local mode only)
    if config.mode == "local" {
        if crate::docker::is_docker_available() {
            if crate::docker::is_container_running("masday-postgres") {
                println!("  PostgreSQL: {} (Docker, port 5434)", style("✓ running").green());
            } else {
                println!("  PostgreSQL: {}", style("✗ not running").red());
                println!(
                    "              Run '{}' to start",
                    style("masday db start").cyan()
                );
            }

            if crate::docker::is_container_running("masday-redis") {
                println!("  Redis:      {} (Docker, port 6379)", style("✓ running").green());
            } else {
                println!("  Redis:      {}", style("✗ not running").yellow());
            }
        } else {
            println!("  Docker:     {}", style("✗ not installed").yellow());
        }

        // Check embedding
        if let Some(ref provider) = config.embedding_provider {
            println!("  Embedding:  {} ({})", style("✓ configured").green(), provider);
        } else {
            println!("  Embedding:  {}", style("— not configured").yellow());
        }
    }

    // Show config info
    println!();
    println!("  Mode:       {}", config.mode);
    println!("  Platforms:  {}", config.platforms.join(", "));
    println!(
        "  Config:     {}",
        MasdayConfig::config_path().display()
    );

    Ok(())
}
