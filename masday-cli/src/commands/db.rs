//! Database management commands — start/stop/reset/migrate PostgreSQL via Docker.

use anyhow::Result;
use console::style;
use indicatif::{ProgressBar, ProgressStyle};
use masday_core::constants::ports;
use std::time::Duration;

use crate::docker;

fn spinner(message: &str) -> ProgressBar {
    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.cyan} {msg}")
            .unwrap(),
    );
    pb.set_message(message.to_string());
    pb.enable_steady_tick(Duration::from_millis(80));
    pb
}

/// Start PostgreSQL and Redis containers
pub fn start() -> Result<()> {
    println!("{}", style("Starting database containers...").cyan());

    let pb = spinner("Starting PostgreSQL...");
    docker::start_postgres_default()?;
    docker::wait_for_postgres("localhost", ports::postgres_port(), 30)?;
    pb.finish_with_message(format!("{} PostgreSQL ready", style("✓").green()));

    let pb = spinner("Starting Redis...");
    docker::start_redis()?;
    pb.finish_with_message(format!("{} Redis ready", style("✓").green()));

    println!();
    println!("{}", style("✓ Database containers ready").green());
    println!("  PostgreSQL: localhost:{}", ports::postgres_port());
    println!("  Redis:      localhost:{}", ports::redis_port());
    Ok(())
}

/// Stop PostgreSQL and Redis containers
pub fn stop() -> Result<()> {
    println!("{}", style("Stopping database containers...").cyan());

    let pb = spinner("Stopping containers...");
    docker::stop_all()?;
    pb.finish_with_message(format!("{} Containers stopped", style("✓").green()));

    println!();
    println!("{}", style("✓ Database containers stopped").green());
    Ok(())
}

/// Run pending database migrations on the PostgreSQL instance
pub async fn migrate() -> Result<()> {
    println!("{}", style("Running database migrations...").cyan());

    let database_url = match std::env::var("DATABASE_URL") {
        Ok(url) => url,
        Err(_) => {
            let config = crate::config::MasdayConfig::load().ok_or_else(|| {
                anyhow::anyhow!(
                    "DATABASE_URL not set and config not found. Run 'masday quickstart' first."
                )
            })?;
            config.database_url.ok_or_else(|| {
                anyhow::anyhow!("No database_url in config. Run 'masday quickstart' first.")
            })?
        }
    };

    let pb = spinner("Connecting to database...");
    std::env::set_var("DATABASE_URL", &database_url);
    let pool = masday_db::pool::init_pool_with_retry(5)
        .await
        .map_err(|e| {
            pb.finish_with_message(format!("{} Connection failed", style("✗").red()));
            anyhow::anyhow!("Could not connect to database: {}", e)
        })?;
    pb.finish_with_message(format!("{} Connected", style("✓").green()));

    let pb = spinner("Applying migrations...");
    masday_db::run_migrations(&pool)
        .await
        .map_err(|e| {
            pb.finish_with_message(format!("{} Migration failed", style("✗").red()));
            anyhow::anyhow!("Migration failed: {}", e)
        })?;
    pb.finish_with_message(format!("{} Migrations applied", style("✓").green()));

    // Verify tables were created
    let pb = spinner("Verifying schema...");
    let client = pool.get().await.map_err(|e| anyhow::anyhow!("{}", e))?;
    let rows = client
        .query(
            "SELECT count(*) FROM information_schema.tables WHERE table_schema = 'public'",
            &[],
        )
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))?;
    let table_count: i64 = rows[0].get(0);
    pb.finish_with_message(format!(
        "{} Found {} tables",
        style("✓").green(),
        table_count
    ));

    println!();
    println!(
        "{}",
        style(format!(
            "✓ Migrations complete — {} tables in database",
            table_count
        ))
        .green()
    );
    Ok(())
}

/// Reset PostgreSQL (delete data and recreate)
pub async fn reset() -> Result<()> {
    println!("{}", style("Resetting database...").cyan());
    println!();
    println!(
        "  {} This will DELETE all data!",
        style("⚠ WARNING:").yellow()
    );
    let confirm = inquire::Confirm::new("Are you sure?")
        .with_default(false)
        .prompt()?;

    if !confirm {
        println!("Reset cancelled.");
        return Ok(());
    }

    let pb = spinner("Resetting PostgreSQL...");
    docker::reset_postgres_default()?;
    docker::wait_for_postgres("localhost", ports::postgres_port(), 30)?;
    pb.finish_with_message(format!("{} Database recreated", style("✓").green()));

    // Run migrations on fresh database
    std::env::set_var("DATABASE_URL", docker::default_database_url());

    let pb = spinner("Connecting...");
    let pool = masday_db::pool::init_pool_with_retry(5)
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))?;
    pb.finish_with_message(format!("{} Connected", style("✓").green()));

    let pb = spinner("Running migrations...");
    masday_db::run_migrations(&pool)
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))?;
    pb.finish_with_message(format!("{} Schema created", style("✓").green()));

    println!();
    println!("{}", style("✓ Database reset complete").green());
    Ok(())
}
