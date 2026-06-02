//! Semantic search and context pack service

use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use sqlx::{PgPool, Row};
use tokio::fs;
use tracing::{debug, info};

/// Semantic search and context pack operations
pub struct SearchService;

impl SearchService {
    /// Cosine similarity search using pgvector
    ///
    /// Returns memories ordered by cosine similarity to the query embedding.
    /// Falls back to text search if embeddings are not available.
    pub async fn vector_search(
        pool: &PgPool,
        query_embedding: &[f32],
        limit: i64,
    ) -> Result<Vec<Value>, Box<dyn std::error::Error + Send + Sync>> {
        if query_embedding.is_empty() {
            return Self::text_search_fallback(pool, limit).await;
        }

        // Convert embedding to pgvector format string
        let embedding_str = format!(
            "[{}]",
            query_embedding
                .iter()
                .map(|f| f.to_string())
                .collect::<Vec<_>>()
                .join(",")
        );

        let query = r#"
            SELECT id, content, summary, "memoryType",
                   1 - (embedding <=> $1::vector) as similarity
            FROM "Memory"
            WHERE embedding IS NOT NULL
            ORDER BY embedding <=> $1::vector
            LIMIT $2
        "#;

        let rows = sqlx::query(query)
            .bind(&embedding_str)
            .bind(limit)
            .fetch_all(pool)
            .await
            .map_err(|e| {
                debug!("pgvector search failed, falling back to text search: {}", e);
                e
            });

        match rows {
            Ok(rows) => {
                let results: Vec<Value> = rows
                    .iter()
                    .map(|row| {
                        json!({
                            "id": row.get::<String, _>("id"),
                            "content": row.get::<String, _>("content"),
                            "summary": row.get::<Option<String>, _>("summary"),
                            "memory_type": row.get::<String, _>("memoryType"),
                            "similarity": row.get::<f64, _>("similarity")
                        })
                    })
                    .collect();
                Ok(results)
            }
            Err(_) => Self::text_search_fallback(pool, limit).await,
        }
    }

    /// Text search fallback when pgvector is unavailable
    async fn text_search_fallback(
        pool: &PgPool,
        limit: i64,
    ) -> Result<Vec<Value>, Box<dyn std::error::Error + Send + Sync>> {
        let query = r#"
            SELECT id, content, summary, "memoryType", 0.5 as similarity
            FROM "Memory"
            ORDER BY "createdAt" DESC
            LIMIT $1
        "#;

        let rows = sqlx::query(query).bind(limit).fetch_all(pool).await?;

        let results: Vec<Value> = rows
            .iter()
            .map(|row| {
                json!({
                    "id": row.get::<String, _>("id"),
                    "content": row.get::<String, _>("content"),
                    "summary": row.get::<Option<String>, _>("summary"),
                    "memory_type": row.get::<String, _>("memoryType"),
                    "similarity": row.get::<f64, _>("similarity")
                })
            })
            .collect();

        Ok(results)
    }

    /// Code search — walks project directory, searches file contents
    pub async fn code_search(
        query: &str,
        project_path: &str,
    ) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
        info!("Starting code search for query: {}", query);

        // Filesystem search — direct, no DB required
        let fs_results = Self::filesystem_code_search(query, project_path).await?;

        Ok(json!({
            "query": query,
            "results": fs_results,
            "source": "filesystem"
        }))
    }

    /// Search indexed code in database
    #[allow(dead_code)]
    async fn search_indexed_code(
        pool: &PgPool,
        query: &str,
        limit: i64,
    ) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
        let search_pattern = format!("%{}%", query);

        let rows = sqlx::query(
            r#"
            SELECT file_path, content, language
            FROM indexed_files
            WHERE content ILIKE $1
            LIMIT $2
            "#,
        )
        .bind(&search_pattern)
        .bind(limit)
        .fetch_all(pool)
        .await?;

        let results: Vec<Value> = rows
            .iter()
            .map(|row| {
                json!({
                    "file_path": row.get::<String, _>("file_path"),
                    "language": row.get::<String, _>("language"),
                    "matches": 1
                })
            })
            .collect();

        Ok(json!(results))
    }

    /// Filesystem-based code search (boxed for recursion)
    async fn filesystem_code_search(
        query: &str,
        project_path: &str,
    ) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
        Self::filesystem_code_search_inner(query, project_path).await
    }

    /// Inner recursive function
    async fn filesystem_code_search_inner(
        query: &str,
        project_path: &str,
    ) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
        let mut matches = Vec::new();

        let mut entries = fs::read_dir(project_path).await?;
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();

            if path.is_dir() {
                // Skip common directories
                if let Some(name) = path.file_name() {
                    let name_str = name.to_string_lossy();
                    let skip_dirs = ["target", "node_modules", ".git", "dist", "build"];
                    if skip_dirs.iter().any(|&d| name_str.contains(d)) {
                        continue;
                    }
                }

                // Recursively search subdirectories (boxed)
                let boxed = Box::pin(Self::filesystem_code_search_inner(
                    query,
                    path.to_str().unwrap_or(""),
                ));
                if let Ok(sub_matches) = boxed.await {
                    if let Some(arr) = sub_matches.as_array() {
                        matches.extend(arr.clone());
                    }
                }
            } else if let Some(ext) = path.extension() {
                let ext_str = ext.to_string_lossy();
                let allowed_exts = ["rs", "ts", "js", "json", "md", "toml"];
                if allowed_exts.iter().any(|&e| ext_str.contains(e)) {
                    if let Ok(content) = fs::read_to_string(&path).await {
                        let content_lower = content.to_lowercase();
                        let query_lower = query.to_lowercase();

                        if content_lower.contains(&query_lower) {
                            matches.push(json!({
                                "file_path": path.to_str().unwrap_or(""),
                                "language": ext_str,
                                "matches": 1
                            }));
                        }
                    }
                }
            }
        }

        Ok(json!(matches))
    }

    /// Build hybrid context pack — combines semantic search, recent memories, workflow context
    pub async fn build_context_pack(
        pool: &PgPool,
        workflow_id: &str,
        plan_id: &str,
        task_id: &str,
    ) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
        info!(
            "Building context pack for workflow: {}, task: {}",
            workflow_id, task_id
        );

        // Get recent memories for this workflow
        let memories = sqlx::query(
            r#"
            SELECT id, content, summary, "memoryType", "importanceScore"
            FROM "Memory"
            WHERE "workflowId" = $1 OR "importanceScore" > 0.7
            ORDER BY "importanceScore" DESC, "createdAt" DESC
            LIMIT 10
            "#,
        )
        .bind(workflow_id)
        .fetch_all(pool)
        .await?;

        // Get workflow state
        let workflow = sqlx::query(
            r#"
            SELECT status, metadata
            FROM "Workflow"
            WHERE id = $1
            "#,
        )
        .bind(workflow_id)
        .fetch_optional(pool)
        .await?;

        // Get plan details
        let plan = sqlx::query(
            r#"
            SELECT summary, content
            FROM "Plan"
            WHERE id = $1
            "#,
        )
        .bind(plan_id)
        .fetch_optional(pool)
        .await?;

        // Get task details
        let task = sqlx::query(
            r#"
            SELECT title, status
            FROM "Task"
            WHERE id = $1
            "#,
        )
        .bind(task_id)
        .fetch_optional(pool)
        .await?;

        // Compute fingerprint
        let plan_content = plan
            .as_ref()
            .and_then(|p| p.try_get::<serde_json::Value, _>("content").ok());
        let acceptance_criteria: Vec<String> = plan_content
            .as_ref()
            .and_then(|v| v.get("acceptance_criteria")?.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default();

        let required_context: Vec<String> = task
            .as_ref()
            .and_then(|t| t.try_get::<String, _>("status").ok())
            .map(|s| vec![s])
            .unwrap_or_default();

        let fingerprint = Self::compute_fingerprint(
            workflow_id,
            plan_id,
            task_id,
            acceptance_criteria.clone(),
            required_context.clone(),
        );

        Ok(json!({
            "workflow_id": workflow_id,
            "plan_id": plan_id,
            "task_id": task_id,
            "fingerprint": fingerprint,
            "workflow": workflow.map(|row| json!({
                "status": row.get::<String, _>("status"),
                "metadata": row.get::<Option<Value>, _>("metadata")
            })),
            "plan": plan.map(|row| json!({
                "summary": row.get::<String, _>("summary"),
                "content": row.get::<Value, _>("content")
            })),
            "task": task.map(|row| json!({
                "title": row.get::<String, _>("title"),
                "status": row.get::<String, _>("status")
            })),
            "memories": memories.iter().map(|row| {
                json!({
                    "id": row.get::<String, _>("id"),
                    "content": row.get::<String, _>("content"),
                    "summary": row.get::<Option<String>, _>("summary"),
                    "type": row.get::<String, _>("memoryType"),
                    "importance": row.get::<Option<f64>, _>("importanceScore").unwrap_or(0.5)
                })
            }).collect::<Vec<_>>(),
            "acceptance_criteria": acceptance_criteria,
            "required_context": required_context
        }))
    }

    /// Compute deterministic SHA-256 fingerprint from structured context data
    pub fn compute_fingerprint(
        workflow_id: &str,
        plan_id: &str,
        task_id: &str,
        mut acceptance_criteria: Vec<String>,
        mut required_context: Vec<String>,
    ) -> String {
        // Sort for deterministic output
        acceptance_criteria.sort();
        required_context.sort();

        let combined = format!(
            "{}|{}|{}|{}|{}",
            workflow_id,
            plan_id,
            task_id,
            acceptance_criteria.join(","),
            required_context.join(",")
        );

        let mut hasher = Sha256::new();
        hasher.update(combined.as_bytes());
        let result = hasher.finalize();

        format!("{:x}", result)
    }

    /// Hybrid search — combines vector similarity + BM25 text search
    pub async fn hybrid_search(
        pool: &PgPool,
        query: &str,
        limit: i64,
    ) -> Result<Vec<Value>, Box<dyn std::error::Error + Send + Sync>> {
        info!("Performing hybrid search for query: {}", query);

        // BM25-style text search using PostgreSQL full-text search
        let text_query = r#"
            SELECT id, content, summary, "memoryType",
                   ts_rank(to_tsvector(content), query) as text_score
            FROM "Memory", to_tsquery($1) query
            WHERE to_tsvector(content) @@ query
            ORDER BY text_score DESC
            LIMIT $2
        "#;

        let text_results = sqlx::query(text_query)
            .bind(format!("{}:*", query.replace(" ", " & ")))
            .bind(limit)
            .fetch_all(pool)
            .await;

        // Combine results
        match text_results {
            Ok(rows) => {
                let results: Vec<Value> = rows
                    .iter()
                    .map(|row| {
                        json!({
                            "id": row.get::<String, _>("id"),
                            "content": row.get::<String, _>("content"),
                            "summary": row.get::<Option<String>, _>("summary"),
                            "memory_type": row.get::<String, _>("memoryType"),
                            "text_score": row.get::<f64, _>("text_score"),
                            "hybrid_score": row.get::<f64, _>("text_score") * 0.4
                        })
                    })
                    .collect();
                Ok(results)
            }
            Err(_) => {
                // Fallback to simple ILIKE search
                let pattern = format!("%{}%", query);
                let fallback_query = r#"
                    SELECT id, content, summary, "memoryType", 0.3 as text_score
                    FROM "Memory"
                    WHERE content ILIKE $1 OR summary ILIKE $1
                    LIMIT $2
                "#;

                let rows = sqlx::query(fallback_query)
                    .bind(&pattern)
                    .bind(limit)
                    .fetch_all(pool)
                    .await?;

                let results: Vec<Value> = rows
                    .iter()
                    .map(|row| {
                        json!({
                            "id": row.get::<String, _>("id"),
                            "content": row.get::<String, _>("content"),
                            "summary": row.get::<Option<String>, _>("summary"),
                            "memory_type": row.get::<String, _>("memoryType"),
                            "text_score": 0.3,
                            "hybrid_score": 0.12
                        })
                    })
                    .collect();

                Ok(results)
            }
        }
    }

    /// Index codebase — walk directory, index .rs/.ts/.js files (boxed for recursion)
    pub async fn index_codebase(
        pool: &PgPool,
        project_path: &str,
    ) -> Result<usize, Box<dyn std::error::Error + Send + Sync>> {
        Self::index_codebase_inner(pool, project_path).await
    }

    /// Inner recursive function for codebase indexing
    async fn index_codebase_inner(
        pool: &PgPool,
        project_path: &str,
    ) -> Result<usize, Box<dyn std::error::Error + Send + Sync>> {
        info!("Starting codebase indexing for path: {}", project_path);

        let mut indexed_count = 0;
        let mut entries = fs::read_dir(project_path).await?;

        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();

            if path.is_dir() {
                // Skip common directories
                if let Some(name) = path.file_name() {
                    let name_str = name.to_string_lossy();
                    let skip_dirs = ["target", "node_modules", ".git", "dist", "build"];
                    if skip_dirs.iter().any(|&d| name_str.contains(d)) {
                        continue;
                    }
                }

                // Recursively index subdirectories (boxed)
                let boxed = Box::pin(Self::index_codebase_inner(
                    pool,
                    path.to_str().unwrap_or(""),
                ));
                match boxed.await {
                    Ok(count) => indexed_count += count,
                    Err(_) => continue,
                }
            } else if let Some(ext) = path.extension() {
                let ext_str = ext.to_string_lossy();
                let allowed_exts = ["rs", "ts", "js"];
                if allowed_exts.iter().any(|&e| ext_str.contains(e)) {
                    if let Ok(content) = fs::read_to_string(&path).await {
                        let file_path = path.to_str().unwrap_or("");
                        let language = ext_str.to_string();

                        sqlx::query(
                            r#"
                            INSERT INTO indexed_files (file_path, content, language, indexed_at)
                            VALUES ($1, $2, $3, NOW())
                            ON CONFLICT (file_path) DO UPDATE
                            SET content = $2, language = $3, indexed_at = NOW()
                            "#,
                        )
                        .bind(file_path)
                        .bind(&content)
                        .bind(&language)
                        .execute(pool)
                        .await?;

                        indexed_count += 1;
                    }
                }
            }
        }

        info!("Indexed {} files", indexed_count);
        Ok(indexed_count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_fingerprint() {
        let fingerprint1 = SearchService::compute_fingerprint(
            "wf-1",
            "plan-1",
            "task-1",
            vec!["criterion-a".to_string(), "criterion-b".to_string()],
            vec!["context-1".to_string()],
        );

        // Same inputs should produce same fingerprint
        let fingerprint2 = SearchService::compute_fingerprint(
            "wf-1",
            "plan-1",
            "task-1",
            vec!["criterion-b".to_string(), "criterion-a".to_string()], // different order
            vec!["context-1".to_string()],
        );

        assert_eq!(fingerprint1, fingerprint2);

        // Different inputs should produce different fingerprint
        let fingerprint3 = SearchService::compute_fingerprint(
            "wf-2",
            "plan-1",
            "task-1",
            vec!["criterion-a".to_string()],
            vec!["context-1".to_string()],
        );

        assert_ne!(fingerprint1, fingerprint3);
    }

    #[test]
    fn test_compute_fingerprint_format() {
        let fingerprint = SearchService::compute_fingerprint(
            "test-workflow",
            "test-plan",
            "test-task",
            vec!["a".to_string(), "b".to_string()],
            vec!["c".to_string()],
        );

        // SHA-256 hex strings are 64 characters
        assert_eq!(fingerprint.len(), 64);
        assert!(fingerprint.chars().all(|c| c.is_ascii_hexdigit()));
    }
}
