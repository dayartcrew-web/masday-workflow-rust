//! Database pool initialization for standalone (stdio) mode.
//!
//! Reads `DATABASE_URL` from environment, creates a connection pool with retry,
//! and stores it in a global `OnceLock` for access from direct-call handlers.

use masday_db::pool::{create_pool, health_check, DbPool};
use std::sync::OnceLock;
use tracing::info;

static POOL: OnceLock<DbPool> = OnceLock::new();

/// Initialize the database pool from `DATABASE_URL` env var.
///
/// Retries up to 3 times with 5s backoff between attempts.
/// Runs a health check after pool creation to verify connectivity.
pub async fn init_db_pool() -> Result<(), Box<dyn std::error::Error>> {
    let pool = masday_db::pool::init_pool_with_retry(3)
        .await
        .map_err(|e| format!("Database pool init failed: {}", e))?;

    health_check(&pool)
        .await
        .map_err(|e| format!("Database health check failed: {}", e))?;

    POOL.set(pool)
        .map_err(|_| "Pool already initialized".to_string())?;

    info!("Database pool initialized (standalone mode)");
    Ok(())
}

/// Get the global DB pool. Panics if not initialized.
pub fn pool() -> DbPool {
    POOL.get()
        .expect("Database pool not initialized — call init_db_pool() first")
        .clone()
}
