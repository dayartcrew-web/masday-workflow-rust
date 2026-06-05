//! PostgreSQL connection for MCP local mode.
//!
//! Uses masday-db pool + repos for direct database access.
//! Falls back to SQLite if PostgreSQL is unavailable.

use masday_db::pool::{create_pool, health_check, DbPool};
use once_cell::sync::Lazy;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Global PostgreSQL pool for MCP local mode.
/// None if PostgreSQL is not available or not configured.
static PG_POOL: Lazy<Arc<RwLock<Option<DbPool>>>> = Lazy::new(|| Arc::new(RwLock::new(None)));

/// Initialize PostgreSQL pool from DATABASE_URL env var.
/// Returns true if pool was created successfully.
pub async fn init_pg() -> bool {
    let database_url = std::env::var("DATABASE_URL").unwrap_or_default();
    if database_url.is_empty() {
        tracing::info!("No DATABASE_URL configured, PostgreSQL not available");
        return false;
    }

    tracing::info!(
        "Connecting to PostgreSQL: {}...",
        &database_url[..database_url.len().min(40)]
    );

    match create_pool() {
        Ok(pool) => {
            // Verify connection
            match health_check(&pool).await {
                Ok(_) => {
                    tracing::info!("PostgreSQL connected successfully");
                    // Run migrations
                    match masday_db::run_migrations(&pool).await {
                        Ok(_) => tracing::info!("PostgreSQL schema ready"),
                        Err(e) => tracing::warn!("Migration warning: {}", e),
                    }
                    let mut pg = PG_POOL.write().await;
                    *pg = Some(pool);
                    true
                }
                Err(e) => {
                    tracing::warn!("PostgreSQL health check failed: {}", e);
                    false
                }
            }
        }
        Err(e) => {
            tracing::warn!("PostgreSQL pool creation failed: {}", e);
            false
        }
    }
}

/// Get the PostgreSQL pool if available.
pub async fn get_pool() -> Option<DbPool> {
    let pg = PG_POOL.read().await;
    pg.clone()
}

/// Check if PostgreSQL is available.
pub async fn is_available() -> bool {
    let pg = PG_POOL.read().await;
    pg.is_some()
}
