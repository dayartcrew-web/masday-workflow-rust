//! SQLite connection management for standalone stdio mode.
//!
//! Creates a single-file SQLite database at `~/.masday/data.db`.
//! Schema is auto-created on first run. Thread-safe via `Mutex`.

use rusqlite::Connection;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};
use tracing::info;

static DB: OnceLock<Mutex<Connection>> = OnceLock::new();

/// Initialize the SQLite database.
///
/// Creates `~/.masday/data.db` if it doesn't exist, then runs the embedded schema.
/// Panics if initialization fails.
pub fn init_sqlite() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let db_path = db_path()?;

    // Ensure parent directory exists
    if let Some(parent) = db_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    info!("Opening SQLite database at {}", db_path.display());

    let conn = Connection::open(&db_path)?;
    conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")?;

    // Create schema
    conn.execute_batch(crate::sqlite_schema::SCHEMA)?;

    info!("SQLite schema initialized");

    DB.set(Mutex::new(conn))
        .map_err(|_| "SQLite database already initialized".to_string())?;

    Ok(())
}

/// Get a locked reference to the SQLite connection.
///
/// Panics if `init_sqlite()` hasn't been called.
pub fn conn() -> std::sync::MutexGuard<'static, Connection> {
    DB.get()
        .expect("SQLite not initialized — call init_sqlite() first")
        .lock()
        .expect("SQLite connection lock poisoned")
}

/// Get the database file path.
///
/// Priority:
/// 1. `MASDAY_SQLITE_PATH` env var
/// 2. `~/.masday/data.db`
fn db_path() -> Result<PathBuf, Box<dyn std::error::Error + Send + Sync>> {
    if let Ok(path) = std::env::var("MASDAY_SQLITE_PATH") {
        return Ok(PathBuf::from(path));
    }

    let home = dirs_home()?;
    Ok(home.join(".masday").join("data.db"))
}

fn dirs_home() -> Result<PathBuf, Box<dyn std::error::Error + Send + Sync>> {
    // Try HOME env var first (Unix)
    if let Ok(home) = std::env::var("HOME") {
        return Ok(PathBuf::from(home));
    }
    // Try USERPROFILE (Windows)
    if let Ok(home) = std::env::var("USERPROFILE") {
        return Ok(PathBuf::from(home));
    }
    Err("Cannot determine home directory: set HOME or MASYDAY_SQLITE_PATH".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_db_path_from_env() {
        std::env::set_var("MASDAY_SQLITE_PATH", "/tmp/test-masday.db");
        let path = db_path().unwrap();
        assert_eq!(path, PathBuf::from("/tmp/test-masday.db"));
        std::env::remove_var("MASDAY_SQLITE_PATH");
    }

    #[test]
    fn test_db_path_default() {
        std::env::remove_var("MASDAY_SQLITE_PATH");
        let path = db_path().unwrap();
        assert!(path.to_string_lossy().ends_with("data.db"));
    }
}
