-- pgvector extension setup for masday-workflow-rebuild
-- Run this once against your PostgreSQL/Supabase database after the
-- initial Prisma migration (pnpm db:push / prisma migrate deploy).
--
-- Supabase: run in the SQL Editor at https://supabase.com/dashboard
-- Local PostgreSQL: psql -d <dbname> -f packages/db/sql/pgvector.sql

-- 1. Enable the pgvector extension (already enabled on Supabase by default)
CREATE EXTENSION IF NOT EXISTS vector;

-- 2. Add vector columns to Memory table (768 dim — matches BGEBaseENV15 / nomic-embed-text)
ALTER TABLE "Memory"
  ADD COLUMN IF NOT EXISTS embedding vector(768);

-- 3. Add vector column to ContextDocument table
ALTER TABLE "ContextDocument"
  ADD COLUMN IF NOT EXISTS embedding vector(768);

-- 4. HNSW indexes for fast ANN search (recommended for large datasets)
CREATE INDEX IF NOT EXISTS memory_embedding_hnsw
  ON "Memory" USING hnsw (embedding vector_cosine_ops);

CREATE INDEX IF NOT EXISTS context_doc_embedding_hnsw
  ON "ContextDocument" USING hnsw (embedding vector_cosine_ops);

-- Verify setup
SELECT table_name, column_name, data_type
FROM information_schema.columns
WHERE column_name = 'embedding'
  AND table_schema = 'public';
