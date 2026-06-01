//! Memory repository
//!
//! Table names are PascalCase (created by Drizzle/TypeScript): "Memory"
//! Column names are camelCase: "memoryType", "importanceScore", "createdByAgent", etc.

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

    /// Store a new memory
    pub async fn store(&self, memory: &NewMemory) -> Result<Memory> {
        let client = self
            .pool
            .get()
            .await
            .map_err(|e| AppError::Database(format!("Failed to get connection: {}", e)))?;

        let id = uuid::Uuid::new_v4().to_string();
        let now: chrono::NaiveDateTime = chrono::Utc::now().naive_utc();

        let query = r#"
            INSERT INTO "Memory" (
                id, "workflowId", "taskId", "memoryType", summary, content,
                "importanceScore", "createdByAgent", tags, source,
                embedding, "createdAt", "updatedAt", "accessedAt", "accessCount", version
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16)
            RETURNING *
        "#;

        let embedding_pg: Option<Vector> = memory.embedding.clone();

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
                    &embedding_pg,
                    &now,
                    &now,
                    &Option::<chrono::NaiveDateTime>::None,
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

        // Update accessedAt and accessCount
        let now: chrono::NaiveDateTime = chrono::Utc::now().naive_utc();
        let update_query = r#"UPDATE "Memory" SET "accessedAt" = $1, "accessCount" = COALESCE("accessCount", 0) + 1 WHERE id = $2"#;
        client
            .execute(update_query, &[&now, &id])
            .await
            .map_err(|e| {
                AppError::Database(format!("Failed to update memory access stats: {}", e))
            })?;

        let query = r#"SELECT * FROM "Memory" WHERE id = $1"#;
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

        let query = r#"SELECT * FROM "Memory" ORDER BY "createdAt" DESC LIMIT $1"#;
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

        let query = r#"SELECT * FROM "Memory" WHERE "taskId" = $1 ORDER BY "createdAt" DESC"#;
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
            r#"SELECT * FROM "Memory" WHERE "workflowId" = $1 ORDER BY "createdAt" DESC LIMIT $2"#;
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
            r#"SELECT * FROM "Memory" WHERE "memoryType" = $1 ORDER BY "createdAt" DESC LIMIT $2"#;
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
            SELECT * FROM "Memory"
            WHERE summary ILIKE $1 OR content ILIKE $1
            ORDER BY "importanceScore" DESC, "createdAt" DESC
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

        let now: chrono::NaiveDateTime = chrono::Utc::now().naive_utc();

        // Build dynamic UPDATE query
        let mut set_clauses = vec![
            r#""updatedAt" = $2"#.to_string(),
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
            set_clauses.push(format!(r#""importanceScore" = ${}"#, param_count));
            params.push(Box::new(importance_score));
        }
        if let Some(tags) = updates.get("tags") {
            param_count += 1;
            set_clauses.push(format!("tags = ${}", param_count));
            params.push(Box::new(tags.clone()));
        }

        let sql = format!(
            r#"UPDATE "Memory" SET {} WHERE id = $1 RETURNING *"#,
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

        let query = r#"DELETE FROM "Memory" WHERE id = $1"#;
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

        let total_query = r#"SELECT COUNT(*) as count FROM "Memory""#;
        let total_row = client
            .query_one(total_query, &[])
            .await
            .map_err(|e| AppError::Database(format!("Failed to get memory count: {}", e)))?;
        let total_count: i64 = total_row.get("count");

        let type_query = r#"SELECT "memoryType", COUNT(*) as count FROM "Memory" GROUP BY "memoryType""#;
        let type_rows = client
            .query(type_query, &[])
            .await
            .map_err(|e| AppError::Database(format!("Failed to get memory type counts: {}", e)))?;

        let by_type = type_rows
            .iter()
            .map(|row| {
                let memory_type: String = row.get("memoryType");
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
        let now: chrono::NaiveDateTime = chrono::Utc::now().naive_utc();

        let pgvec = embedding.map(Vector::from);

        let query = r#"
            INSERT INTO "Memory" (
                id, "workflowId", "taskId", "memoryType", summary, content,
                "importanceScore", "createdByAgent", tags, source,
                embedding, "createdAt", "updatedAt", "accessedAt", "accessCount", version
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
            FROM "Memory"
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

    /// Update embedding for a memory
    pub async fn update_embedding(&self, id: &str, embedding: Vec<f32>) -> Result<bool> {
        let client = self
            .pool
            .get()
            .await
            .map_err(|e| AppError::Database(format!("Failed to get connection: {}", e)))?;

        let pgvec = Vector::from(embedding);
        let rows = client
            .execute(
                r#"UPDATE "Memory" SET embedding = $1, "updatedAt" = NOW() WHERE id = $2"#,
                &[&pgvec, &id],
            )
            .await
            .map_err(|e| AppError::Database(format!("Failed to update embedding: {}", e)))?;

        Ok(rows > 0)
    }
}

/// Memory statistics
#[derive(Debug, Clone)]
pub struct MemoryStats {
    pub total_count: i64,
    pub by_type: std::collections::HashMap<String, i64>,
}
