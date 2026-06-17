-- Code chunks table for chunk-level pgvector code search (PostgreSQL mode).
--
-- Mirrors the MCP SQLite `code_chunks` schema but adds a real `embedding vector(768)`
-- column populated by the Ollama-backed indexer. Chunk-level granularity (vs whole
-- files in indexed_files) gives sharper semantic matches for code search.
--
-- Dimension 768 matches nomic-embed-text (config.toml embedding_model). If the
-- embedding model changes, this index must be rebuilt with the new dimension.

CREATE TABLE IF NOT EXISTS code_chunks (
    id           UUID         PRIMARY KEY DEFAULT gen_random_uuid(),
    project_path TEXT         NOT NULL,
    file_path    TEXT         NOT NULL,
    language     TEXT         NOT NULL,
    chunk_type   TEXT         NOT NULL DEFAULT 'code',
    name         TEXT,
    start_line   INTEGER      NOT NULL,
    end_line     INTEGER      NOT NULL,
    content      TEXT         NOT NULL,
    content_hash TEXT         NOT NULL,
    embedding    vector(768),
    indexed_at   TIMESTAMPTZ  NOT NULL DEFAULT NOW()
);

-- One row per (project, file, start_line). Re-indexing upserts the chunk in place.
CREATE UNIQUE INDEX IF NOT EXISTS code_chunks_project_file_start_idx
    ON code_chunks (project_path, file_path, start_line);

-- Hash lookup for incremental re-indexing (skip chunks whose content is unchanged).
CREATE INDEX IF NOT EXISTS code_chunks_project_hash_idx
    ON code_chunks (project_path, content_hash);

-- pgvector HNSW index for fast cosine-similarity code search.
CREATE INDEX IF NOT EXISTS code_chunks_embedding_hnsw_idx
    ON code_chunks USING hnsw (embedding vector_cosine_ops);
