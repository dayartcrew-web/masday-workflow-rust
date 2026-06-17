//! Code chunk repository — chunk-level pgvector code search.
//!
//! Backs the `code_chunks` table (migration 005). Chunks are produced by the MCP
//! indexer (reusing `code_index.rs` chunking) and embedded via Ollama reading
//! `~/.masday/config.toml`. Search uses pgvector cosine distance (`<=>`).
//!
//! All vector params use `pgvector::Vector` (the `pgvector` type is registered on
//! the pool by `masday-db::pool`). Any error is surfaced as `AppError::Database`
//! so callers can fall back to the SQLite feature-hash path.

use crate::pool::DbPool;
use crate::schema::{CodeChunkResult, NewCodeChunk};
use masday_core::{AppError, Result};
use pgvector::Vector;
use tracing::debug;

pub struct CodeChunkRepo {
    pool: DbPool,
}

impl CodeChunkRepo {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    /// Upsert a single chunk. Embedding may be `None` if Ollama failed for this
    /// chunk (the row is still stored so it can serve text fallback; it is simply
    /// excluded from vector search until re-indexed with an embedding).
    pub async fn upsert_chunk(&self, chunk: &NewCodeChunk) -> Result<()> {
        let client = self
            .pool
            .get()
            .await
            .map_err(|e| AppError::Database(format!("Failed to get connection: {}", e)))?;

        let pgvec = chunk.embedding.as_ref().map(|v| Vector::from(v.clone()));

        let query = r#"
            INSERT INTO code_chunks (
                project_path, file_path, language, chunk_type, name,
                start_line, end_line, content, content_hash, embedding
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            ON CONFLICT (project_path, file_path, start_line) DO UPDATE
            SET language     = EXCLUDED.language,
                chunk_type   = EXCLUDED.chunk_type,
                name         = EXCLUDED.name,
                end_line     = EXCLUDED.end_line,
                content      = EXCLUDED.content,
                content_hash = EXCLUDED.content_hash,
                embedding    = EXCLUDED.embedding,
                indexed_at   = NOW()
        "#;

        client
            .execute(
                query,
                &[
                    &chunk.project_path,
                    &chunk.file_path,
                    &chunk.language,
                    &chunk.chunk_type,
                    &chunk.name,
                    &chunk.start_line,
                    &chunk.end_line,
                    &chunk.content,
                    &chunk.content_hash,
                    &pgvec,
                ],
            )
            .await
            .map_err(|e| AppError::Database(format!("Failed to upsert code chunk: {}", e)))?;

        Ok(())
    }

    /// Count all chunks for a project (any embedding state).
    pub async fn count_for_project(&self, project_path: &str) -> Result<i64> {
        let client = self
            .pool
            .get()
            .await
            .map_err(|e| AppError::Database(format!("Failed to get connection: {}", e)))?;

        let row = client
            .query_one(
                "SELECT COUNT(*) as count FROM code_chunks WHERE project_path = $1",
                &[&project_path],
            )
            .await
            .map_err(|e| AppError::Database(format!("Failed to count code chunks: {}", e)))?;

        Ok(row.get("count"))
    }

    /// Count chunks for a project that have a usable embedding.
    pub async fn count_embedded_for_project(&self, project_path: &str) -> Result<i64> {
        let client = self
            .pool
            .get()
            .await
            .map_err(|e| AppError::Database(format!("Failed to get connection: {}", e)))?;

        let row = client
            .query_one(
                "SELECT COUNT(*) as count FROM code_chunks WHERE project_path = $1 AND embedding IS NOT NULL",
                &[&project_path],
            )
            .await
            .map_err(|e| AppError::Database(format!("Failed to count embedded code chunks: {}", e)))?;

        Ok(row.get("count"))
    }

    /// Delete all chunks for a project (full re-index path).
    pub async fn clear_for_project(&self, project_path: &str) -> Result<u64> {
        let client = self
            .pool
            .get()
            .await
            .map_err(|e| AppError::Database(format!("Failed to get connection: {}", e)))?;

        let rows = client
            .execute(
                "DELETE FROM code_chunks WHERE project_path = $1",
                &[&project_path],
            )
            .await
            .map_err(|e| AppError::Database(format!("Failed to clear code chunks: {}", e)))?;

        Ok(rows)
    }

    /// pgvector cosine-similarity search over embedded chunks for a project.
    ///
    /// Returns chunks ordered by similarity (descending). Empty query vector or
    /// no embedded rows returns an empty vec so the caller can fall back.
    pub async fn vector_search(
        &self,
        query_embedding: &[f32],
        project_path: &str,
        limit: i64,
    ) -> Result<Vec<CodeChunkResult>> {
        if query_embedding.is_empty() {
            return Ok(Vec::new());
        }

        let client = self
            .pool
            .get()
            .await
            .map_err(|e| AppError::Database(format!("Failed to get connection: {}", e)))?;

        let query_vec = Vector::from(query_embedding.to_vec());

        let sql = r#"
            SELECT file_path, language, chunk_type, name, start_line, end_line,
                   content, 1 - (embedding <=> $1::vector) as similarity
            FROM code_chunks
            WHERE project_path = $2 AND embedding IS NOT NULL
            ORDER BY embedding <=> $1::vector
            LIMIT $3
        "#;

        let rows = client
            .query(sql, &[&query_vec, &project_path, &limit])
            .await
            .map_err(|e| {
                debug!("code_chunks vector_search failed: {}", e);
                AppError::Database(format!("Code chunk vector search failed: {}", e))
            })?;

        let results = rows
            .iter()
            .map(|row| CodeChunkResult {
                file_path: row.get("file_path"),
                language: row.get("language"),
                chunk_type: row.get("chunk_type"),
                name: row.get("name"),
                start_line: row.get("start_line"),
                end_line: row.get("end_line"),
                content: row.get("content"),
                similarity: row.get::<_, Option<f64>>("similarity").unwrap_or(0.0),
            })
            .collect();

        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_constructor_signature() {
        // Verifies the public API surface compiles (pool not available in unit tests).
        fn _check(pool: crate::pool::DbPool) {
            let _ = CodeChunkRepo::new(pool);
        }
    }

    #[test]
    fn test_new_code_chunk_fields() {
        let chunk = NewCodeChunk {
            project_path: ".".to_string(),
            file_path: "src/lib.rs".to_string(),
            language: "rs".to_string(),
            chunk_type: "code".to_string(),
            name: Some("main".to_string()),
            start_line: 1,
            end_line: 10,
            content: "fn main() {}".to_string(),
            content_hash: "abc123".to_string(),
            embedding: Some(vec![0.1; 768]),
        };
        assert_eq!(chunk.file_path, "src/lib.rs");
        assert_eq!(chunk.embedding.as_ref().map(|v| v.len()), Some(768));
    }
}
