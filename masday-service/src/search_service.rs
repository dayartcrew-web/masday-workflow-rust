//! Shared retrieval-result helper.
//!
//! Historically this module also held a filesystem `code_search` (a substring
//! scan over the project directory), pgvector/BM25 memory search, a context-pack
//! builder, codebase indexing, and a SHA-256 fingerprint helper. All of those
//! were dead or superseded:
//! - code search now runs over the shared PostgreSQL `code_chunks` index. The
//!   HTTP route (`masday_api::routes::context::code_search`) embeds the query
//!   via `EmbeddingService` and calls `CodeChunkRepo::vector_search` directly;
//!   the stdio MCP resolver (`masday_mcp::direct::resolve_code_search`) owns the
//!   pgvector → API → SQLite priority chain. The filesystem scan served no live
//!   caller — in remote/HTTP mode the API server has none of the client's files
//!   — so it has been removed along with the now-empty `SearchService`.
//! - context packs and the live `compute_fingerprint` live in `ContextService`.
//!
//! What remains is the shared `summarize_retrieval_results` helper used by both
//! the HTTP and stdio retrieval-log call sites.

use serde_json::{json, Value};

/// Compact summary of a retrieval-result body for the `retrieval_logs.results`
/// column. Keeps the row small — the match count plus a handful of sample
/// identifiers — instead of persisting the full (potentially large) result set.
///
/// Recognizes the shapes produced by the search handlers: `{"results": [...]}`
/// (code search), a bare array `[...]`, or any other object (treated as zero
/// matches). Pure (no I/O) so it is unit-testable without a database, and shared
/// by both the HTTP API route handlers and the stdio `direct.rs` handlers so the
/// persisted summary is identical on both paths.
pub fn summarize_retrieval_results(body: &Value) -> Value {
    let arr = body
        .get("results")
        .and_then(|v| v.as_array())
        .or_else(|| body.as_array());
    let count = arr.map(|a| a.len()).unwrap_or(0);
    let sample: Vec<&str> = arr
        .map(|a| {
            a.iter()
                .filter_map(|item| {
                    ["path", "id", "file", "title"]
                        .iter()
                        .find_map(|key| item.get(*key).and_then(|v| v.as_str()))
                })
                .take(5)
                .collect()
        })
        .unwrap_or_default();
    json!({ "count": count, "sample": sample })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_summarize_retrieval_results() {
        // "results" array — count + sample capped at 5, identifier key priority.
        let body = json!({
            "query": "q",
            "results": [
                {"path": "a.rs"}, {"id": "t2"}, {"title": "x"}, {}, {"file": "b.go"}, {"path": "c.rs"}
            ],
            "source": "code_search"
        });
        let s = summarize_retrieval_results(&body);
        assert_eq!(s["count"], json!(6));
        let sample = s["sample"].as_array().unwrap();
        assert_eq!(sample.len(), 5, "sample is capped at 5");
        assert_eq!(sample[0], json!("a.rs"), "path wins");
        assert_eq!(sample[1], json!("t2"), "id fallback");
        assert_eq!(sample[2], json!("x"), "title fallback");
        assert_eq!(sample[3], json!("b.go"), "file fallback");

        // Bare array.
        let s2 = summarize_retrieval_results(&json!([{"id": "1"}, {"id": "2"}]));
        assert_eq!(s2["count"], json!(2));
        assert_eq!(s2["sample"].as_array().unwrap().len(), 2);

        // Object without a "results" key → zero matches.
        let s3 = summarize_retrieval_results(&json!({"contextPack": {}}));
        assert_eq!(s3["count"], json!(0));
        assert!(s3["sample"].as_array().unwrap().is_empty());

        // Empty results array.
        let s4 = summarize_retrieval_results(&json!({"results": []}));
        assert_eq!(s4["count"], json!(0));
    }

    #[test]
    fn test_api_path_sql_has_no_pascalcase_identifiers() {
        // C2.1/C2.2 regression guard. The API-path service layer previously
        // quoted non-existent PascalCase SQL identifiers (e.g. Memory,
        // memoryType, EpisodicMemory, Workflow, Plan, Task) instead of the
        // real snake_case schema, so every API-path memory and context search
        // failed at the SQL layer. The banned tokens below are written as
        // escaped Rust string literals so this test's own source cannot
        // satisfy the assertion.
        for src in [
            include_str!("search_service.rs"),
            include_str!("memory_service.rs"),
        ] {
            for banned in [
                "\"Memory\"",
                "\"memoryType\"",
                "\"Workflow\"",
                "\"Plan\"",
                "\"Task\"",
                "\"EpisodicMemory\"",
                "\"sessionId\"",
                "\"sequenceOrder\"",
                "\"importanceScore\"",
                "\"workflowId\"",
            ] {
                assert!(
                    !src.contains(banned),
                    "PascalCase SQL identifier {banned} still present in service source"
                );
            }
        }
    }
}
