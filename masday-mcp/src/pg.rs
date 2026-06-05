//! PostgreSQL connection for MCP local mode.
//!
//! On-demand connection: pool is NOT initialized at startup.
//! Instead, it connects lazily when a tool needs to store/sync to PostgreSQL.
//! This avoids keeping a connection open when only SQLite is needed.

use masday_db::pool::{create_pool, health_check, DbPool};
use once_cell::sync::Lazy;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Global PostgreSQL pool — lazily initialized on first use, not at startup.
static PG_POOL: Lazy<Arc<RwLock<Option<DbPool>>>> = Lazy::new(|| Arc::new(RwLock::new(None)));

/// Embedded migration SQL — included at compile time from masday-db.
const MIGRATION_SQL: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../masday-db/migrations/001_initial_schema.sql"));

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

/// Run embedded migrations directly (doesn't depend on filesystem migrations dir).
async fn run_embedded_migrations(pool: &DbPool) -> Result<(), String> {
    let client = pool
        .get()
        .await
        .map_err(|e| format!("Failed to get DB connection: {}", e))?;

    // Split by semicolons and execute each statement
    for statement in MIGRATION_SQL.split(';') {
        let trimmed = statement.trim();
        if trimmed.is_empty() || trimmed.starts_with("--") {
            continue;
        }
        // Skip comment-only blocks
        let sql: String = trimmed
            .lines()
            .filter(|line| !line.trim().starts_with("--"))
            .collect::<Vec<_>>()
            .join("\n");
        if sql.trim().is_empty() {
            continue;
        }
        if let Err(e) = client.execute(&sql, &[]).await {
            // Some statements may fail if already exist (CREATE EXTENSION, CREATE TABLE)
            let msg = e.to_string();
            if msg.contains("already exists") {
                tracing::debug!(
                    "Schema already exists, skipping: {}",
                    &msg[..msg.len().min(80)]
                );
            } else {
                tracing::warn!("Migration statement warning: {}", msg);
            }
        }
    }

    tracing::info!("PostgreSQL schema ready (embedded migrations)");
    Ok(())
}

/// Get or create PostgreSQL pool on demand.
/// Returns None if database_url is not configured or connection fails.
/// First call connects + runs migrations. Subsequent calls reuse the pool.
pub async fn get_pool() -> Option<DbPool> {
    // Fast path: pool already exists
    {
        let pg = PG_POOL.read().await;
        if let Some(ref pool) = *pg {
            return Some(pool.clone());
        }
    }

    // Slow path: need to connect
    let database_url = std::env::var("DATABASE_URL")
        .ok()
        .or_else(read_db_url_from_config)
        .unwrap_or_default();

    if database_url.is_empty() {
        return None;
    }

    tracing::info!(
        "PostgreSQL: connecting on-demand → {}...",
        &database_url[..database_url.len().min(40)]
    );

    // Set env var for masday-db::pool::create_pool()
    std::env::set_var("DATABASE_URL", &database_url);

    let pool = create_pool().ok()?;

    // Verify connection
    match health_check(&pool).await {
        Ok(_) => tracing::info!("PostgreSQL connected"),
        Err(e) => {
            tracing::warn!("PostgreSQL health check failed: {}", e);
            return None;
        }
    }

    // Run embedded migrations (first connect only)
    let _ = run_embedded_migrations(&pool).await;

    // Store pool for reuse
    let mut pg = PG_POOL.write().await;
    *pg = Some(pool.clone());
    Some(pool)
}

/// Check if database_url is configured (without connecting).
pub fn is_configured() -> bool {
    let env = std::env::var("DATABASE_URL").ok().filter(|s| !s.is_empty());
    if env.is_some() {
        return true;
    }
    read_db_url_from_config().is_some()
}
