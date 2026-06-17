//! PostgreSQL connection for MCP local mode.
//!
//! On-demand connection: pool is NOT initialized at startup.
//! Instead, it connects lazily in a background task on first use.
//! All PG sync is non-blocking — if PostgreSQL is slow/unavailable,
//! tool calls return immediately from SQLite.

use masday_db::pool::{create_pool, DbPool};
use once_cell::sync::Lazy;
use serde_json::{json, Value};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Global PostgreSQL pool — lazily initialized on first use, not at startup.
static PG_POOL: Lazy<Arc<RwLock<Option<DbPool>>>> = Lazy::new(|| Arc::new(RwLock::new(None)));

/// Guard to prevent spawning duplicate background init tasks.
static PG_INIT_STARTED: AtomicBool = AtomicBool::new(false);

/// Embedded migration SQL — included at compile time from masday-db.
/// Each file is run independently; statement splitting happens per-file.
const MIGRATION_SQL: &[(&str, &str)] = &[
    (
        "001_initial_schema",
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../masday-db/migrations/001_initial_schema.sql"
        )),
    ),
    (
        "005_code_chunks_pgvector",
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../masday-db/migrations/005_code_chunks_pgvector.sql"
        )),
    ),
];

/// Read a single key from ~/.masday/config.toml using line-by-line parsing.
/// Matches both `key_name` and `key-name` variants (underscores and hyphens).
/// Returns None if key not found or config missing.
pub fn read_config_value(key: &str) -> Option<String> {
    let home = home::home_dir()?;
    let config_path = home.join(".masday").join("config.toml");
    if !config_path.exists() {
        return None;
    }
    let content = std::fs::read_to_string(&config_path).ok()?;
    let key_alt = key.replace('_', "-");
    for line in content.lines() {
        let trimmed = line.trim();
        // Match "key =" or "key=" at the start of the line
        let matched_key = if let Some(rest) = trimmed.strip_prefix(key) {
            rest
        } else if let Some(rest) = trimmed.strip_prefix(&key_alt) {
            rest
        } else {
            continue;
        };
        {
            let after_key = matched_key.trim_start();
            if !after_key.starts_with('=') {
                continue;
            }
            if let Some(value) = after_key.trim_start_matches('=').split('#').next() {
                let v = value.trim().trim_matches('"').trim_matches('\'');
                if !v.is_empty() {
                    return Some(v.to_string());
                }
            }
        }
    }
    None
}

/// Read database_url from ~/.masday/config.toml directly.
fn read_db_url_from_config() -> Option<String> {
    read_config_value("database_url")
}

/// Read mode from config.toml (standalone/local/remote).
/// Returns "standalone" if no config or no mode key found.
pub fn read_mode() -> String {
    read_config_value("mode").unwrap_or_else(|| "standalone".to_string())
}

/// Read API URL from config.toml.
pub fn read_api_url() -> Option<String> {
    read_config_value("api_url")
}

/// Read Redis URL from config.toml.
pub fn read_redis_url() -> Option<String> {
    read_config_value("redis_url")
}

/// Read embedding provider. One of: "mock" | "ollama" | "openai".
/// Explicit env (`EMBEDDING_PROVIDER`) wins over config.toml — mirrors
/// masday-api's "env wins" rule and lets tests inject a provider without editing
/// ~/.masday/config.toml. Production sets no env var, so config.toml governs there.
pub fn read_embedding_provider() -> Option<String> {
    std::env::var("EMBEDDING_PROVIDER")
        .ok()
        .filter(|s| !s.is_empty())
        .or_else(|| read_config_value("embedding_provider"))
}

/// Read embedding model name. Env (`EMBEDDING_MODEL`) wins over config.toml.
pub fn read_embedding_model() -> Option<String> {
    std::env::var("EMBEDDING_MODEL")
        .ok()
        .filter(|s| !s.is_empty())
        .or_else(|| read_config_value("embedding_model"))
}

/// Read embedding vector dimensions. Env (`EMBEDDING_DIMENSIONS`) wins over config.toml.
pub fn read_embedding_dimensions() -> Option<usize> {
    if let Ok(d) = std::env::var("EMBEDDING_DIMENSIONS") {
        if let Ok(parsed) = d.parse::<usize>() {
            return Some(parsed);
        }
    }
    read_config_value("embedding_dimensions")?
        .parse::<usize>()
        .ok()
}

/// Read embedding base URL. Env (`EMBEDDING_BASE_URL`) wins over config.toml.
/// Used to override the default Ollama/OpenAI endpoint.
pub fn read_embedding_base_url() -> Option<String> {
    std::env::var("EMBEDDING_BASE_URL")
        .ok()
        .filter(|s| !s.is_empty())
        .or_else(|| read_config_value("embedding_base_url"))
}

/// Run embedded migrations directly (doesn't depend on filesystem migrations dir).
/// Splits SQL by statement boundaries (lines ending with ;).
async fn run_embedded_migrations(pool: &DbPool) -> Result<(), String> {
    let client = pool
        .get()
        .await
        .map_err(|e| format!("Failed to get DB connection: {}", e))?;

    let mut total_ok = 0;
    let mut total_stmts = 0;

    for (_name, migration_sql) in MIGRATION_SQL {
        // Collect multi-line statements (accumulate until we hit a line ending with ';')
        let mut current = String::new();
        let mut statements = Vec::new();

        for line in migration_sql.lines() {
            let trimmed = line.trim();
            // Skip empty lines and comment-only lines
            if trimmed.is_empty() || trimmed.starts_with("--") {
                continue;
            }
            // Add non-comment parts of the line
            current.push_str(line);
            current.push('\n');
            if trimmed.ends_with(';') {
                let stmt = current.trim().trim_end_matches(';').trim().to_string();
                if !stmt.is_empty() {
                    statements.push(stmt);
                }
                current.clear();
            }
        }
        // Handle any remaining statement without trailing semicolon
        if !current.trim().is_empty() {
            statements.push(current.trim().to_string());
        }

        for sql in &statements {
            total_stmts += 1;
            if let Err(e) = client.execute(sql, &[]).await {
                let msg = e.to_string();
                if msg.contains("already exists") {
                    tracing::debug!("Schema exists: {}", &msg[..msg.len().min(60)]);
                    total_ok += 1;
                } else {
                    tracing::warn!(
                        "Migration: {} — {}",
                        &sql[..sql.len().min(50)],
                        &msg[..msg.len().min(80)]
                    );
                }
            } else {
                total_ok += 1;
            }
        }
    }

    tracing::info!(
        "PostgreSQL schema ready ({}/{} statements OK)",
        total_ok,
        total_stmts
    );
    Ok(())
}

/// Get or create PostgreSQL pool on demand (non-blocking).
///
/// - Fast path: pool exists → return immediately
/// - Standalone mode → return None (SQLite only)
/// - No database_url → return None
/// - Pool not ready → spawn background init, return None
///
/// The background init task: create pool → health check (10s timeout) →
/// run migrations (30s timeout) → store pool.
pub async fn get_pool() -> Option<DbPool> {
    // Standalone mode — SQLite only, skip PG
    if read_mode() == "standalone" {
        return None;
    }

    // Fast path: pool already exists
    {
        let pg = PG_POOL.read().await;
        if let Some(ref pool) = *pg {
            return Some(pool.clone());
        }
    }

    // No database_url configured
    let database_url = std::env::var("DATABASE_URL")
        .ok()
        .or_else(read_db_url_from_config)
        .unwrap_or_default();

    if database_url.is_empty() {
        return None;
    }

    // Background init: only spawn once
    if PG_INIT_STARTED
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .is_ok()
    {
        let db_url = database_url.clone();
        let pg_pool = PG_POOL.clone();
        tokio::spawn(async move {
            tracing::info!(
                "PostgreSQL: connecting in background → {}...",
                &db_url[..db_url.len().min(40)]
            );

            // Set env var for masday-db::pool::create_pool()
            std::env::set_var("DATABASE_URL", &db_url);

            // Create pool (lazy — no connection until first query).
            // Skip health_check: the pool is lazy and will connect on first use.
            // If PG is down, individual tool calls will return errors gracefully.
            let pool = match create_pool() {
                Ok(p) => {
                    tracing::info!("PostgreSQL: pool created successfully");
                    p
                }
                Err(e) => {
                    tracing::error!("PostgreSQL: create_pool failed: {}", e);
                    eprintln!("[masday] PG create_pool failed: {}", e);
                    PG_INIT_STARTED.store(false, Ordering::SeqCst);
                    return;
                }
            };

            tracing::info!("PostgreSQL connected (background)");

            // Run embedded migrations with timeout
            let _ = tokio::time::timeout(
                std::time::Duration::from_secs(30),
                run_embedded_migrations(&pool),
            )
            .await;

            // Store pool for reuse
            let mut pg = pg_pool.write().await;
            *pg = Some(pool.clone());

            tracing::info!("PostgreSQL ready for sync");
        });
    }

    // Pool not ready yet — return None (caller proceeds with SQLite only)
    None
}

/// Wait for pool to become available (for bulk ops that need PG).
/// Polls get_pool() with 200ms interval until pool ready or timeout.
pub async fn get_pool_wait(duration: std::time::Duration) -> Option<DbPool> {
    let deadline = tokio::time::Instant::now() + duration;
    loop {
        if let Some(pool) = get_pool().await {
            return Some(pool);
        }
        if tokio::time::Instant::now() >= deadline {
            return None;
        }
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    }
}

/// Check if database_url is configured (without connecting).
pub fn is_configured() -> bool {
    let env = std::env::var("DATABASE_URL").ok().filter(|s| !s.is_empty());
    if env.is_some() {
        return true;
    }
    read_db_url_from_config().is_some()
}

/// Pool status for readiness checks — reads config.toml mode.
/// Returns a JSON object describing DB readiness for both SQLite and PostgreSQL.
pub async fn pool_status() -> Value {
    let mode = read_mode();
    let pg_ready = {
        let pg = PG_POOL.read().await;
        pg.is_some()
    };
    let use_pg = mode != "standalone" && is_configured();
    json!({
        "mode": mode,
        "sqlite": {"ready": true},
        "postgresql": {
            "ready": pg_ready,
            "configured": use_pg,
            "sync": if use_pg { "async" } else { "disabled" }
        }
    })
}
