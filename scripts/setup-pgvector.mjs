#!/usr/bin/env node
// scripts/setup-pgvector.mjs
// Runs pgvector setup against the configured PostgreSQL database.
// Reads EMBEDDING_DIMENSIONS from .env (defaults to 768).
//
// Usage:
//   node scripts/setup-pgvector.mjs
//   EMBEDDING_DIMENSIONS=384 node scripts/setup-pgvector.mjs

import { readFileSync } from "fs";
import { createRequire } from "module";
import { resolve, dirname } from "path";
import { fileURLToPath } from "url";

const __dirname = dirname(fileURLToPath(import.meta.url));
const root = resolve(__dirname, "..");

// Load .env manually (no dotenv dep needed)
try {
  const envText = readFileSync(resolve(root, ".env"), "utf8");
  for (const line of envText.split("\n")) {
    const trimmed = line.trim();
    if (!trimmed || trimmed.startsWith("#")) continue;
    const eq = trimmed.indexOf("=");
    if (eq === -1) continue;
    const key = trimmed.slice(0, eq).trim();
    const val = trimmed.slice(eq + 1).trim().replace(/^["']|["']$/g, "");
    if (!(key in process.env)) process.env[key] = val;
  }
} catch { /* .env not found — rely on shell env */ }

const dims = parseInt(process.env.EMBEDDING_DIMENSIONS ?? "768", 10);
const dbUrl = process.env.DATABASE_URL;

if (!dbUrl || dbUrl === "sqlite://local") {
  console.error("DATABASE_URL must be a PostgreSQL connection string to run pgvector setup.");
  process.exit(1);
}

const require = createRequire(import.meta.url);

// Use pg directly — it's a transitive dep of Prisma already installed
let pg;
try {
  pg = require("pg");
} catch {
  console.error("pg package not found. Run: pnpm add -D pg");
  process.exit(1);
}

const { Client } = pg;
const client = new Client({ connectionString: dbUrl });

const sql = `
CREATE EXTENSION IF NOT EXISTS vector;

ALTER TABLE "Memory"
  ADD COLUMN IF NOT EXISTS embedding vector(${dims});

ALTER TABLE "ContextDocument"
  ADD COLUMN IF NOT EXISTS embedding vector(${dims});

CREATE INDEX IF NOT EXISTS memory_embedding_hnsw
  ON "Memory" USING hnsw (embedding vector_cosine_ops);

CREATE INDEX IF NOT EXISTS context_doc_embedding_hnsw
  ON "ContextDocument" USING hnsw (embedding vector_cosine_ops);
`;

console.log(`Setting up pgvector with ${dims}-dim vectors...`);

await client.connect();
try {
  await client.query(sql);
  const { rows } = await client.query(`
    SELECT table_name, column_name, data_type
    FROM information_schema.columns
    WHERE column_name = 'embedding' AND table_schema = 'public'
  `);
  console.log("Done. Embedding columns:");
  for (const r of rows) console.log(`  ${r.table_name}.${r.column_name} (${r.data_type})`);
} finally {
  await client.end();
}
