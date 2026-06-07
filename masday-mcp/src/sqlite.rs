//! SQLite connection management for standalone stdio mode.
//!
//! Creates a single-file SQLite database at `~/.masday/data.db`.
//! Schema is auto-created on first run. Thread-safe via `Mutex`.

use rusqlite::Connection;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};
use tracing::info;

static DB: OnceLock<Mutex<Connection>> = OnceLock::new();

/// Backfill embeddings for existing memories that don't have them yet.
///
/// This function is called during SQLite initialization to ensure all memories
/// have embeddings for semantic search. It processes memories in batches of 50.
fn backfill_embeddings(conn: &Connection) {
    use crate::embedding::{text_to_vector, vector_to_blob};

    // Count memories without embeddings
    let count: i64 = match conn.query_row(
        "SELECT COUNT(*) FROM memories WHERE embedding IS NULL",
        [],
        |row| row.get(0),
    ) {
        Ok(c) => c,
        Err(e) => {
            info!("Failed to count memories without embeddings: {}", e);
            return;
        }
    };

    if count == 0 {
        info!("No memories require embedding backfill");
        return;
    }

    info!("Backfilling {} embeddings...", count);

    // Process in batches
    let batch_size = 50;
    let mut processed = 0;

    loop {
        // Fetch batch of memories without embeddings
        let mut stmt = match conn
            .prepare("SELECT id, summary, content FROM memories WHERE embedding IS NULL LIMIT ?1")
        {
            Ok(s) => s,
            Err(e) => {
                info!("Failed to prepare backfill query: {}", e);
                break;
            }
        };

        let rows: Vec<(String, String, String)> = stmt
            .query_map([batch_size], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                ))
            })
            .and_then(|mapped| mapped.collect::<Result<Vec<_>, _>>())
            .unwrap_or_default();

        if rows.is_empty() {
            break;
        }

        // Generate and store embeddings for each memory
        for (id, summary, content) in rows {
            let embedding_text = format!("{} {}", summary, content);
            let embedding_vector = text_to_vector(&embedding_text);
            let embedding_blob = vector_to_blob(&embedding_vector);

            if let Err(e) = conn.execute(
                "UPDATE memories SET embedding = ?1 WHERE id = ?2",
                [
                    &embedding_blob as &dyn rusqlite::ToSql,
                    &id as &dyn rusqlite::ToSql,
                ],
            ) {
                info!("Failed to update embedding for memory {}: {}", id, e);
            } else {
                processed += 1;
            }
        }

        if processed < batch_size {
            break;
        }
    }

    info!("Backfilled {} embeddings", processed);
}

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

    // Run migrations for existing databases
    run_migrations(&conn)?;

    // Backfill embeddings for existing memories
    backfill_embeddings(&conn);

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

/// Try to execute a simple query to verify database connectivity.
///
/// Returns Ok(()) if the connection is working, Err with description otherwise.
pub fn try_connection() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let conn_guard = conn();
    conn_guard
        .query_row("SELECT 1", [], |_| Ok(()))
        .map_err(|e| e.to_string())?;
    Ok(())
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

/// Run migrations to update existing databases.
///
/// This function handles schema migrations for databases that were created
/// with older versions of the schema. It uses safe checks to avoid errors
/// when columns already exist.
fn run_migrations(conn: &Connection) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Migration 001: Add embedding column to memories table
    // Check if the column exists first
    let has_embedding: Result<i64, _> = conn.query_row(
        "SELECT COUNT(*) FROM pragma_table_info('memories') WHERE name='embedding'",
        [],
        |row| row.get(0),
    );

    match has_embedding {
        Ok(0) => {
            // Column doesn't exist, add it
            info!("Adding embedding column to memories table");
            conn.execute("ALTER TABLE memories ADD COLUMN embedding BLOB", [])?;
            info!("Migration 001 complete: embedding column added");
        }
        Ok(_) => {
            // Column already exists, skip
            info!("Migration 001 skipped: embedding column already exists");
        }
        Err(_) => {
            // Query failed (table might not exist or other issue), try to add column
            info!("Checking embedding column failed, attempting to add it");
            match conn.execute("ALTER TABLE memories ADD COLUMN embedding BLOB", []) {
                Ok(_) => {
                    info!("Migration 001 complete: embedding column added");
                }
                Err(e) => {
                    // Column likely already exists or table doesn't exist
                    info!("Migration 001 skipped: column may already exist - {}", e);
                }
            }
        }
    }

    Ok(())
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

    #[test]
    fn test_schema_has_embedding_column() {
        // Create a temporary in-memory database
        let conn = Connection::open_in_memory().unwrap();

        // Execute the schema
        conn.execute_batch(crate::sqlite_schema::SCHEMA).unwrap();

        // Check that the embedding column exists
        let has_embedding: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM pragma_table_info('memories') WHERE name='embedding'",
                [],
                |row| row.get(0),
            )
            .unwrap();

        assert_eq!(
            has_embedding, 1,
            "embedding column should exist in memories table"
        );

        // Verify the column type is BLOB
        let column_type: String = conn
            .query_row(
                "SELECT type FROM pragma_table_info('memories') WHERE name='embedding'",
                [],
                |row| row.get(0),
            )
            .unwrap();

        assert_eq!(column_type, "BLOB", "embedding column should be BLOB type");

        // Verify embedding column is nullable (notnull should be 0)
        let not_null: i64 = conn
            .query_row(
                "SELECT \"notnull\" FROM pragma_table_info('memories') WHERE name='embedding'",
                [],
                |row| row.get(0),
            )
            .unwrap();

        assert_eq!(
            not_null, 0,
            "embedding column should be nullable (notnull=0)"
        );
    }

    #[test]
    fn test_migration_adds_embedding_to_existing_database() {
        // Create a temporary in-memory database without the embedding column
        let conn = Connection::open_in_memory().unwrap();

        // Create the memories table without embedding (simulating old schema)
        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS workflows (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                description TEXT,
                status TEXT NOT NULL DEFAULT 'INIT',
                project_path TEXT,
                trace_id TEXT,
                metadata TEXT DEFAULT '{}',
                created_at TEXT NOT NULL DEFAULT (datetime('now')),
                updated_at TEXT NOT NULL DEFAULT (datetime('now'))
            );

            CREATE TABLE IF NOT EXISTS plans (
                id TEXT PRIMARY KEY,
                workflow_id TEXT NOT NULL REFERENCES workflows(id) ON DELETE CASCADE,
                version INTEGER NOT NULL,
                status TEXT NOT NULL DEFAULT 'PENDING',
                summary TEXT NOT NULL,
                content TEXT NOT NULL DEFAULT '{}',
                created_by_agent TEXT NOT NULL,
                created_at TEXT NOT NULL DEFAULT (datetime('now'))
            );

            CREATE TABLE IF NOT EXISTS tasks (
                id TEXT PRIMARY KEY,
                workflow_id TEXT NOT NULL REFERENCES workflows(id) ON DELETE CASCADE,
                plan_id TEXT NOT NULL REFERENCES plans(id) ON DELETE CASCADE,
                title TEXT NOT NULL,
                status TEXT NOT NULL DEFAULT 'PENDING',
                priority TEXT,
                owner_agent TEXT,
                skill TEXT,
                description TEXT,
                dependencies TEXT,
                acceptance_criteria TEXT,
                required_context TEXT,
                verification_steps TEXT,
                context_fingerprint TEXT,
                progress_percent INTEGER,
                requires_tdd INTEGER DEFAULT 0,
                input TEXT,
                result TEXT,
                test_evidence TEXT,
                metadata TEXT,
                created_at TEXT NOT NULL DEFAULT (datetime('now')),
                started_at TEXT,
                completed_at TEXT,
                updated_at TEXT NOT NULL DEFAULT (datetime('now'))
            );

            CREATE TABLE IF NOT EXISTS memories (
                id TEXT PRIMARY KEY,
                workflow_id TEXT REFERENCES workflows(id) ON DELETE CASCADE,
                task_id TEXT REFERENCES tasks(id) ON DELETE CASCADE,
                memory_type TEXT NOT NULL,
                summary TEXT NOT NULL DEFAULT '',
                content TEXT NOT NULL,
                importance_score REAL DEFAULT 0.5,
                created_by_agent TEXT NOT NULL,
                tags TEXT DEFAULT '[]',
                source TEXT,
                created_at TEXT NOT NULL DEFAULT (datetime('now')),
                updated_at TEXT NOT NULL DEFAULT (datetime('now')),
                accessed_at TEXT,
                access_count INTEGER DEFAULT 0,
                version INTEGER DEFAULT 1
            );
        "#,
        )
        .unwrap();

        // Verify embedding column doesn't exist initially
        let has_embedding_before: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM pragma_table_info('memories') WHERE name='embedding'",
                [],
                |row| row.get(0),
            )
            .unwrap();

        assert_eq!(
            has_embedding_before, 0,
            "embedding column should not exist initially"
        );

        // Run migrations
        run_migrations(&conn).unwrap();

        // Verify embedding column now exists
        let has_embedding_after: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM pragma_table_info('memories') WHERE name='embedding'",
                [],
                |row| row.get(0),
            )
            .unwrap();

        assert_eq!(
            has_embedding_after, 1,
            "embedding column should exist after migration"
        );

        // Create parent records for foreign key constraints
        use uuid::Uuid;

        let workflow_id = Uuid::new_v4().to_string();
        let plan_id = Uuid::new_v4().to_string();
        let task_id = Uuid::new_v4().to_string();

        // Insert workflow
        conn.execute(
            "INSERT INTO workflows (id, name, status) VALUES (?1, ?2, ?3)",
            [
                &workflow_id,
                &"Test Workflow".to_string(),
                &"INIT".to_string(),
            ],
        )
        .unwrap();

        // Insert plan
        conn.execute(
            "INSERT INTO plans (id, workflow_id, version, status, summary, content, created_by_agent) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            [&plan_id as &dyn rusqlite::ToSql, &workflow_id, &1, &"PENDING".to_string(), &"Test Plan".to_string(), &"{}".to_string(), &"test-agent".to_string()],
        ).unwrap();

        // Insert task
        conn.execute(
            "INSERT INTO tasks (id, workflow_id, plan_id, title, status) VALUES (?1, ?2, ?3, ?4, ?5)",
            [&task_id, &workflow_id, &plan_id, &"Test Task".to_string(), &"PENDING".to_string()],
        ).unwrap();

        // Verify we can insert records with and without embeddings
        let memory_id = Uuid::new_v4().to_string();

        // Insert a memory without embedding (should work)
        conn.execute(
            "INSERT INTO memories (id, workflow_id, task_id, memory_type, summary, content, created_by_agent)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            [
                &memory_id, &workflow_id, &task_id, &"test".to_string(), &"Test memory".to_string(), &"Content".to_string(), &"test-agent".to_string()
            ],
        ).unwrap();

        // Verify embedding is NULL for the inserted record
        let embedding_value: Option<Vec<u8>> = conn
            .query_row(
                "SELECT embedding FROM memories WHERE id = ?1",
                [&memory_id],
                |row| row.get(0),
            )
            .unwrap();

        assert!(
            embedding_value.is_none(),
            "embedding should be NULL for newly inserted record"
        );

        // Insert a memory with embedding (should work)
        let memory_id2 = Uuid::new_v4().to_string();
        let test_embedding: Vec<u8> = vec![0u8; 3072]; // Simulated f32[768] = 3072 bytes

        conn.execute(
            "INSERT INTO memories (id, workflow_id, task_id, memory_type, summary, content, created_by_agent, embedding)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            [
                &memory_id2 as &dyn rusqlite::ToSql, &workflow_id, &task_id, &"test".to_string(), &"Test memory with embedding".to_string(), &"Content".to_string(), &"test-agent".to_string(), &test_embedding
            ],
        ).unwrap();

        // Verify embedding was stored correctly
        let retrieved_embedding: Option<Vec<u8>> = conn
            .query_row(
                "SELECT embedding FROM memories WHERE id = ?1",
                [&memory_id2],
                |row| row.get(0),
            )
            .unwrap();

        assert!(
            retrieved_embedding.is_some(),
            "embedding should exist for memory with embedding"
        );

        // Verify the embedding size is correct
        let retrieved_vec = retrieved_embedding.unwrap();
        assert_eq!(retrieved_vec.len(), 3072, "embedding should be 3072 bytes");
    }

    #[test]
    fn test_migration_idempotent() {
        // Create a temporary in-memory database
        let conn = Connection::open_in_memory().unwrap();

        // Execute the full schema (includes embedding column)
        conn.execute_batch(crate::sqlite_schema::SCHEMA).unwrap();

        // Run migrations - should skip since column already exists
        run_migrations(&conn).unwrap();

        // Verify no errors occurred
        // Verify embedding column still exists
        let has_embedding: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM pragma_table_info('memories') WHERE name='embedding'",
                [],
                |row| row.get(0),
            )
            .unwrap();

        assert_eq!(
            has_embedding, 1,
            "embedding column should still exist after re-running migration"
        );
    }

    #[test]
    fn test_backfill_embeddings_no_memories() {
        // Create a temporary in-memory database
        let conn = Connection::open_in_memory().unwrap();

        // Execute the full schema
        conn.execute_batch(crate::sqlite_schema::SCHEMA).unwrap();

        // Run backfill on empty database
        backfill_embeddings(&conn);

        // Verify no errors occurred
        // Verify no rows were affected
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM memories", [], |row| row.get(0))
            .unwrap();

        assert_eq!(
            count, 0,
            "No memories should exist after backfill on empty database"
        );
    }

    #[test]
    fn test_backfill_embeddings_with_memories() {
        use uuid::Uuid;

        // Create a temporary in-memory database
        let conn = Connection::open_in_memory().unwrap();

        // Execute the full schema
        conn.execute_batch(crate::sqlite_schema::SCHEMA).unwrap();

        // Create parent records
        let workflow_id = Uuid::new_v4().to_string();
        conn.execute(
            "INSERT INTO workflows (id, name, status) VALUES (?1, ?2, ?3)",
            [
                &workflow_id,
                &"Test Workflow".to_string(),
                &"INIT".to_string(),
            ],
        )
        .unwrap();

        // Insert memories without embeddings
        for i in 1..=5 {
            let memory_id = Uuid::new_v4().to_string();
            conn.execute(
                "INSERT INTO memories (id, workflow_id, memory_type, summary, content, created_by_agent)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                [
                    &memory_id,
                    &workflow_id,
                    &"test".to_string(),
                    &format!("Summary {}", i),
                    &format!("Content for memory {}", i),
                    &"test-agent".to_string(),
                ],
            ).unwrap();
        }

        // Verify embeddings are NULL initially
        let null_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM memories WHERE embedding IS NULL",
                [],
                |row| row.get(0),
            )
            .unwrap();

        assert_eq!(
            null_count, 5,
            "All memories should have NULL embeddings initially"
        );

        // Run backfill
        backfill_embeddings(&conn);

        // Verify all memories now have embeddings
        let null_count_after: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM memories WHERE embedding IS NULL",
                [],
                |row| row.get(0),
            )
            .unwrap();

        assert_eq!(
            null_count_after, 0,
            "No memories should have NULL embeddings after backfill"
        );

        // Verify embeddings are the correct size (768 f32 = 3072 bytes)
        let embedding_sizes: Vec<i64> = conn
            .prepare("SELECT length(embedding) FROM memories WHERE embedding IS NOT NULL")
            .unwrap()
            .query_map([], |row| row.get(0))
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();

        for size in embedding_sizes {
            assert_eq!(size, 3072, "Each embedding should be 3072 bytes (768 f32)");
        }
    }

    #[test]
    fn test_backfill_embeddings_idempotent() {
        use uuid::Uuid;

        // Create a temporary in-memory database
        let conn = Connection::open_in_memory().unwrap();

        // Execute the full schema
        conn.execute_batch(crate::sqlite_schema::SCHEMA).unwrap();

        // Create parent records
        let workflow_id = Uuid::new_v4().to_string();
        conn.execute(
            "INSERT INTO workflows (id, name, status) VALUES (?1, ?2, ?3)",
            [
                &workflow_id,
                &"Test Workflow".to_string(),
                &"INIT".to_string(),
            ],
        )
        .unwrap();

        // Insert memories without embeddings
        let memory_id = Uuid::new_v4().to_string();
        conn.execute(
            "INSERT INTO memories (id, workflow_id, memory_type, summary, content, created_by_agent)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            [
                &memory_id,
                &workflow_id,
                &"test".to_string(),
                &"Test summary".to_string(),
                &"Test content".to_string(),
                &"test-agent".to_string(),
            ],
        ).unwrap();

        // First backfill
        backfill_embeddings(&conn);

        // Get the embedding after first backfill
        let embedding_after_first: Option<Vec<u8>> = conn
            .query_row(
                "SELECT embedding FROM memories WHERE id = ?1",
                [&memory_id],
                |row| row.get(0),
            )
            .unwrap();

        assert!(
            embedding_after_first.is_some(),
            "Embedding should exist after first backfill"
        );

        // Second backfill (should be idempotent - no changes)
        backfill_embeddings(&conn);

        // Get the embedding after second backfill
        let embedding_after_second: Option<Vec<u8>> = conn
            .query_row(
                "SELECT embedding FROM memories WHERE id = ?1",
                [&memory_id],
                |row| row.get(0),
            )
            .unwrap();

        assert!(
            embedding_after_second.is_some(),
            "Embedding should still exist after second backfill"
        );

        // Verify embeddings are identical
        assert_eq!(
            embedding_after_first.unwrap(),
            embedding_after_second.unwrap(),
            "Embedding should not change on second backfill"
        );
    }

    #[test]
    fn test_backfill_embeddings_partial() {
        use uuid::Uuid;

        // Create a temporary in-memory database
        let conn = Connection::open_in_memory().unwrap();

        // Execute the full schema
        conn.execute_batch(crate::sqlite_schema::SCHEMA).unwrap();

        // Create parent records
        let workflow_id = Uuid::new_v4().to_string();
        conn.execute(
            "INSERT INTO workflows (id, name, status) VALUES (?1, ?2, ?3)",
            [
                &workflow_id,
                &"Test Workflow".to_string(),
                &"INIT".to_string(),
            ],
        )
        .unwrap();

        // Insert some memories with embeddings, some without
        for i in 1..=3 {
            let memory_id = Uuid::new_v4().to_string();
            conn.execute(
                "INSERT INTO memories (id, workflow_id, memory_type, summary, content, created_by_agent)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                [
                    &memory_id,
                    &workflow_id,
                    &"test".to_string(),
                    &format!("Summary {}", i),
                    &format!("Content {}", i),
                    &"test-agent".to_string(),
                ],
            ).unwrap();
        }

        // Insert one memory with embedding
        let memory_with_embedding = Uuid::new_v4().to_string();
        let test_embedding: Vec<u8> = vec![0u8; 3072];
        conn.execute(
            "INSERT INTO memories (id, workflow_id, memory_type, summary, content, created_by_agent, embedding)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            [
                &memory_with_embedding as &dyn rusqlite::ToSql,
                &workflow_id as &dyn rusqlite::ToSql,
                &"test".to_string() as &dyn rusqlite::ToSql,
                &"Has embedding".to_string() as &dyn rusqlite::ToSql,
                &"Already has embedding".to_string() as &dyn rusqlite::ToSql,
                &"test-agent".to_string() as &dyn rusqlite::ToSql,
                &test_embedding as &dyn rusqlite::ToSql,
            ],
        ).unwrap();

        // Verify initial state
        let null_count_before: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM memories WHERE embedding IS NULL",
                [],
                |row| row.get(0),
            )
            .unwrap();

        assert_eq!(
            null_count_before, 3,
            "Three memories should have NULL embeddings initially"
        );

        // Run backfill
        backfill_embeddings(&conn);

        // Verify only NULL embeddings were filled
        let null_count_after: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM memories WHERE embedding IS NULL",
                [],
                |row| row.get(0),
            )
            .unwrap();

        assert_eq!(
            null_count_after, 0,
            "No memories should have NULL embeddings after backfill"
        );

        // Verify total memory count
        let total_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM memories", [], |row| row.get(0))
            .unwrap();

        assert_eq!(total_count, 4, "Total memory count should remain unchanged");

        // Verify the original embedding was not overwritten
        let original_embedding: Option<Vec<u8>> = conn
            .query_row(
                "SELECT embedding FROM memories WHERE id = ?1",
                [&memory_with_embedding],
                |row| row.get(0),
            )
            .unwrap();

        assert!(
            original_embedding.is_some(),
            "Original embedding should still exist"
        );
        assert_eq!(
            original_embedding.unwrap().len(),
            3072,
            "Original embedding should be unchanged"
        );
    }
}
