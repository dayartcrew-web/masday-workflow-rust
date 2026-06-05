//! PostgreSQL connection for MCP local mode.
//!
//! Uses masday-db pool + repos for direct database access.
//! Falls back to SQLite if PostgreSQL is unavailable.

use masday_db::pool::{create_pool, health_check, DbPool};
use once_cell::sync::Lazy;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Read database_url from ~/.masday/config.toml directly.
fn read_db_url_from_config() -> Option<String> {
    let home = home::home_dir()?;
    let config_path = home.join(".masday").join("config.toml");
    if !config_path.exists() {
        return None;
    }

    let content = std::fs::read_to_string(&config_path).ok()?;
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("database_url") || trimmed.starts_with("database-url") {
            if let Some(value) = trimmed.split('=').nth(1) {
                let url = value
                    .trim()
                    .trim_matches('"')
                    .trim_matches('\'')
                    .to_string();
                if !url.is_empty() {
                    return Some(url);
                }
            }
        }
    }
    None
}

/// Global PostgreSQL pool for MCP local mode.
/// None if PostgreSQL is not available or not configured.
static PG_POOL: Lazy<Arc<RwLock<Option<DbPool>>>> = Lazy::new(|| Arc::new(RwLock::new(None)));

/// Initialize PostgreSQL pool.
/// Reads DATABASE_URL from env var (set by masday CLI via config.toml) or falls back to config file.
pub async fn init_pg() -> bool {
    let database_url = std::env::var("DATABASE_URL").unwrap_or_default();

    // If no env var, try reading config.toml directly
    let database_url = if database_url.is_empty() {
        match read_db_url_from_config() {
            Some(url) => url,
            None => {
                tracing::info!("No DATABASE_URL configured, PostgreSQL not available");
                return false;
            }
        }
    } else {
        database_url
    };

    tracing::info!(
        "Connecting to PostgreSQL: {}...",
        &database_url[..database_url.len().min(40)]
    );

    // Set env var for masday-db::pool::create_pool()
    std::env::set_var("DATABASE_URL", &database_url);

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
