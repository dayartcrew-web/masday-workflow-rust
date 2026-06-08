//! Code indexing and semantic search module.
//!
//! Chunks source files into meaningful pieces, embeds each chunk via the
//! feature-hashing vectorizer, stores in SQLite `code_chunks` table,
//! and ranks results by cosine similarity on search.

use crate::embedding::{blob_to_vector, cosine_similarity, text_to_vector, vector_to_blob};
use rusqlite::params;
use serde_json::{json, Value};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use tracing::info;

/// Minimum cosine similarity threshold for a result to be included.
const SIMILARITY_THRESHOLD: f32 = 0.10;

/// Default maximum results to return.
#[allow(dead_code)]
const DEFAULT_LIMIT: usize = 20;

/// Maximum lines per chunk before splitting.
const MAX_CHUNK_LINES: usize = 120;

/// Target split point within oversized chunks.
const SPLIT_TARGET: usize = 60;

/// Minimum lines for a standalone chunk (smaller merged with previous).
const MIN_CHUNK_LINES: usize = 2;

/// A single code chunk extracted from a source file.
#[derive(Debug, Clone)]
struct CodeChunk {
    file_path: String,
    language: String,
    chunk_type: String,
    name: Option<String>,
    start_line: usize,
    end_line: usize,
    content: String,
}

/// Deterministic content hash using DefaultHasher.
fn content_hash(content: &str) -> String {
    let mut hasher = DefaultHasher::new();
    content.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

/// Detect structure keywords that start a new chunk boundary.
fn is_structure_boundary(line: &str) -> bool {
    let trimmed = line.trim_start();
    let prefixes = [
        "pub fn ",
        "pub async fn ",
        "async fn ",
        "fn ",
        "pub struct ",
        "struct ",
        "pub enum ",
        "enum ",
        "pub trait ",
        "trait ",
        "impl ",
        "pub impl ",
        "mod ",
        "pub mod ",
        "class ",
        "def ",
        "async def ",
        "function ",
        "export function ",
        "export async function ",
        "interface ",
        "type ",
        "export type ",
        "const ",
        "export const ",
    ];
    prefixes.iter().any(|p| trimmed.starts_with(p))
}

/// Extract a name from a structure boundary line.
fn extract_name(line: &str) -> Option<String> {
    let trimmed = line.trim_start();
    // Remove common prefixes
    let stripped = trimmed
        .trim_start_matches("pub ")
        .trim_start_matches("async ")
        .trim_start_matches("export ")
        .trim_start_matches("type ")
        .trim_start_matches("const ");

    let prefixes = ["fn ", "struct ", "enum ", "trait ", "impl ", "mod ", "class ", "def ", "function ", "interface "];
    for prefix in &prefixes {
        if let Some(rest) = stripped.strip_prefix(prefix) {
            let name = rest
                .split(|c: char| c == '(' || c == '<' || c == '{' || c == ':' || c.is_whitespace())
                .next()
                .unwrap_or("")
                .trim()
                .trim_start_matches('<')
                .to_string();
            if !name.is_empty() {
                return Some(name);
            }
        }
    }
    None
}

/// Map file extension to language identifier.
fn ext_to_language(ext: &str) -> &str {
    match ext {
        "rs" => "rust",
        "ts" | "tsx" => "typescript",
        "js" | "jsx" => "javascript",
        "py" => "python",
        "toml" => "toml",
        "json" => "json",
        "md" => "markdown",
        "go" => "go",
        "java" => "java",
        "c" | "h" => "c",
        "cpp" | "hpp" => "cpp",
        _ => ext,
    }
}

/// Chunk a single file's content into searchable pieces.
fn chunk_file(content: &str, file_path: &str, language: &str) -> Vec<CodeChunk> {
    let lines: Vec<&str> = content.lines().collect();
    if lines.is_empty() {
        return Vec::new();
    }

    let mut chunks = Vec::new();

    // Find all boundary line indices
    let mut boundaries: Vec<usize> = Vec::new();
    for (i, line) in lines.iter().enumerate() {
        if i > 0 && is_structure_boundary(line) {
            boundaries.push(i);
        }
    }

    // Build chunk ranges from boundaries
    let mut ranges: Vec<(usize, usize, String, Option<String>)> = Vec::new();

    if boundaries.is_empty() || boundaries[0] > 0 {
        // File header chunk (imports, module docs, etc.)
        let end = boundaries.first().copied().unwrap_or(lines.len());
        ranges.push((0, end, "file_header".to_string(), None));
    }

    for (idx, &start) in boundaries.iter().enumerate() {
        let end = boundaries.get(idx + 1).copied().unwrap_or(lines.len());
        let name = extract_name(lines[start]);
        let chunk_type = if lines[start].trim_start().starts_with("fn")
            || lines[start].trim_start().starts_with("pub fn")
            || lines[start].trim_start().starts_with("async fn")
            || lines[start].trim_start().starts_with("pub async fn")
            || lines[start].trim_start().starts_with("def ")
            || lines[start].trim_start().starts_with("async def ")
            || lines[start].trim_start().starts_with("function ")
        {
            "function"
        } else if lines[start].trim_start().starts_with("struct")
            || lines[start].trim_start().starts_with("pub struct")
        {
            "struct"
        } else if lines[start].trim_start().starts_with("impl")
            || lines[start].trim_start().starts_with("pub impl")
        {
            "impl"
        } else if lines[start].trim_start().starts_with("enum")
            || lines[start].trim_start().starts_with("pub enum")
        {
            "enum"
        } else if lines[start].trim_start().starts_with("trait")
            || lines[start].trim_start().starts_with("pub trait")
        {
            "trait"
        } else {
            "block"
        };
        ranges.push((start, end, chunk_type.to_string(), name));
    }

    // Build chunks from ranges, handling oversized ones
    for (start, end, chunk_type, name) in ranges {
        let chunk_lines: Vec<(&str, usize)> = (start..end).map(|i| (lines[i], i)).collect();

        // Split oversized chunks at blank lines
        if chunk_lines.len() > MAX_CHUNK_LINES {
            let mut split_idx = SPLIT_TARGET;
            // Find nearest blank line after target
            for i in SPLIT_TARGET..chunk_lines.len().min(SPLIT_TARGET + 30) {
                if chunk_lines[i].0.trim().is_empty() {
                    split_idx = i + 1;
                    break;
                }
            }
            let first: Vec<(&str, usize)> = chunk_lines[..split_idx].to_vec();
            let second: Vec<(&str, usize)> = chunk_lines[split_idx..].to_vec();
            push_chunk(&mut chunks, first, file_path, language, &chunk_type, name.clone());
            push_chunk(&mut chunks, second, file_path, language, &chunk_type, None);
        } else {
            push_chunk(&mut chunks, chunk_lines, file_path, language, &chunk_type, name);
        }
    }

    // Merge small chunks with previous
    let mut merged: Vec<CodeChunk> = Vec::new();
    for chunk in chunks {
        if let Some(last) = merged.last_mut() {
            if chunk.content.lines().count() < MIN_CHUNK_LINES {
                // Merge with previous
                last.end_line = chunk.end_line;
                last.content = format!("{}\n{}", last.content, chunk.content);
                continue;
            }
        }
        merged.push(chunk);
    }

    merged
}

/// Helper to build a CodeChunk from line data.
fn push_chunk(
    chunks: &mut Vec<CodeChunk>,
    lines: Vec<(&str, usize)>,
    file_path: &str,
    language: &str,
    chunk_type: &str,
    name: Option<String>,
) {
    if lines.is_empty() {
        return;
    }
    let start_line = lines[0].1 + 1; // 1-based
    let end_line = lines.last().unwrap().1 + 1;
    let content = lines.iter().map(|(l, _)| *l).collect::<Vec<_>>().join("\n");
    chunks.push(CodeChunk {
        file_path: file_path.to_string(),
        language: language.to_string(),
        chunk_type: chunk_type.to_string(),
        name,
        start_line,
        end_line,
        content,
    });
}

/// Check if a file extension should be indexed.
fn should_index(ext: &str) -> bool {
    matches!(
        ext,
        "rs"
            | "ts"
            | "tsx"
            | "js"
            | "jsx"
            | "py"
            | "toml"
            | "md"
            | "go"
            | "java"
            | "c"
            | "h"
            | "cpp"
            | "hpp"
    )
}

/// Directories to skip during indexing.
const SKIP_DIRS: &[&str] = &[
    "target",
    "node_modules",
    ".git",
    "dist",
    "build",
    ".next",
    "__pycache__",
    ".venv",
    "out",
    ".masday",
    "coverage",
    ".turbo",
];

/// Recursively collect files to index.
fn collect_files(project_path: &str) -> Vec<(String, String)> {
    let mut files = Vec::new();
    collect_files_inner(project_path, &mut files);
    files
}

fn collect_files_inner(dir: &str, files: &mut Vec<(String, String)>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };

    for entry in entries.flatten() {
        let path = entry.path();

        if path.is_dir() {
            if let Some(name) = path.file_name() {
                let name_str = name.to_string_lossy();
                if SKIP_DIRS.iter().any(|&d| name_str == d) {
                    continue;
                }
            }
            collect_files_inner(path.to_str().unwrap_or(""), files);
        } else if let Some(ext) = path.extension() {
            let ext_str = ext.to_string_lossy();
            if should_index(&ext_str) {
                if let Some(p) = path.to_str() {
                    files.push((p.to_string(), ext_str.to_string()));
                }
            }
        }
    }
}

/// Index a project: walk files, chunk, embed, store in SQLite.
///
/// Uses content hashing for incremental updates — only re-indexes changed files.
pub fn index_project(project_path: &str) -> Result<usize, Box<dyn std::error::Error + Send + Sync>> {
    info!("Indexing project: {}", project_path);
    let files = collect_files(project_path);
    let mut indexed = 0usize;

    let conn = crate::sqlite::conn();

    for (file_path, ext) in &files {
        let language = ext_to_language(ext).to_string();

        let Ok(content) = std::fs::read_to_string(file_path) else {
            continue;
        };

        // Check if file has changed via content hash
        let file_hash = content_hash(&content);

        let existing_hash: Option<String> = conn
            .query_row(
                "SELECT content_hash FROM code_chunks WHERE file_path = ?1 LIMIT 1",
                [file_path],
                |row| row.get(0),
            )
            .ok();

        if existing_hash.as_deref() == Some(&file_hash) {
            continue; // File unchanged, skip
        }

        // Delete old chunks for this file
        let _ = conn.execute("DELETE FROM code_chunks WHERE file_path = ?1", [file_path]);

        // Chunk the file
        let chunks = chunk_file(&content, file_path, &language);

        for chunk in chunks {
            let id = uuid::Uuid::new_v4().to_string();
            let chunk_hash = content_hash(&chunk.content);
            let embedding_vec = text_to_vector(&chunk.content);
            let embedding_blob = vector_to_blob(&embedding_vec);

            conn.execute(
                "INSERT INTO code_chunks (id, project_path, file_path, language, chunk_type, name, start_line, end_line, content, content_hash, embedding, indexed_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, datetime('now'))",
                params![
                    id,
                    project_path,
                    chunk.file_path,
                    chunk.language,
                    chunk.chunk_type,
                    chunk.name,
                    chunk.start_line as i64,
                    chunk.end_line as i64,
                    chunk.content,
                    chunk_hash,
                    &embedding_blob as &[u8],
                ],
            )?;

            indexed += 1;
        }
    }

    info!("Indexed {} chunks from {} files", indexed, files.len());
    Ok(indexed)
}

/// Check if the index is stale and re-index changed files.
fn refresh_stale_chunks(
    conn: &rusqlite::Connection,
    project_path: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Get distinct indexed files
    let mut stmt = conn.prepare(
        "SELECT DISTINCT file_path, content_hash, indexed_at FROM code_chunks WHERE project_path = ?1",
    )?;
    let rows: Vec<(String, String, String)> = stmt
        .query_map([project_path], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?))
        })?
        .filter_map(|r| r.ok())
        .collect();

    for (file_path, stored_hash, _indexed_at) in &rows {
        // Check if file still exists
        let Ok(content) = std::fs::read_to_string(file_path) else {
            // File deleted — remove its chunks
            let _ = conn.execute("DELETE FROM code_chunks WHERE file_path = ?1", [file_path]);
            continue;
        };

        let current_hash = content_hash(&content);
        if current_hash != *stored_hash {
            // File changed — delete old chunks (will be re-indexed by caller)
            let _ = conn.execute("DELETE FROM code_chunks WHERE file_path = ?1", [file_path]);
        }
    }

    Ok(())
}

/// Semantic code search: embed query, compare against indexed chunks, return ranked results.
///
/// Falls back to grep when no semantic results pass the similarity threshold.
pub fn search_code(
    query: &str,
    project_path: &str,
    limit: usize,
) -> Result<Vec<Value>, Box<dyn std::error::Error + Send + Sync>> {
    let conn = crate::sqlite::conn();

    // Check if index exists for this project
    let chunk_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM code_chunks WHERE project_path = ?1",
            [project_path],
            |row| row.get(0),
        )
        .unwrap_or(0);

    if chunk_count == 0 {
        // Lazy indexing — build index on first search
        drop(conn); // Release lock before indexing
        index_project(project_path)?;
        let conn2 = crate::sqlite::conn();

        let new_count: i64 = conn2
            .query_row(
                "SELECT COUNT(*) FROM code_chunks WHERE project_path = ?1",
                [project_path],
                |row| row.get(0),
            )
            .unwrap_or(0);

        if new_count == 0 {
            return Ok(vec![]);
        }
    } else {
        // Refresh stale chunks
        refresh_stale_chunks(&conn, project_path)?;
    }

    // Re-acquire connection after potential drop
    let conn = crate::sqlite::conn();

    // Embed the query
    let query_vec = text_to_vector(query);

    // Fetch all chunks for this project — collect into a Vec first, then drop stmt and conn
    let rows = {
        let mut stmt = conn.prepare(
            "SELECT id, file_path, language, chunk_type, name, start_line, end_line, content, embedding
             FROM code_chunks WHERE project_path = ?1 AND embedding IS NOT NULL",
        )?;

        let mapped = stmt.query_map([project_path], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, Option<String>>(4)?,
                row.get::<_, i64>(5)?,
                row.get::<_, i64>(6)?,
                row.get::<_, String>(7)?,
                row.get::<_, Vec<u8>>(8)?,
            ))
        })?;

        let collected: Vec<(String, String, String, String, Option<String>, i64, i64, String, Vec<u8>)> =
            mapped.filter_map(|r| r.ok()).collect();
        collected
    };
    // stmt and its borrow on conn dropped here

    drop(conn); // Release lock before computing similarities

    // Compute cosine similarity for each chunk
    let mut scored: Vec<(Value, f32)> = Vec::new();
    for (id, file_path, language, chunk_type, name, start_line, end_line, content, embedding_blob) in rows {
        let candidate_vec = blob_to_vector(&embedding_blob);
        if candidate_vec.is_empty() {
            continue;
        }

        let similarity = cosine_similarity(&query_vec, &candidate_vec);
        if similarity >= SIMILARITY_THRESHOLD {
            scored.push((
                json!({
                    "id": id,
                    "file_path": file_path,
                    "language": language,
                    "chunk_type": chunk_type,
                    "name": name,
                    "start_line": start_line,
                    "end_line": end_line,
                    "content": content,
                    "similarity": (similarity as f64),
                }),
                similarity,
            ));
        }
    }

    // Sort by similarity descending
    scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    scored.truncate(limit);

    let results: Vec<Value> = scored.into_iter().map(|(v, _)| v).collect();

    // Fallback to grep if no semantic results
    if results.is_empty() {
        return grep_fallback(query, project_path, limit);
    }

    Ok(results)
}

/// Grep fallback when semantic search returns no results.
fn grep_fallback(
    query: &str,
    project_path: &str,
    limit: usize,
) -> Result<Vec<Value>, Box<dyn std::error::Error + Send + Sync>> {
    let output = std::process::Command::new("grep")
        .args([
            "-rn",
            "--include=*.rs",
            "--include=*.ts",
            "--include=*.js",
            "--include=*.py",
            "--include=*.go",
            "--include=*.java",
            "--exclude-dir=node_modules",
            "--exclude-dir=target",
            "--exclude-dir=.git",
            "--exclude-dir=dist",
            "--exclude-dir=build",
            "--exclude-dir=.next",
            "--exclude-dir=__pycache__",
            "--exclude-dir=.venv",
            "--exclude-dir=out",
            query,
            project_path,
        ])
        .output();

    match output {
        Ok(out) => {
            let results: Vec<Value> = String::from_utf8_lossy(&out.stdout)
                .lines()
                .take(limit)
                .map(|line| json!({ "match": line, "source": "grep_fallback" }))
                .collect();
            Ok(results)
        }
        Err(_) => Ok(vec![]),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_content_hash_determinism() {
        let h1 = content_hash("hello world");
        let h2 = content_hash("hello world");
        assert_eq!(h1, h2, "Same content must produce same hash");
    }

    #[test]
    fn test_content_hash_different() {
        let h1 = content_hash("hello");
        let h2 = content_hash("world");
        assert_ne!(h1, h2, "Different content must produce different hash");
    }

    #[test]
    fn test_is_structure_boundary() {
        assert!(is_structure_boundary("fn foo() {"));
        assert!(is_structure_boundary("pub fn create_workflow() -> Result<()> {"));
        assert!(is_structure_boundary("async fn handle_request() {"));
        assert!(is_structure_boundary("struct Workflow {"));
        assert!(is_structure_boundary("pub struct AppState {"));
        assert!(is_structure_boundary("impl WorkflowService {"));
        assert!(is_structure_boundary("enum Status {"));
        assert!(is_structure_boundary("trait Handler {"));
        assert!(is_structure_boundary("mod tests {"));
        assert!(is_structure_boundary("def create_item():"));
        assert!(is_structure_boundary("class MyClass:"));
        assert!(is_structure_boundary("function handleClick() {"));

        assert!(!is_structure_boundary("let x = 5;"));
        assert!(!is_structure_boundary("// comment"));
        assert!(!is_structure_boundary("    inner_call();"));
    }

    #[test]
    fn test_extract_name() {
        assert_eq!(extract_name("fn create_workflow() {"), Some("create_workflow".to_string()));
        assert_eq!(extract_name("pub fn handle_request() -> Result<()> {"), Some("handle_request".to_string()));
        assert_eq!(extract_name("struct AppState {"), Some("AppState".to_string()));
        assert_eq!(extract_name("pub struct WorkflowService {"), Some("WorkflowService".to_string()));
        assert_eq!(extract_name("impl Handler for Server {"), Some("Handler".to_string()));
        assert_eq!(extract_name("enum Status {"), Some("Status".to_string()));
        assert_eq!(extract_name("def create_item():"), Some("create_item".to_string()));
        assert_eq!(extract_name("class MyClass:"), Some("MyClass".to_string()));
    }

    #[test]
    fn test_chunk_file_rust() {
        let content = r#"use std::io;

/// A simple handler
fn init() {
    println!("init");
}

pub fn create_workflow(name: &str) -> Result<()> {
    let id = Uuid::new_v4();
    Ok(())
}

struct AppState {
    pool: PgPool,
}

impl AppState {
    fn new() -> Self {
        Self { pool: Pool::new() }
    }
}
"#;
        let chunks = chunk_file(content, "src/main.rs", "rust");
        assert!(!chunks.is_empty(), "Should produce at least one chunk");

        // Should have: file_header (imports), init(), create_workflow(), AppState struct, impl AppState
        assert!(chunks.len() >= 4, "Expected at least 4 chunks, got {}", chunks.len());

        // Check that functions are detected
        let fn_chunks: Vec<&CodeChunk> = chunks.iter().filter(|c| c.chunk_type == "function").collect();
        assert!(fn_chunks.len() >= 2, "Expected at least 2 function chunks, got {}", fn_chunks.len());

        // Check names
        let names: Vec<Option<&str>> = chunks.iter().map(|c| c.name.as_deref()).collect();
        assert!(names.contains(&Some("init")), "Should find 'init' function, got {:?}", names);
        assert!(names.contains(&Some("create_workflow")), "Should find 'create_workflow' function, got {:?}", names);
    }

    #[test]
    fn test_chunk_file_python() {
        let content = r#"import os

def hello():
    print("hello")

class MyClass:
    def __init__(self):
        self.x = 1

    def process(self):
        return self.x
"#;
        let chunks = chunk_file(content, "src/main.py", "python");
        assert!(!chunks.is_empty());
        assert!(chunks.len() >= 3, "Expected at least 3 chunks for Python file");
    }

    #[test]
    fn test_chunk_file_empty() {
        let chunks = chunk_file("", "empty.rs", "rust");
        assert!(chunks.is_empty(), "Empty file should produce no chunks");
    }

    #[test]
    fn test_chunk_file_single_line() {
        let chunks = chunk_file("fn main() {}", "single.rs", "rust");
        assert!(!chunks.is_empty(), "Single function should produce a chunk");
    }

    #[test]
    fn test_ext_to_language() {
        assert_eq!(ext_to_language("rs"), "rust");
        assert_eq!(ext_to_language("ts"), "typescript");
        assert_eq!(ext_to_language("tsx"), "typescript");
        assert_eq!(ext_to_language("js"), "javascript");
        assert_eq!(ext_to_language("py"), "python");
        assert_eq!(ext_to_language("go"), "go");
    }

    #[test]
    fn test_should_index() {
        assert!(should_index("rs"));
        assert!(should_index("ts"));
        assert!(should_index("py"));
        assert!(should_index("go"));
        assert!(!should_index("exe"));
        assert!(!should_index("bin"));
        assert!(!should_index("png"));
    }

    #[test]
    fn test_search_code_returns_ranked_by_similarity() {
        use rusqlite::Connection;

        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(crate::sqlite_schema::SCHEMA).unwrap();

        // Insert test chunks with known embeddings
        let query_vec = text_to_vector("workflow state machine");
        let similar_vec = text_to_vector("workflow state transition logic");
        let different_vec = text_to_vector("color picker component");

        let similar_blob = vector_to_blob(&similar_vec);
        let different_blob = vector_to_blob(&different_vec);

        conn.execute(
            "INSERT INTO code_chunks (id, project_path, file_path, language, chunk_type, name, start_line, end_line, content, content_hash, embedding)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            rusqlite::params![
                "c1", "/test", "a.rs", "rust", "function", "transition",
                1i64, 10i64, "workflow state transition", "h1",
                &similar_blob as &[u8],
            ],
        ).unwrap();

        conn.execute(
            "INSERT INTO code_chunks (id, project_path, file_path, language, chunk_type, name, start_line, end_line, content, content_hash, embedding)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            rusqlite::params![
                "c2", "/test", "b.rs", "rust", "function", "render",
                1i64, 10i64, "color picker component", "h2",
                &different_blob as &[u8],
            ],
        ).unwrap();

        // Compute expected similarities
        let sim_similar = cosine_similarity(&query_vec, &similar_vec);
        let sim_different = cosine_similarity(&query_vec, &different_vec);

        assert!(sim_similar > sim_different, "Similar text should have higher cosine similarity");

        // Verify ranking logic: fetch from DB and compare
        let mut stmt = conn.prepare(
            "SELECT id, content, embedding FROM code_chunks WHERE project_path = ?1 ORDER BY id",
        ).unwrap();
        let rows: Vec<(String, String, Vec<u8>)> = stmt.query_map(["/test"], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?))
        }).unwrap().filter_map(|r| r.ok()).collect();

        assert_eq!(rows.len(), 2, "Should have 2 chunks");

        let mut scored: Vec<(String, f32)> = rows.iter().map(|(id, _, blob)| {
            let vec = blob_to_vector(blob);
            let sim = cosine_similarity(&query_vec, &vec);
            (id.clone(), sim)
        }).collect();
        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        assert_eq!(scored[0].0, "c1", "Most similar should be the workflow chunk");
        assert!(scored[0].1 > scored[1].1, "Workflow chunk should score higher than color picker");
    }
}
