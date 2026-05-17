-- pgvector extension setup
-- Run this after the initial Prisma migration to enable vector columns.
-- Requires pgvector to be installed in the PostgreSQL instance.

CREATE EXTENSION IF NOT EXISTS vector;
