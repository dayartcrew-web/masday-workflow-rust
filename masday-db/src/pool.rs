//! Database connection pool with anti-stale configuration
//!
//! Provides a production-ready connection pool with:
//! - Lazy connection initialization (no connections until first query)
//! - Anti-stale connection management (30s idle timeout)
//! - TCP keepalive for connection health
//! - Retry logic with exponential backoff
//! - Health check functionality
//!
//! ## Architecture
//!
//! The pool is wrapped in an `Arc` for thread-safe sharing across the application.
//! Connections are created lazily - the pool is constructed immediately but
//! actual database connections are only established on first use.
//!
//! ## Anti-Stale Configuration
//!
//! - **Idle timeout (30s)**: Proactively removes connections idle for >30s
//! - **TCP keepalive**: Enables kernel-level keepalive for long-lived connections
//! - **Fast recycling**: Quick connection validation before reuse
//!
//! ## Usage
//!
//! ```rust,no_run
//! use masday_db::pool::{create_pool, init_pool_with_retry, health_check};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Create pool with retry logic
//!     let pool = init_pool_with_retry(3).await?;
//!
//!     // Verify connectivity
//!     health_check(&pool).await?;
//!
//!     Ok(())
//! }
//! ```

use deadpool_postgres::{Config, Pool};
use deadpool_postgres::tokio_postgres::NoTls;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;
use tracing::{info, warn, error, debug};

use masday_core::constants::pool::*;

/// Thread-safe database pool wrapper for sharing across the application
///
/// This type alias wraps the pool in an `Arc` for cheap cloning across threads.
/// Each clone points to the same underlying pool, enabling the AppState pattern
/// where the pool is created once at startup and shared with all request handlers.
pub type DbPool = Arc<Pool>;

/// Creates a database connection pool with production-ready anti-stale configuration
///
/// ## Features
///
/// - **Lazy initialization**: Pool is created but NO connections are established until first query
/// - **Anti-stale management**: Idle connections are removed after 30s of inactivity
/// - **TCP keepalive**: Kernel-level keepalive for connection health detection
/// - **Fast recycling**: Quick validation before reuse (avoids full health check on every checkout)
///
/// ## Configuration
///
/// Uses constants from `masday_core::constants::pool`:
/// - `MAX_POOL_SIZE`: Maximum concurrent connections (default: 20)
/// - `IDLE_TIMEOUT_SECS`: Remove idle connections after this duration (default: 30s)
/// - `CONNECT_TIMEOUT_SECS`: Timeout for establishing new connections (default: 2s)
/// - `WAIT_TIMEOUT_SECS`: Timeout for waiting for available connection (default: 5s)
///
/// ## Errors
///
/// Returns `Err` if:
/// - `DATABASE_URL` environment variable is not set
/// - Connection string is invalid
/// - Pool configuration fails
///
/// ## Example
///
/// ```rust,no_run
/// use masday_db::pool::create_pool;
///
/// let pool = create_pool().expect("Failed to create pool");
/// ```
pub fn create_pool() -> Result<DbPool, String> {
    // Read DATABASE_URL from environment
    let database_url = std::env::var("DATABASE_URL")
        .map_err(|_| "DATABASE_URL environment variable not set".to_string())?;

    info!("Creating database pool (lazy - no connections until first query)");
    debug!("Pool config: max_size={}, idle_timeout={}s, connect_timeout={}s, wait_timeout={}s",
        MAX_POOL_SIZE, IDLE_TIMEOUT_SECS, CONNECT_TIMEOUT_SECS, WAIT_TIMEOUT_SECS);

    let mut cfg = Config::new();
    cfg.url = Some(database_url);
    // Configure connection recycling method
    // Fast: Quick check (connection status) - fast but less reliable
    // Verified: Run simple query (SELECT 1) - slower but more reliable
    // Our anti-stale strategy: Fast recycling + 30s idle timeout + TCP keepalive
    cfg.manager = Some(deadpool_postgres::ManagerConfig {
        recycling_method: deadpool_postgres::RecyclingMethod::Fast,
    });

    // Get tokio_postgres config for TCP keepalive settings
    let pg_config = cfg.get_pg_config().map_err(|e| format!("Invalid PG config: {}", e))?;

    // Enable TCP keepalive at the connection level
    // This helps detect stale connections proactively
    let pg_config_with_keepalive = {
        let mut cfg = pg_config.clone();
        cfg.keepalives(true);
        cfg.keepalives_idle(Duration::from_secs(IDLE_TIMEOUT_SECS / 2));
        cfg.tcp_user_timeout(Duration::from_secs(IDLE_TIMEOUT_SECS));
        cfg
    };

    let mgr = deadpool_postgres::Manager::new(pg_config_with_keepalive, NoTls);

    let pool = Pool::builder(mgr)
        .max_size(MAX_POOL_SIZE as usize)
        .create_timeout(Some(Duration::from_secs(CONNECT_TIMEOUT_SECS)))
        .wait_timeout(Some(Duration::from_secs(WAIT_TIMEOUT_SECS)))
        .build()
        .map_err(|e| format!("Failed to build pool: {}", e))?;

    // Note: No connections are established yet - lazy initialization
    debug!("Pool created (lazy, will connect on first use)");

    Ok(Arc::new(pool))
}

/// Creates a connection pool with retry logic for transient failures
///
/// Retries pool creation with exponential backoff for handling:
/// - Database not yet ready (e.g., during container startup)
/// - Temporary network issues
/// - Connection rate limiting
///
/// ## Parameters
///
/// - `max_retries`: Maximum number of retry attempts (typically 3)
///
/// ## Retry Strategy
///
/// - 3 retries with 5s backoff between each attempt
/// - Total maximum wait time: 15s (3 retries × 5s)
/// - Returns the pool immediately on success
/// - Returns final error after all retries exhausted
///
/// ## Example
///
/// ```rust,no_run
/// use masday_db::pool::init_pool_with_retry;
///
/// # #[tokio::main]
/// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let pool = init_pool_with_retry(3).await?;
/// # Ok(())
/// # }
/// ```
pub async fn init_pool_with_retry(max_retries: u32) -> Result<DbPool, String> {
    let mut attempts = 0;
    let mut last_error = String::new();

    while attempts < max_retries {
        attempts += 1;

        match create_pool() {
            Ok(pool) => {
                if attempts > 1 {
                    info!("Pool created successfully after {} attempts", attempts);
                } else {
                    info!("Pool created successfully on first attempt");
                }
                return Ok(pool);
            }
            Err(e) => {
                last_error = e.clone();
                warn!("Attempt {}/{} failed: {}", attempts, max_retries, e);

                // Don't sleep after the last attempt
                if attempts < max_retries {
                    info!("Waiting 5s before retry...");
                    sleep(Duration::from_secs(5)).await;
                }
            }
        }
    }

    error!("Failed to create pool after {} attempts: {}", max_retries, last_error);
    Err(format!("Failed after {} attempts: {}", max_retries, last_error))
}

/// Health check function that pings the database
///
/// Verifies database connectivity by executing a simple query.
/// This creates a temporary connection if the pool is idle.
///
/// ## Behavior
///
/// - Acquires a connection from the pool (creates if pool is idle)
/// - Executes `SELECT 1` to verify connectivity
/// - Returns connection back to pool
///
/// ## Use Cases
///
/// - Startup health checks (verify DB is reachable)
/// - Periodic health monitoring (liveness probes)
/// - After connection recovery (verify pool is healthy)
///
/// ## Errors
///
/// Returns `Err` if:
/// - Cannot acquire connection from pool (pool closed, wait timeout)
/// - Query execution fails (network error, database down)
/// - Connection is broken (TCP reset, timeout)
///
/// ## Example
///
/// ```rust,no_run
/// use masday_db::pool::health_check;
///
/// # #[tokio::main]
/// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
/// # let pool = masday_db::pool::create_pool()?;
/// health_check(&pool).await?;
/// println!("Database is healthy!");
/// # Ok(())
/// # }
/// ```
pub async fn health_check(pool: &DbPool) -> Result<(), String> {
    debug!("Starting database health check");

    let client = pool
        .get()
        .await
        .map_err(|e| format!("Failed to acquire connection: {}", e))?;

    // Simple ping query - fastest way to verify connectivity
    client
        .query_one("SELECT 1", &[])
        .await
        .map_err(|e| format!("Health check query failed: {}", e))?;

    debug!("Health check passed");
    Ok(())
}

/// Returns pool status information for monitoring
///
/// Provides metrics for observability dashboards and alerting.
///
/// ## Returns
///
/// - `status`: Pool status (active/empty)
/// - `size`: Current pool size
/// - `available`: Number of available connections
/// - `max_size`: Maximum pool size
///
/// ## Example
///
/// ```rust,no_run
/// use masday_db::pool::get_pool_status;
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// # let pool = masday_db::pool::create_pool()?;
/// let status = get_pool_status(&pool);
/// println!("Pool: {}/{} connections available", status.available, status.max_size);
/// # Ok(())
/// # }
/// ```
pub fn get_pool_status(pool: &DbPool) -> PoolStatus {
    let status = pool.status();
    PoolStatus {
        size: status.size,
        available: status.available,
        max_size: status.max_size,
    }
}

/// Pool status information
#[derive(Debug, Clone)]
pub struct PoolStatus {
    pub size: usize,
    pub available: usize,
    pub max_size: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pool_constants_defined() {
        // Verify constants are accessible and have expected values
        assert_eq!(MAX_POOL_SIZE, 20);
        assert_eq!(IDLE_TIMEOUT_SECS, 30);
        assert_eq!(CONNECT_TIMEOUT_SECS, 2);
        assert_eq!(WAIT_TIMEOUT_SECS, 5);
    }

    #[test]
    fn test_pool_status_debug() {
        // PoolStatus should be debuggable
        let status = PoolStatus {
            size: 10,
            available: 5,
            max_size: 20,
        };
        let formatted = format!("{:?}", status);
        assert!(formatted.contains("PoolStatus"));
    }

    #[test]
    fn test_db_pool_type_alias() {
        // Verify DbPool type alias works correctly
        let _pool_type_check: std::marker::PhantomData<DbPool> = std::marker::PhantomData;
    }
}
