//! Database management commands — start/stop/reset PostgreSQL via Docker.

use anyhow::Result;
use console::style;
use masday_core::constants::ports;

use crate::docker;

/// Start PostgreSQL and Redis containers
pub fn start() -> Result<()> {
    println!("{}", style("Starting database containers...").cyan());
    docker::start_postgres_default()?;
    docker::wait_for_postgres("localhost", ports::postgres_port(), 30)?;
    docker::start_redis()?;
    println!();
    println!("{}", style("✓ Database containers ready").green());
    println!("  PostgreSQL: localhost:{}", ports::postgres_port());
    println!("  Redis:      localhost:{}", ports::redis_port());
    Ok(())
}

/// Stop PostgreSQL and Redis containers
pub fn stop() -> Result<()> {
    println!("{}", style("Stopping database containers...").cyan());
    docker::stop_all()?;
    println!();
    println!("{}", style("✓ Database containers stopped").green());
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

    docker::reset_postgres_default()?;
    docker::wait_for_postgres("localhost", ports::postgres_port(), 30)?;

    // Run migrations on fresh database
    std::env::set_var("DATABASE_URL", docker::default_database_url());
    let pool = masday_db::pool::init_pool_with_retry(5)
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))?;
    masday_db::run_migrations(&pool)
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))?;

    println!();
    println!("{}", style("✓ Database reset complete").green());
    Ok(())
}
