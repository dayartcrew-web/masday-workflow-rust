//! Memory repository

use crate::schema::{Memory, NewMemory};
use crate::pool::DbPool;
use masday_core::{AppError, Result};
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
        let client = self.pool.get().await
            .map_err(|e| AppError::Database(format!("Failed to get connection: {}", e)))?;

        let id = uuid::Uuid::new_v4().to_string();
        let now: chrono::DateTime<chrono::Utc> = chrono::Utc::now();

        let query = "
            INSERT INTO memories (
                id, workflow_id, task_id, memory_type, summary, content,
                importance_score, created_by_agent, tags, source, embedding,
                created_at, updated_at, accessed_at, access_count, version
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16)
            RETURNING *
        ";

        let row = client.query_one(
            query,
            &[
                &id, &memory.workflow_id, &memory.task_id, &memory.memory_type,
                &memory.summary, &memory.content, &memory.importance_score,
                &memory.created_by_agent, &memory.tags, &memory.source,
                &memory.embedding, &now, &now, &now, &0, &1,
            ],
        ).await.map_err(|e| AppError::Database(format!("Failed to store memory: {}", e)))?;

        Ok(Memory {
            id: row.get("id"),
            workflow_id: row.get("workflow_id"),
            task_id: row.get("task_id"),
            memory_type: row.get("memory_type"),
            summary: row.get("summary"),
            content: row.get("content"),
            importance_score: row.get("importance_score"),
            created_by_agent: row.get("created_by_agent"),
            tags: row.get("tags"),
            source: row.get("source"),
            embedding: row.get("embedding"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
            accessed_at: row.get("accessed_at"),
            access_count: row.get("access_count"),
            version: row.get("version"),
        })
    }

    /// Get memory by ID
    pub async fn get_by_id(&self, id: &str) -> Result<Memory> {
        let client = self.pool.get().await
            .map_err(|e| AppError::Database(format!("Failed to get connection: {}", e)))?;

        // Update accessed_at and access_count
        let now: chrono::DateTime<chrono::Utc> = chrono::Utc::now();
        let update_query = "UPDATE memories SET accessed_at = $1, access_count = COALESCE(access_count, 0) + 1 WHERE id = $2";
        client.execute(update_query, &[&now, &id]).await
            .map_err(|e| AppError::Database(format!("Failed to update memory access stats: {}", e)))?;

        let query = "SELECT * FROM memories WHERE id = $1";
        let row = client.query_one(query, &[&id]).await
            .map_err(|_e| AppError::not_found("Memory", id))?;

        Ok(Memory {
            id: row.get("id"),
            workflow_id: row.get("workflow_id"),
            task_id: row.get("task_id"),
            memory_type: row.get("memory_type"),
            summary: row.get("summary"),
            content: row.get("content"),
            importance_score: row.get("importance_score"),
            created_by_agent: row.get("created_by_agent"),
            tags: row.get("tags"),
            source: row.get("source"),
            embedding: row.get("embedding"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
            accessed_at: row.get("accessed_at"),
            access_count: row.get("access_count"),
            version: row.get("version"),
        })
    }

    /// Recall recent memories (limit by count)
    pub async fn recall_recent(&self, limit: i64) -> Result<Vec<Memory>> {
        let client = self.pool.get().await
            .map_err(|e| AppError::Database(format!("Failed to get connection: {}", e)))?;

        let query = "SELECT * FROM memories ORDER BY created_at DESC LIMIT $1";
        let rows = client.query(query, &[&limit]).await
            .map_err(|e| AppError::Database(format!("Failed to recall recent memories: {}", e)))?;

        let memories = rows.iter().map(|row| Memory {
            id: row.get("id"),
            workflow_id: row.get("workflow_id"),
            task_id: row.get("task_id"),
            memory_type: row.get("memory_type"),
            summary: row.get("summary"),
            content: row.get("content"),
            importance_score: row.get("importance_score"),
            created_by_agent: row.get("created_by_agent"),
            tags: row.get("tags"),
            source: row.get("source"),
            embedding: row.get("embedding"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
            accessed_at: row.get("accessed_at"),
            access_count: row.get("access_count"),
            version: row.get("version"),
        }).collect();

        Ok(memories)
    }

    /// Recall memories by task ID
    pub async fn recall_by_task(&self, task_id: &str) -> Result<Vec<Memory>> {
        let client = self.pool.get().await
            .map_err(|e| AppError::Database(format!("Failed to get connection: {}", e)))?;

        let query = "SELECT * FROM memories WHERE task_id = $1 ORDER BY created_at DESC";
        let rows = client.query(query, &[&task_id]).await
            .map_err(|e| AppError::Database(format!("Failed to recall task memories: {}", e)))?;

        let memories = rows.iter().map(|row| Memory {
            id: row.get("id"),
            workflow_id: row.get("workflow_id"),
            task_id: row.get("task_id"),
            memory_type: row.get("memory_type"),
            summary: row.get("summary"),
            content: row.get("content"),
            importance_score: row.get("importance_score"),
            created_by_agent: row.get("created_by_agent"),
            tags: row.get("tags"),
            source: row.get("source"),
            embedding: row.get("embedding"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
            accessed_at: row.get("accessed_at"),
            access_count: row.get("access_count"),
            version: row.get("version"),
        }).collect();

        Ok(memories)
    }

    /// Recall memories by workflow ID
    pub async fn recall_by_workflow(&self, workflow_id: &str) -> Result<Vec<Memory>> {
        let client = self.pool.get().await
            .map_err(|e| AppError::Database(format!("Failed to get connection: {}", e)))?;

        let query = "SELECT * FROM memories WHERE workflow_id = $1 ORDER BY created_at DESC";
        let rows = client.query(query, &[&workflow_id]).await
            .map_err(|e| AppError::Database(format!("Failed to recall workflow memories: {}", e)))?;

        let memories = rows.iter().map(|row| Memory {
            id: row.get("id"),
            workflow_id: row.get("workflow_id"),
            task_id: row.get("task_id"),
            memory_type: row.get("memory_type"),
            summary: row.get("summary"),
            content: row.get("content"),
            importance_score: row.get("importance_score"),
            created_by_agent: row.get("created_by_agent"),
            tags: row.get("tags"),
            source: row.get("source"),
            embedding: row.get("embedding"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
            accessed_at: row.get("accessed_at"),
            access_count: row.get("access_count"),
            version: row.get("version"),
        }).collect();

        Ok(memories)
    }

    /// Recall memories by type
    pub async fn recall_by_type(&self, memory_type: &str, limit: i64) -> Result<Vec<Memory>> {
        let client = self.pool.get().await
            .map_err(|e| AppError::Database(format!("Failed to get connection: {}", e)))?;

        let query = "SELECT * FROM memories WHERE memory_type = $1 ORDER BY created_at DESC LIMIT $2";
        let rows = client.query(query, &[&memory_type, &limit]).await
            .map_err(|e| AppError::Database(format!("Failed to recall memories by type: {}", e)))?;

        let memories = rows.iter().map(|row| Memory {
            id: row.get("id"),
            workflow_id: row.get("workflow_id"),
            task_id: row.get("task_id"),
            memory_type: row.get("memory_type"),
            summary: row.get("summary"),
            content: row.get("content"),
            importance_score: row.get("importance_score"),
            created_by_agent: row.get("created_by_agent"),
            tags: row.get("tags"),
            source: row.get("source"),
            embedding: row.get("embedding"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
            accessed_at: row.get("accessed_at"),
            access_count: row.get("access_count"),
            version: row.get("version"),
        }).collect();

        Ok(memories)
    }

    /// Search memories by query (simple text search in summary and content)
    pub async fn search(&self, query: &str, limit: i64) -> Result<Vec<Memory>> {
        let client = self.pool.get().await
            .map_err(|e| AppError::Database(format!("Failed to get connection: {}", e)))?;

        let search_pattern = format!("%{}%", query);

        let sql = "
            SELECT * FROM memories
            WHERE summary ILIKE $1 OR content ILIKE $1
            ORDER BY importance_score DESC, created_at DESC
            LIMIT $2
        ";

        let rows = client.query(sql, &[&search_pattern, &limit]).await
            .map_err(|e| AppError::Database(format!("Failed to search memories: {}", e)))?;

        let memories = rows.iter().map(|row| Memory {
            id: row.get("id"),
            workflow_id: row.get("workflow_id"),
            task_id: row.get("task_id"),
            memory_type: row.get("memory_type"),
            summary: row.get("summary"),
            content: row.get("content"),
            importance_score: row.get("importance_score"),
            created_by_agent: row.get("created_by_agent"),
            tags: row.get("tags"),
            source: row.get("source"),
            embedding: row.get("embedding"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
            accessed_at: row.get("accessed_at"),
            access_count: row.get("access_count"),
            version: row.get("version"),
        }).collect();

        Ok(memories)
    }

    /// Update a memory
    pub async fn update(&self, id: &str, updates: serde_json::Value) -> Result<Memory> {
        let client = self.pool.get().await
            .map_err(|e| AppError::Database(format!("Failed to get connection: {}", e)))?;

        let now: chrono::DateTime<chrono::Utc> = chrono::Utc::now();

        // Build dynamic UPDATE query
        let mut set_clauses = vec!["updated_at = $2".to_string(), "version = COALESCE(version, 0) + 1".to_string()];
        let mut param_count = 2;
        let mut params: Vec<Box<dyn tokio_postgres::types::ToSql + Sync>> = vec![
            Box::new(id.to_string()),
            Box::new(now)
        ];

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
            set_clauses.push(format!("importance_score = ${}", param_count));
            params.push(Box::new(importance_score));
        }
        if let Some(tags) = updates.get("tags") {
            param_count += 1;
            set_clauses.push(format!("tags = ${}", param_count));
            params.push(Box::new(tags.clone()));
        }

        let sql = format!(
            "UPDATE memories SET {} WHERE id = $1 RETURNING *",
            set_clauses.join(", ")
        );

        debug!("Executing memory update query: {}", sql);

        let params_refs: Vec<&(dyn tokio_postgres::types::ToSql + Sync)> = params.iter().map(|p| p.as_ref()).collect();

        let row = client.query_one(&sql, params_refs.as_slice()).await
            .map_err(|e| AppError::Database(format!("Failed to update memory: {}", e)))?;

        Ok(Memory {
            id: row.get("id"),
            workflow_id: row.get("workflow_id"),
            task_id: row.get("task_id"),
            memory_type: row.get("memory_type"),
            summary: row.get("summary"),
            content: row.get("content"),
            importance_score: row.get("importance_score"),
            created_by_agent: row.get("created_by_agent"),
            tags: row.get("tags"),
            source: row.get("source"),
            embedding: row.get("embedding"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
            accessed_at: row.get("accessed_at"),
            access_count: row.get("access_count"),
            version: row.get("version"),
        })
    }

    /// Delete a memory
    pub async fn delete(&self, id: &str) -> Result<bool> {
        let client = self.pool.get().await
            .map_err(|e| AppError::Database(format!("Failed to get connection: {}", e)))?;

        let query = "DELETE FROM memories WHERE id = $1";
        let rows_affected = client.execute(query, &[&id]).await
            .map_err(|e| AppError::Database(format!("Failed to delete memory: {}", e)))?;

        Ok(rows_affected > 0)
    }

    /// Get memory statistics
    pub async fn stats(&self) -> Result<MemoryStats> {
        let client = self.pool.get().await
            .map_err(|e| AppError::Database(format!("Failed to get connection: {}", e)))?;

        let total_query = "SELECT COUNT(*) as count FROM memories";
        let total_row = client.query_one(total_query, &[]).await
            .map_err(|e| AppError::Database(format!("Failed to get memory count: {}", e)))?;
        let total_count: i64 = total_row.get("count");

        let type_query = "SELECT memory_type, COUNT(*) as count FROM memories GROUP BY memory_type";
        let type_rows = client.query(type_query, &[]).await
            .map_err(|e| AppError::Database(format!("Failed to get memory type counts: {}", e)))?;

        let by_type = type_rows.iter().map(|row| {
            let memory_type: String = row.get("memory_type");
            let count: i64 = row.get("count");
            (memory_type, count)
        }).collect();

        Ok(MemoryStats {
            total_count,
            by_type,
        })
    }
}

/// Memory statistics
#[derive(Debug, Clone)]
pub struct MemoryStats {
    pub total_count: i64,
    pub by_type: std::collections::HashMap<String, i64>,
}
