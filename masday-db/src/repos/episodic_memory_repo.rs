//! Episodic memory repository
//!
//! Table names are snake_case: "episodic_memories"
//! Column names are snake_case: "session_id", "sequence_order", etc.

use crate::pool::DbPool;
use crate::schema::{EpisodicMemory, NewEpisodicMemory};
use masday_core::{AppError, Result};

pub struct EpisodicMemoryRepo {
    pool: DbPool,
}

impl EpisodicMemoryRepo {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    /// Create a new episodic memory entry
    pub async fn create(&self, memory: &NewEpisodicMemory) -> Result<EpisodicMemory> {
        let client = self
            .pool
            .get()
            .await
            .map_err(|e| AppError::Database(format!("Failed to get connection: {}", e)))?;

        let id = uuid::Uuid::new_v4().to_string();
        let now: chrono::NaiveDateTime = chrono::Utc::now().naive_utc();

        let query = r#"
            INSERT INTO episodic_memories (
                id, session_id, role, content, sequence_order, created_at
            )
            VALUES ($1, $2, $3, $4, $5, $6)
            RETURNING *
        "#;

        let row = client
            .query_one(
                query,
                &[
                    &id,
                    &memory.session_id,
                    &memory.role,
                    &memory.content,
                    &memory.sequence_order,
                    &now,
                ],
            )
            .await
            .map_err(|e| AppError::Database(format!("Failed to create episodic memory: {}", e)))?;

        Ok(EpisodicMemory::from_row(&row))
    }

    /// Get episodic memory by ID
    pub async fn get_by_id(&self, id: &str) -> Result<EpisodicMemory> {
        let client = self
            .pool
            .get()
            .await
            .map_err(|e| AppError::Database(format!("Failed to get connection: {}", e)))?;

        let query = r#"SELECT * FROM episodic_memories WHERE id = $1"#;
        let row = client
            .query_one(query, &[&id])
            .await
            .map_err(|_e| AppError::not_found("EpisodicMemory", id))?;

        Ok(EpisodicMemory::from_row(&row))
    }

    /// List all episodic memories for a session
    pub async fn list_by_session(&self, session_id: &str) -> Result<Vec<EpisodicMemory>> {
        let client = self
            .pool
            .get()
            .await
            .map_err(|e| AppError::Database(format!("Failed to get connection: {}", e)))?;

        let query = r#"
            SELECT * FROM episodic_memories
            WHERE session_id = $1
            ORDER BY sequence_order ASC, created_at ASC
        "#;
        let rows = client
            .query(query, &[&session_id])
            .await
            .map_err(|e| AppError::Database(format!("Failed to list episodic memories: {}", e)))?;

        Ok(rows.iter().map(EpisodicMemory::from_row).collect())
    }

    /// List all episodic memories (with optional limit)
    pub async fn list_all(&self, limit: Option<i64>) -> Result<Vec<EpisodicMemory>> {
        let client = self
            .pool
            .get()
            .await
            .map_err(|e| AppError::Database(format!("Failed to get connection: {}", e)))?;

        let capped = limit.unwrap_or(100).min(1000);
        let query = r#"SELECT * FROM episodic_memories ORDER BY created_at DESC LIMIT $1"#;
        let rows = client.query(query, &[&capped]).await.map_err(|e| {
            AppError::Database(format!("Failed to list all episodic memories: {}", e))
        })?;

        Ok(rows.iter().map(EpisodicMemory::from_row).collect())
    }

    /// Delete an episodic memory entry
    pub async fn delete(&self, id: &str) -> Result<bool> {
        let client = self
            .pool
            .get()
            .await
            .map_err(|e| AppError::Database(format!("Failed to get connection: {}", e)))?;

        let result = client
            .execute(r#"DELETE FROM episodic_memories WHERE id = $1"#, &[&id])
            .await
            .map_err(|e| AppError::Database(format!("Failed to delete episodic memory: {}", e)))?;

        Ok(result > 0)
    }

    /// Delete all episodic memories for a session
    pub async fn delete_by_session(&self, session_id: &str) -> Result<u64> {
        let client = self
            .pool
            .get()
            .await
            .map_err(|e| AppError::Database(format!("Failed to get connection: {}", e)))?;

        let result = client
            .execute(
                r#"DELETE FROM episodic_memories WHERE session_id = $1"#,
                &[&session_id],
            )
            .await
            .map_err(|e| {
                AppError::Database(format!("Failed to delete session episodic memories: {}", e))
            })?;

        Ok(result)
    }
}
