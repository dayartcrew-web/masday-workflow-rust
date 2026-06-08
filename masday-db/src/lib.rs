//! masday-db - Database layer with connection pooling and repositories
//!
//! Provides a production-ready database layer with:
//! - Anti-stale connection pooling (30s idle timeout, TCP keepalive)
//! - Lazy connection initialization (no connections until first query)
//! - Retry logic with exponential backoff
//! - Health check functionality
//! - Repository pattern for domain entities
//!
//! ## Architecture
//!
//! The database layer is organized into:
//! - **pool**: Connection pool configuration and management
//! - **schema**: Database table models and type definitions
//! - **repos**: Repository implementations for each domain entity
//!
//! ## Usage
//!
//! ```rust,no_run
//! use masday_db::{pool, repos, DbPool};
//!
//! # #[tokio::main]
//! # async fn main() -> Result<(), Box<dyn std::error::Error>> {
//! // Create pool with retry logic
//! let pool = pool::init_pool_with_retry(3).await?;
//!
//! // Verify connectivity
//! pool::health_check(&pool).await?;
//!
//! // Use repositories (DbPool = Arc<Pool> is passed directly)
//! let workflow_repo = repos::WorkflowRepo::new(pool.clone());
//! # Ok(())
//! # }
//! ```

pub mod pool;
pub mod repos;
pub mod schema;

// Re-export commonly used types
pub use pool::DbPool;

/// Version of the masday-db crate
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Default embedding vector dimension (768 for bge-base, configurable via EMBEDDING_DIMENSIONS env)
pub fn embedding_dimensions() -> usize {
    std::env::var("EMBEDDING_DIMENSIONS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(768)
}

/// Run all database migrations.
/// Uses embedded SQL (compile-time include_str!) so it works from any installed location.
pub async fn run_migrations(pool: &DbPool) -> Result<(), String> {
    // Embedded migrations — works regardless of working directory or install path
    static MIGRATIONS: &[(&str, &str)] = &[
        (
            "001_initial_schema",
            include_str!("../migrations/001_initial_schema.sql"),
        ),
        (
            "002_add_missing_columns",
            include_str!("../migrations/002_add_missing_columns.sql"),
        ),
    ];

    let client = pool
        .get()
        .await
        .map_err(|e| format!("Failed to get DB connection: {}", e))?;

    for (name, sql) in MIGRATIONS {
        tracing::info!("Running migration: {}", name);
        client
            .batch_execute(sql)
            .await
            .map_err(|e| format!("Migration {} failed: {}", name, e))?;
    }

    tracing::info!("All {} migrations applied successfully", MIGRATIONS.len());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_defined() {
        assert!(!VERSION.is_empty());
    }

    #[test]
    fn test_db_pool_type_exported() {
        // Verify DbPool is accessible as a re-export
        let _type_check: std::marker::PhantomData<DbPool> = std::marker::PhantomData;
    }
}
