-- Code index table for semantic search (PostgreSQL mode)
CREATE TABLE IF NOT EXISTS indexed_files (
    id TEXT PRIMARY KEY DEFAULT gen_random_uuid()::text,
    file_path TEXT UNIQUE NOT NULL,
    content TEXT NOT NULL,
    language TEXT NOT NULL,
    indexed_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_indexed_files_language ON indexed_files(language);
