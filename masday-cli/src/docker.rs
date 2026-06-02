//! Docker container management for local PostgreSQL and Redis.
//!
//! Manages containers using the `docker` CLI.
//! Used by `masday db start/stop/reset` and the setup wizard.

use anyhow::{bail, Context, Result};
use std::process::Command;
use std::time::Duration;

const POSTGRES_CONTAINER: &str = "masday-postgres";
const POSTGRES_IMAGE: &str = "pgvector/pgvector:pg16";
const POSTGRES_PORT: u16 = 5434;
const REDIS_CONTAINER: &str = "masday-redis";
const REDIS_IMAGE: &str = "redis:7-alpine";
const REDIS_PORT: u16 = 6379;

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
        println!("  PostgreSQL already running on port {}", POSTGRES_PORT);
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
            &format!("{}:5432", POSTGRES_PORT),
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
        println!("  Redis already running on port {}", REDIS_PORT);
        return Ok(());
    }

    if container_exists(REDIS_CONTAINER) {
        let _ = Command::new("docker")
            .args(["rm", "-f", REDIS_CONTAINER])
            .output();
    }

    println!("  Starting Redis container...");
    let status = Command::new("docker")
        .args(["run", "-d", "--name", REDIS_CONTAINER, "-p", &format!("{}:6379", REDIS_PORT), REDIS_IMAGE])
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
            &addr.parse().map_err(|e| anyhow::anyhow!("Invalid address {}: {}", addr, e))?,
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

/// Get default database URL for local mode
pub fn default_database_url() -> String {
    "postgresql://USER:PASS@localhost:5434/masday_workflow".to_string()
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
        assert!(url.contains("localhost:5434"));
        assert!(url.contains("masday_workflow"));
    }
}
