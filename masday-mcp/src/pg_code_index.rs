//! PostgreSQL code indexer — chunk + embed source files into the `code_chunks` table.
//!
//! Mirrors `code_index::index_project` (SQLite) but stores **real** Ollama embeddings
//! (read from `~/.masday/config.toml`) for pgvector cosine search. The MCP cannot use
//! `masday_service::EmbeddingService` (compiled `default-features=false`), so it calls
//! Ollama HTTP directly via `tools::local::generate_embedding`, exactly like the memory
//! sync path.
//!
//! Embedding failures are per-chunk resilient: a failed embed stores the chunk with a
//! NULL embedding (excluded from vector search, available for a later re-index) rather
//! than dropping it.
//!
//! Indexing is one Ollama call per chunk and can take minutes on a large project, so
//! `trigger_background_index` runs it fire-and-forget behind a once-guard: the first
//! search returns SQLite results immediately, and pgvector becomes available once the
//! background index completes. Never blocks a tool call.

use crate::code_index::{chunk_file, collect_files, content_hash, ext_to_language};
use masday_db::repos::{normalize_project_path, CodeChunkRepo};
use masday_db::schema::NewCodeChunk;
use serde_json::{json, Value};
use std::sync::atomic::{AtomicBool, Ordering};
use tracing::{info, warn};

/// Guard so we never spawn two concurrent background indexes.
/// Coarse/global — fine for single-project local mode; a path change re-indexes anyway.
static INDEX_RUNNING: AtomicBool = AtomicBool::new(false);

/// Index a project into the PostgreSQL `code_chunks` table (awaitable).
///
/// Walks the project, chunks each source file, embeds each chunk via Ollama
/// (config.toml), and upserts via `CodeChunkRepo`. Returns a stats object.
/// Pool failure returns `Err` so the caller can fall back; per-chunk embed/upsert
/// failures are logged and skipped (chunk stored without an embedding).
pub async fn index_project_pg(project_path: &str) -> Result<Value, String> {
    let pool = crate::pg::get_pool_wait(std::time::Duration::from_secs(10))
        .await
        .ok_or_else(|| "PostgreSQL pool not ready".to_string())?;

    let canonical = normalize_project_path(project_path);
    let repo = CodeChunkRepo::new(pool);
    let files = collect_files(project_path);

    let mut files_indexed = 0usize;
    let mut chunks_total = 0usize;
    let mut embedded = 0usize;
    let mut embed_failed = 0usize;

    for (file_path, ext) in &files {
        let language = ext_to_language(ext).to_string();
        let Ok(content) = std::fs::read_to_string(file_path) else {
            continue;
        };
        files_indexed += 1;

        let chunks = chunk_file(&content, file_path, &language);
        for chunk in chunks {
            chunks_total += 1;

            // Embed via Ollama reading ~/.masday/config.toml. generate_embedding has a
            // 15s timeout; on failure we store the chunk with no embedding so a later
            // re-index can fill it in without losing the chunk entirely.
            let embedding = match crate::tools::local::generate_embedding(&chunk.content).await {
                Ok(v) => {
                    embedded += 1;
                    Some(v.into_iter().map(|x| x as f32).collect::<Vec<f32>>())
                }
                Err(e) => {
                    embed_failed += 1;
                    warn!(
                        "PG code index: embed failed {}:{} — {}",
                        file_path, chunk.start_line, e
                    );
                    None
                }
            };

            let new_chunk = NewCodeChunk {
                project_path: canonical.clone(),
                file_path: chunk.file_path.clone(),
                language: chunk.language.clone(),
                chunk_type: chunk.chunk_type.clone(),
                name: chunk.name.clone(),
                start_line: chunk.start_line as i32,
                end_line: chunk.end_line as i32,
                content: chunk.content.clone(),
                content_hash: content_hash(&chunk.content),
                embedding,
            };

            if let Err(e) = repo.upsert_chunk(&new_chunk).await {
                warn!(
                    "PG code index: upsert failed {}:{} — {}",
                    file_path, chunk.start_line, e
                );
            }
        }
    }

    info!(
        "PG code index complete: {} files, {} chunks, {} embedded, {} failed [{}]",
        files_indexed, chunks_total, embedded, embed_failed, canonical
    );

    Ok(json!({
        "project_path": canonical,
        "files_indexed": files_indexed,
        "chunks_total": chunks_total,
        "embedded": embedded,
        "embed_failed": embed_failed,
    }))
}

/// Fire-and-forget background index. Spawns at most one concurrent index task;
/// calls while one is already running are no-ops. Never blocks the caller.
///
/// Use from the search path: if the project has no embedded chunks yet, trigger this
/// and return the SQLite fallback immediately; pgvector results appear once indexing
/// finishes.
pub fn trigger_background_index(project_path: &str) {
    // CAS guards the spawn — avoid duplicate concurrent index tasks.
    if INDEX_RUNNING.swap(true, Ordering::SeqCst) {
        return;
    }
    let path = project_path.to_string();
    tokio::spawn(async move {
        let result = index_project_pg(&path).await;
        INDEX_RUNNING.store(false, Ordering::SeqCst);
        match result {
            Ok(stats) => info!("Background PG code index finished: {}", stats),
            Err(e) => warn!("Background PG code index aborted: {}", e),
        }
    });
}

#[cfg(test)]
mod tests {
    use masday_db::repos::normalize_project_path;

    #[test]
    fn test_normalize_relative_dot() {
        // "." canonicalizes to the current working directory (non-empty, absolute).
        let normalized = normalize_project_path(".");
        assert!(!normalized.is_empty());
        assert!(
            normalized.starts_with('/'),
            "canonical path should be absolute, got: {}",
            normalized
        );
    }

    #[test]
    fn test_normalize_missing_path_falls_back() {
        // A path that doesn't exist falls back to the input verbatim (no panic).
        let normalized = normalize_project_path("/this/does/not/exist/xyzzy");
        assert_eq!(normalized, "/this/does/not/exist/xyzzy");
    }
}
