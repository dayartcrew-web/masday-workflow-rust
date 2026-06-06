//! Docker container management for local PostgreSQL and Redis.
//!
//! Manages containers using the `docker` CLI.
//! Used by `masday db start/stop/reset` and the setup wizard.

use anyhow::{bail, Context, Result};
use masday_core::constants::ports;
use std::process::Command;
use std::time::Duration;

const POSTGRES_CONTAINER: &str = "masday-postgres";
const POSTGRES_IMAGE: &str = "pgvector/pgvector:pg16";
const REDIS_CONTAINER: &str = "masday-redis";
const REDIS_IMAGE: &str = "redis:7-alpine";

/// Default PostgreSQL credentials for Docker containers.
/// Can be overridden via environment variables MASDAY_PG_USER / MASDAY_PG_PASSWORD / MASDAY_PG_DB.
pub const DEFAULT_PG_USER: &str = "masday";
pub const DEFAULT_PG_PASSWORD: &str = "masdaypass";
pub const DEFAULT_PG_DB: &str = "masday_workflow";

/// Read PG user from env or return default.
pub fn pg_user() -> String {
    std::env::var("MASDAY_PG_USER").unwrap_or_else(|_| DEFAULT_PG_USER.to_string())
}

/// Read PG password from env or return default.
pub fn pg_password() -> String {
    std::env::var("MASDAY_PG_PASSWORD").unwrap_or_else(|_| DEFAULT_PG_PASSWORD.to_string())
}

/// Read PG database name from env or return default.
pub fn pg_db() -> String {
    std::env::var("MASDAY_PG_DB").unwrap_or_else(|_| DEFAULT_PG_DB.to_string())
}

/// Check if Docker CLI is available
pub fn is_docker_available() -> bool {
    which::which("docker").is_ok()
}

/// Check if a Docker container is currently running
pub fn is_container_running(name: &str) -> bool {
    Command::new("docker")
        .args(["ps", "-q", "-f", &format!("name={}", name)])
        .output()
        .map(|o| !o.stdout.is_empty())
        .unwrap_or(false)
}

/// Check if a container exists (running or stopped)
fn container_exists(name: &str) -> bool {
    Command::new("docker")
        .args(["ps", "-aq", "-f", &format!("name={}", name)])
        .output()
        .map(|o| !o.stdout.is_empty())
        .unwrap_or(false)
}

/// Start PostgreSQL container. Creates if not exists, starts if stopped.
pub fn start_postgres(user: &str, password: &str, db_name: &str) -> Result<()> {
    if !is_docker_available() {
        bail!("Docker is not installed. Install Docker Desktop or Docker Engine first.");
    }

    if is_container_running(POSTGRES_CONTAINER) {
        println!(
            "  PostgreSQL already running on port {}",
            ports::postgres_port()
        );
        return Ok(());
    }

    // Remove old stopped container
    if container_exists(POSTGRES_CONTAINER) {
        let _ = Command::new("docker")
            .args(["rm", "-f", POSTGRES_CONTAINER])
            .output();
    }

    println!("  Starting PostgreSQL container...");
    let status = Command::new("docker")
        .args([
            "run",
            "-d",
            "--name",
            POSTGRES_CONTAINER,
            "-p",
            &format!("{}:5432", ports::postgres_port()),
            "-e",
            &format!("POSTGRES_USER={}", user),
            "-e",
            &format!("POSTGRES_PASSWORD={}", password),
            "-e",
            &format!("POSTGRES_DB={}", db_name),
            POSTGRES_IMAGE,
        ])
        .status()
        .context("Failed to run docker command")?;

    if !status.success() {
        bail!(
            "Failed to start PostgreSQL container. Check 'docker logs {}' for details.",
            POSTGRES_CONTAINER
        );
    }

    Ok(())
}

/// Start Redis container
pub fn start_redis() -> Result<()> {
    if is_container_running(REDIS_CONTAINER) {
        println!("  Redis already running on port {}", ports::redis_port());
        return Ok(());
    }

    if container_exists(REDIS_CONTAINER) {
        let _ = Command::new("docker")
            .args(["rm", "-f", REDIS_CONTAINER])
            .output();
    }

    println!("  Starting Redis container...");
    let status = Command::new("docker")
        .args([
            "run",
            "-d",
            "--name",
            REDIS_CONTAINER,
            "-p",
            &format!("{}:6379", ports::redis_port()),
            REDIS_IMAGE,
        ])
        .status()
        .context("Failed to run docker command")?;

    if !status.success() {
        bail!("Failed to start Redis container.");
    }

    Ok(())
}

/// Stop PostgreSQL container
pub fn stop_postgres() -> Result<()> {
    if !is_container_running(POSTGRES_CONTAINER) {
        println!("  PostgreSQL is not running");
        return Ok(());
    }
    let status = Command::new("docker")
        .args(["stop", POSTGRES_CONTAINER])
        .status()
        .context("Failed to stop container")?;
    if !status.success() {
        bail!("Failed to stop PostgreSQL container");
    }
    println!("  PostgreSQL stopped");
    Ok(())
}

/// Stop Redis container
pub fn stop_redis() -> Result<()> {
    if !is_container_running(REDIS_CONTAINER) {
        println!("  Redis is not running");
        return Ok(());
    }
    let _ = Command::new("docker")
        .args(["stop", REDIS_CONTAINER])
        .status();
    println!("  Redis stopped");
    Ok(())
}

/// Stop all masday containers
pub fn stop_all() -> Result<()> {
    stop_postgres()?;
    stop_redis()?;
    Ok(())
}

/// Reset: remove and recreate PostgreSQL container
pub fn reset_postgres(user: &str, password: &str, db_name: &str) -> Result<()> {
    println!("  Resetting PostgreSQL...");
    let _ = Command::new("docker")
        .args(["rm", "-f", POSTGRES_CONTAINER])
        .output();
    start_postgres(user, password, db_name)?;
    println!("  PostgreSQL reset complete");
    Ok(())
}

/// Wait for PostgreSQL to accept TCP connections.
/// Retries with 500ms interval until timeout.
pub fn wait_for_postgres(host: &str, port: u16, timeout_secs: u64) -> Result<()> {
    let addr = format!("{}:{}", host, port);
    let timeout = Duration::from_secs(timeout_secs);
    let start = std::time::Instant::now();

    while start.elapsed() < timeout {
        if std::net::TcpStream::connect_timeout(
            &addr
                .parse()
                .map_err(|e| anyhow::anyhow!("Invalid address {}: {}", addr, e))?,
            Duration::from_secs(2),
        )
        .is_ok()
        {
            return Ok(());
        }
        std::thread::sleep(Duration::from_millis(500));
    }

    bail!(
        "PostgreSQL did not become ready within {}s on {}",
        timeout_secs,
        addr
    )
}

/// Start PostgreSQL container with default masday credentials.
pub fn start_postgres_default() -> Result<()> {
    start_postgres(&pg_user(), &pg_password(), &pg_db())
}

/// Reset PostgreSQL container with default masday credentials.
pub fn reset_postgres_default() -> Result<()> {
    reset_postgres(&pg_user(), &pg_password(), &pg_db())
}

/// Get default database URL for local mode (includes credentials).
/// Reads from MASDAY_PG_USER, MASDAY_PG_PASSWORD, MASDAY_PG_DB env vars
/// with sensible defaults.
pub fn default_database_url() -> String {
    format!(
        "postgresql://{}:{}@localhost:{}/{}",
        pg_user(),
        pg_password(),
        ports::postgres_port(),
        pg_db()
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_docker_available_check() {
        // Just verify it doesn't panic
        let _ = is_docker_available();
    }

    #[test]
    fn test_container_not_running() {
        assert!(!is_container_running("masday-test-nonexistent-xyz-12345"));
    }

    #[test]
    fn test_default_database_url() {
        let url = default_database_url();
        assert!(url.contains(&format!("localhost:{}", ports::POSTGRES_PORT)));
        assert!(url.contains("masday_workflow"));
    }

    #[test]
    fn test_default_database_url_includes_credentials() {
        let url = default_database_url();
        assert!(
            url.starts_with("postgresql://masday:masdaypass@"),
            "URL must contain user:password@host, got: {}",
            url
        );
        assert!(url.contains("/masday_workflow"));
    }

    #[test]
    fn test_pg_credential_helpers() {
        assert_eq!(pg_user(), "masday");
        assert_eq!(pg_password(), "masdaypass");
        assert_eq!(pg_db(), "masday_workflow");
    }

    #[test]
    fn test_pg_credential_constants() {
        assert_eq!(DEFAULT_PG_USER, "masday");
        assert_eq!(DEFAULT_PG_PASSWORD, "masdaypass");
        assert_eq!(DEFAULT_PG_DB, "masday_workflow");
    }
}
