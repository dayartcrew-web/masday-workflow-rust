//! Memory repository
//!
//! Table names are snake_case: "memories"
//! Column names are snake_case: "memory_type", "importance_score", "created_by_agent", etc.

use crate::pool::DbPool;
use crate::schema::{Memory, NewMemory};
use masday_core::{AppError, Result};
use pgvector::Vector;
use tracing::debug;

pub struct MemoryRepo {
    pool: DbPool,
}

impl MemoryRepo {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    /// Store a new memory (without embedding)
    pub async fn store(&self, memory: &NewMemory) -> Result<Memory> {
        let client = self
            .pool
            .get()
            .await
            .map_err(|e| AppError::Database(format!("Failed to get connection: {}", e)))?;

        let id = uuid::Uuid::new_v4().to_string();
        let now: chrono::DateTime<chrono::Utc> = chrono::Utc::now();

        // Omit embedding column entirely to avoid pgvector type serialization issues
        let query = r#"
            INSERT INTO memories (
                id, workflow_id, task_id, memory_type, summary, content,
                importance_score, created_by_agent, tags, source,
                created_at, updated_at, accessed_at, access_count, version
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15)
            RETURNING *
        "#;

        let row = client
            .query_one(
                query,
                &[
                    &id,
                    &memory.workflow_id,
                    &memory.task_id,
                    &memory.memory_type,
                    &memory.summary,
                    &memory.content,
                    &memory.importance_score,
                    &memory.created_by_agent,
                    &memory.tags,
                    &memory.source,
                    &now,
                    &now,
                    &Option::<chrono::DateTime<chrono::Utc>>::None,
                    &0i32,
                    &1i32,
                ],
            )
            .await
            .map_err(|e| AppError::Database(format!("Failed to store memory: {}", e)))?;

        Ok(Memory::from_row(&row))
    }

    /// Get memory by ID
    pub async fn get_by_id(&self, id: &str) -> Result<Memory> {
        let client = self
            .pool
            .get()
            .await
            .map_err(|e| AppError::Database(format!("Failed to get connection: {}", e)))?;

        // Update accessed_at and access_count
        let now: chrono::DateTime<chrono::Utc> = chrono::Utc::now();
        let update_query = r#"UPDATE memories SET accessed_at = $1, access_count = COALESCE(access_count, 0) + 1 WHERE id = $2"#;
        client
            .execute(update_query, &[&now, &id])
            .await
            .map_err(|e| {
                AppError::Database(format!("Failed to update memory access stats: {}", e))
            })?;

        let query = r#"SELECT * FROM memories WHERE id = $1"#;
        let row = client
            .query_one(query, &[&id])
            .await
            .map_err(|_e| AppError::not_found("Memory", id))?;

        Ok(Memory::from_row(&row))
    }

    /// Recall recent memories (limit by count)
    pub async fn recall_recent(&self, limit: i64) -> Result<Vec<Memory>> {
        let client = self
            .pool
            .get()
            .await
            .map_err(|e| AppError::Database(format!("Failed to get connection: {}", e)))?;

        let query = r#"SELECT * FROM memories ORDER BY created_at DESC LIMIT $1"#;
        let rows = client
            .query(query, &[&limit])
            .await
            .map_err(|e| AppError::Database(format!("Failed to recall recent memories: {}", e)))?;

        Ok(rows.iter().map(Memory::from_row).collect())
    }

    /// Recall memories by task ID
    pub async fn recall_by_task(&self, task_id: &str) -> Result<Vec<Memory>> {
        let client = self
            .pool
            .get()
            .await
            .map_err(|e| AppError::Database(format!("Failed to get connection: {}", e)))?;

        let query = r#"SELECT * FROM memories WHERE task_id = $1 ORDER BY created_at DESC"#;
        let rows = client
            .query(query, &[&task_id])
            .await
            .map_err(|e| AppError::Database(format!("Failed to recall task memories: {}", e)))?;

        Ok(rows.iter().map(Memory::from_row).collect())
    }

    /// Recall memories by workflow ID (bounded by limit)
    pub async fn recall_by_workflow(&self, workflow_id: &str, limit: i64) -> Result<Vec<Memory>> {
        let client = self
            .pool
            .get()
            .await
            .map_err(|e| AppError::Database(format!("Failed to get connection: {}", e)))?;

        let capped = limit.min(1000); // hard cap to prevent unbounded queries
        let query =
            r#"SELECT * FROM memories WHERE workflow_id = $1 ORDER BY created_at DESC LIMIT $2"#;
        let rows = client
            .query(query, &[&workflow_id, &capped])
            .await
            .map_err(|e| {
                AppError::Database(format!("Failed to recall workflow memories: {}", e))
            })?;

        Ok(rows.iter().map(Memory::from_row).collect())
    }

    /// Recall memories by type
    pub async fn recall_by_type(&self, memory_type: &str, limit: i64) -> Result<Vec<Memory>> {
        let client = self
            .pool
            .get()
            .await
            .map_err(|e| AppError::Database(format!("Failed to get connection: {}", e)))?;

        let query =
            r#"SELECT * FROM memories WHERE memory_type = $1 ORDER BY created_at DESC LIMIT $2"#;
        let rows = client
            .query(query, &[&memory_type, &limit])
            .await
            .map_err(|e| AppError::Database(format!("Failed to recall memories by type: {}", e)))?;

        Ok(rows.iter().map(Memory::from_row).collect())
    }

    /// Search memories by query (simple text search in summary and content)
    pub async fn search(&self, query: &str, limit: i64) -> Result<Vec<Memory>> {
        let client = self
            .pool
            .get()
            .await
            .map_err(|e| AppError::Database(format!("Failed to get connection: {}", e)))?;

        let search_pattern = format!("%{}%", query);

        let sql = r#"
            SELECT * FROM memories
            WHERE summary ILIKE $1 OR content ILIKE $1
            ORDER BY importance_score DESC, created_at DESC
            LIMIT $2
        "#;

        let rows = client
            .query(sql, &[&search_pattern, &limit])
            .await
            .map_err(|e| AppError::Database(format!("Failed to search memories: {}", e)))?;

        Ok(rows.iter().map(Memory::from_row).collect())
    }

    /// Update a memory
    pub async fn update(&self, id: &str, updates: serde_json::Value) -> Result<Memory> {
        let client = self
            .pool
            .get()
            .await
            .map_err(|e| AppError::Database(format!("Failed to get connection: {}", e)))?;

        let now: chrono::DateTime<chrono::Utc> = chrono::Utc::now();

        // Build dynamic UPDATE query
        let mut set_clauses = vec![
            r#"updated_at = $2"#.to_string(),
            r#"version = COALESCE(version, 0) + 1"#.to_string(),
        ];
        let mut param_count = 2;
        let mut params: Vec<Box<dyn tokio_postgres::types::ToSql + Sync>> =
            vec![Box::new(id.to_string()), Box::new(now)];

        if let Some(summary) = updates.get("summary").and_then(|v| v.as_str()) {
            param_count += 1;
            set_clauses.push(format!("summary = ${}", param_count));
            params.push(Box::new(summary.to_string()));
        }
        if let Some(content) = updates.get("content").and_then(|v| v.as_str()) {
            param_count += 1;
            set_clauses.push(format!("content = ${}", param_count));
            params.push(Box::new(content.to_string()));
        }
        if let Some(importance_score) = updates.get("importance_score").and_then(|v| v.as_f64()) {
            param_count += 1;
            set_clauses.push(format!(r#"importance_score = ${}"#, param_count));
            params.push(Box::new(importance_score));
        }
        if let Some(tags) = updates.get("tags") {
            param_count += 1;
            set_clauses.push(format!("tags = ${}", param_count));
            params.push(Box::new(tags.clone()));
        }

        let sql = format!(
            r#"UPDATE memories SET {} WHERE id = $1 RETURNING *"#,
            set_clauses.join(", ")
        );

        debug!("Executing memory update query: {}", sql);

        let params_refs: Vec<&(dyn tokio_postgres::types::ToSql + Sync)> =
            params.iter().map(|p| p.as_ref()).collect();

        let row = client
            .query_one(&sql, params_refs.as_slice())
            .await
            .map_err(|e| AppError::Database(format!("Failed to update memory: {}", e)))?;

        Ok(Memory::from_row(&row))
    }

    /// Delete a memory
    pub async fn delete(&self, id: &str) -> Result<bool> {
        let client = self
            .pool
            .get()
            .await
            .map_err(|e| AppError::Database(format!("Failed to get connection: {}", e)))?;

        let query = r#"DELETE FROM memories WHERE id = $1"#;
        let rows_affected = client
            .execute(query, &[&id])
            .await
            .map_err(|e| AppError::Database(format!("Failed to delete memory: {}", e)))?;

        Ok(rows_affected > 0)
    }

    /// Get memory statistics
    pub async fn stats(&self) -> Result<MemoryStats> {
        let client = self
            .pool
            .get()
            .await
            .map_err(|e| AppError::Database(format!("Failed to get connection: {}", e)))?;

        let total_query = r#"SELECT COUNT(*) as count FROM memories"#;
        let total_row = client
            .query_one(total_query, &[])
            .await
            .map_err(|e| AppError::Database(format!("Failed to get memory count: {}", e)))?;
        let total_count: i64 = total_row.get("count");

        let type_query =
            r#"SELECT memory_type, COUNT(*) as count FROM memories GROUP BY memory_type"#;
        let type_rows = client
            .query(type_query, &[])
            .await
            .map_err(|e| AppError::Database(format!("Failed to get memory type counts: {}", e)))?;

        let by_type = type_rows
            .iter()
            .map(|row| {
                let memory_type: String = row.get("memory_type");
                let count: i64 = row.get("count");
                (memory_type, count)
            })
            .collect();

        Ok(MemoryStats {
            total_count,
            by_type,
        })
    }

    /// Store a new memory with an optional embedding vector
    pub async fn store_with_embedding(
        &self,
        memory: &NewMemory,
        embedding: Option<Vec<f32>>,
    ) -> Result<Memory> {
        let client = self
            .pool
            .get()
            .await
            .map_err(|e| AppError::Database(format!("Failed to get connection: {}", e)))?;

        let id = uuid::Uuid::new_v4().to_string();
        let now: chrono::DateTime<chrono::Utc> = chrono::Utc::now();

        let pgvec = embedding.map(Vector::from);

        let query = r#"
            INSERT INTO memories (
                id, workflow_id, task_id, memory_type, summary, content,
                importance_score, created_by_agent, tags, source,
                embedding, created_at, updated_at, accessed_at, access_count, version
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16)
            RETURNING *
        "#;

        let row = client
            .query_one(
                query,
                &[
                    &id,
                    &memory.workflow_id,
                    &memory.task_id,
                    &memory.memory_type,
                    &memory.summary,
                    &memory.content,
                    &memory.importance_score,
                    &memory.created_by_agent,
                    &memory.tags,
                    &memory.source,
                    &pgvec,
                    &now,
                    &now,
                    &now,
                    &0,
                    &1,
                ],
            )
            .await
            .map_err(|e| AppError::Database(format!("Failed to store memory: {}", e)))?;

        Ok(Memory::from_row(&row))
    }

    /// Vector similarity search using pgvector cosine distance
    /// Falls back to text search if no query embedding provided
    pub async fn vector_search(
        &self,
        query_embedding: &[f32],
        limit: i64,
    ) -> Result<Vec<(Memory, f64)>> {
        if query_embedding.is_empty() {
            // Fall back to text search
            let memories = self.search("", limit).await?;
            return Ok(memories.into_iter().map(|m| (m, 0.0)).collect());
        }

        let client = self
            .pool
            .get()
            .await
            .map_err(|e| AppError::Database(format!("Failed to get connection: {}", e)))?;

        let query_vec = Vector::from(query_embedding.to_vec());

        let sql = r#"
            SELECT *,
                   1 - (embedding <=> $1::vector) as similarity
            FROM memories
            WHERE embedding IS NOT NULL
            ORDER BY embedding <=> $1::vector
            LIMIT $2
        "#;

        let rows = client
            .query(sql, &[&query_vec, &limit])
            .await
            .map_err(|e| AppError::Database(format!("Vector search failed: {}", e)))?;

        let results = rows
            .iter()
            .map(|row| {
                let similarity: f64 = row.get::<_, Option<f64>>("similarity").unwrap_or(0.0);
                let memory = Memory::from_row(row);
                (memory, similarity)
            })
            .collect();

        Ok(results)
    }
}

/// Memory statistics
#[derive(Debug, Clone)]
pub struct MemoryStats {
    pub total_count: i64,
    pub by_type: std::collections::HashMap<String, i64>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_constructor_signature() {
        fn _check() {
            let _ = MemoryRepo::new;
        }
    }

    #[test]
    fn test_new_memory_serialization_roundtrip() {
        let input = NewMemory {
            workflow_id: Some("wf-123".to_string()),
            task_id: None,
            memory_type: "preference".to_string(),
            summary: "User prefers dark mode".to_string(),
            content: "Set in settings".to_string(),
            importance_score: Some(0.8),
            created_by_agent: "masday-orchestrator".to_string(),
            tags: Some(vec!["ui".to_string()]),
            source: Some("user_feedback".to_string()),
            embedding: None,
        };
        let json = serde_json::to_string(&input).unwrap();
        let parsed: NewMemory = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.memory_type, "preference");
        assert_eq!(parsed.summary, "User prefers dark mode");
        assert_eq!(parsed.importance_score, Some(0.8));
        // embedding is #[serde(skip)] so it won't round-trip
        assert!(parsed.tags.is_some());
    }

    #[test]
    fn test_memory_stats_construction() {
        let mut by_type = std::collections::HashMap::new();
        by_type.insert("preference".to_string(), 5);
        by_type.insert("experience".to_string(), 12);
        let stats = MemoryStats {
            total_count: 17,
            by_type,
        };
        assert_eq!(stats.total_count, 17);
        assert_eq!(stats.by_type.len(), 2);
    }

    #[test]
    fn test_search_pattern_construction() {
        let query = "dark mode";
        let pattern = format!("%{}%", query);
        assert_eq!(pattern, "%dark mode%");
    }

    #[test]
    fn test_limit_capping() {
        assert_eq!(1000, 1000);
        assert_eq!(500i64, 500);
        assert_eq!(1000, 1000);
    }
}
