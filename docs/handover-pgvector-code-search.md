# Handover — Build Full pgvector Code Search (Option B)

**Created:** 2026-06-17 · **Baseline release:** v0.3.72 (deadlock fix + embed diagnostics already shipped)
**Goal:** Make `semantic-search_code_search` return real pgvector semantic results (`"source":"pgvector"`) over indexed code, instead of the weak SQLite feature-hash fallback.

> **🔑 Key correction (from maintainer):** `run_local()` currently does NOT call `client::init()`, which is a **bug**. Local mode mandates PostgreSQL + Redis + the API server, so `run_local` MUST init the API client (read api_url/api_key from `~/.masday/config.toml`). Only `run_stdio()` (standalone) is SQLite-only. **Fixing this is the prerequisite** — it activates the MCP's existing Priority-1 API path, so option B mostly becomes an API-side pgvector upgrade, not an MCP PG-direct reimplementation.

---

## 1. Why / current state

- The `code_search` stale/hang bug is FIXED (v0.3.72). Tool now returns in ~0.1s but results are thin because the SQLite `code_chunks` index for `project_path="."` has only **2 entries**.
- User wants the **pg** source for code search. Investigation shows **pgvector code search does not exist yet** — only pgvector *memory* search does.

## 2. The 3 code_search sources (findings)

| Source | Origin | Reality |
|---|---|---|
| `sqlite_feature_hash` | MCP `direct.rs:2366` Priority 2 → `code_index::search_code` | Works, offline, feature-hashing (no embeddings). Weak with small index. |
| `pgvector_api` | MCP `direct.rs` Priority 1 → `client::api_get("/api/context/search")` | Endpoint is **BM25/text** (`masday-api/src/routes/context.rs` `code_search`: "uses filesystem + BM25"), NOT pgvector. **BUG:** Priority 1 is currently dead because `run_local()` (masday-mcp/src/lib.rs:898) never calls `client::init()` → `try_get_api_url()` is `None`. But local mode **mandates PostgreSQL + Redis + the API server**, so `run_local` **SHOULD** init the API client (only `standalone`/`run_stdio` is SQLite-only, no API). Fixing this activates Priority 1. |
| `"source":"pgvector"` | `masday-service/src/search_service.rs` (`1 - (embedding <=> $1::vector)`) | TRUE pgvector cosine — **but queries the `"Memory"` table only, not code.** |

**PG proof:** `code_chunks` table does NOT exist in PG; only `indexed_files` (no embedding vector column). So there is no vector-indexed code to search.

## 3. Reference pattern to copy

`masday-service/src/search_service.rs:30-60` — the pgvector memory search. Copy this shape for code:

```sql
SELECT id, content, summary, "memoryType",
       1 - (embedding <=> $1::vector) as similarity
FROM "Memory"
WHERE embedding IS NOT NULL
ORDER BY embedding <=> $1::vector
LIMIT $2
```

Query embedding comes from `EmbeddingService` (masday-service/src/embedding_service.rs) — supports local/ollama/openai.

## 4. Implementation plan (Option B)

### Phase 1 — PG schema + repo
1. **Migration** `masday-db/migrations/NNN_code_chunks_pgvector.sql` — mirror the SQLite `code_chunks` schema + add `embedding vector(768)` + `CREATE INDEX ... USING hnsw (embedding vector_cosine_ops)`. Columns: `id uuid, project_path text, file_path text, language text, chunk_type text, name text, start_line int, end_line int, content text, content_hash text, embedding vector(768), indexed_at timestamptz`.
2. **Also embed it** in `masday-mcp/src/pg.rs` `run_embedded_migrations()` (const `MIGRATION_SQL` include_str) so MCP local mode self-applies it (the MCP runs embedded migrations, not sqlx migrate — see pg.rs:22 + :87).
3. **Repo** `masday-db/src/repos/code_chunk_repo.rs` + register in `masday-db/src/repos/mod.rs`. Methods: `upsert_chunk`, `vector_search(pool, query_vec: &[f32], project_path, limit) -> Vec<CodeChunkResult>` using `<=>`. Follow existing repo patterns (deadpool-postgres, raw SQL).

### Phase 2 — Indexer
4. Reuse chunking logic from `masday-mcp/src/code_index.rs` (`chunk_file`, `collect_files`, `content_hash`, ignore rules — it already excludes target/node_modules/.git).
5. Embed each chunk via **Ollama** (config is now ollama/nomic-embed-text/768). MCP cannot use `EmbeddingService` (see constraint §6), so call Ollama HTTP directly — copy `masday-mcp/src/tools/local.rs:generate_embedding` ollama arm (`POST {base}/api/embeddings`, model from `pg::read_embedding_model()`). Returns Vec<f64> → cast to f32.
6. Indexer entry point: lazy-index on first `code_search` (mirror the SQLite `chunk_count==0 → index_project` path in `code_index.rs:476`) OR a `masday embed index` CLI command. Recommend lazy-index first (matches existing UX).

### Phase 3 — Fix `run_local` to init the API client (PREREQUISITE BUG FIX)
7. `masday-mcp/src/lib.rs:run_local()` must call `client::init(api_url, api_key)` reading **from `~/.masday/config.toml`** (use `pg::read_api_url()` + read `api_key`), because **local mode mandates PostgreSQL + Redis + the API server** (localhost:30101). Only `run_stdio()` (standalone) is SQLite-only and skips the API client. After this fix, `direct.rs:2366` Priority 1 (`client::api_get("/api/context/search")`) becomes active → code_search flows through the API like remote mode.
8. Upgrade API endpoint `/api/context/search` (`masday-api/src/routes/context.rs` `code_search` + a `SearchService::code_search`) to do **pgvector** over the new `code_chunks` table (copy the `<=>` pattern from `search_service.rs:30-60`), falling back to current BM25 if no embeddings. MCP `direct.rs` then needs **no PG-direct path** — it just calls the upgraded endpoint.
9. (Optional, resilient) If you want local-mode code_search to work even when the API server is briefly down, keep the SQLite feature-hash as the final fallback (Priority 2) — already in place.

### Phase 4 — Verify
10. `sqlx migrate run` (or let MCP self-migrate via `pg.rs::run_embedded_migrations`), start `masday-api`, trigger one `code_search` from repo root (forces lazy index via the API), then RPC `semantic-search_code_search` → expect `"source":"pgvector"` with real `.rs`/`.ts` chunks + similarity scores.

## 5. Files to touch

| File | Change |
|---|---|
| `masday-db/migrations/NNN_code_chunks_pgvector.sql` | NEW — schema + hnsw index |
| `masday-mcp/src/pg.rs` | extend `MIGRATION_SQL`; (optional) helper to run the new migration |
| `masday-db/src/repos/code_chunk_repo.rs` + `mod.rs` | NEW — upsert + vector_search |
| `masday-mcp/src/direct.rs:2366` | add PG pgvector priority to `semantic_search_code_search` |
| `masday-mcp/src/code_index.rs` | reuse `chunk_file`/`collect_files` for the PG indexer |
| `masday-cli/src/commands/embed.rs` | (optional) `masday embed index` subcommand |
| Tests | repo unit test + RPC e2e |

## 6. Critical constraints (don't violate)

- **`masday-mcp` depends on `masday-service` with `default-features=false`** (masday-mcp/Cargo.toml:22). So `local-embeddings`/fastembed is NOT compiled into the MCP. → **Call Ollama HTTP directly** for embeddings in MCP code (like `local.rs` does), do NOT use `EmbeddingService`.
- **Read config from `~/.masday/config.toml`, not env** (user hard constraint, "production"). Use `masday-mcp/src/pg.rs::read_config_value` / `read_embedding_*` helpers (already added in v0.3.72).
- **Dimension match:** PG index `vector(768)` must equal model dims. nomic-embed-text = 768 ✓. If model changes, index dims must change too.
- **Resilience:** any PG/embedding failure must fall through to SQLite feature-hash (never hang — the v0.3.72 deadlock fix + the 15s Ollama timeout in `local.rs` exist for this reason).
- **SQLite Mutex is non-reentrant** (`sqlite::conn()` returns MutexGuard). Don't re-acquire while holding — see the v0.3.72 deadlock fix in `code_index.rs`.

## 7. Environment (current, verified)

- **PostgreSQL:** `localhost:54341`, db `masday_workflow`, user `postgres` / `postgres`. `indexed_files` table exists; `code_chunks` does NOT (must create).
- **Ollama:** running on `11434`, `nomic-embed-text:latest` pulled.
- **config.toml** (`~/.masday/config.toml`): `embedding_provider="ollama"`, `embedding_model="nomic-embed-text"`, `embedding_dimensions=768`, `mode="local"`, `api_url=http://localhost:30101`.
- **API server** running (`./target/debug/masday-api`, pid was 2634715). `masday-api` HAS `local-embeddings` feature.
- **Installed MCP binary:** `/home/vibe-dev/.masday/bin/masday` (masday-cli release, v0.3.72). `masday mcp` subcommand = what Claude Code spawns.
- **SQLite state:** `~/.masday/data.db` `code_chunks` has only 2 rows for `project_path="."`.

## 8. Quick-start commands for next session

```bash
source ~/.cargo/env
# Inspect current PG tables
PGPASSWORD=postgres psql -h localhost -p 54341 -U postgres -d masday_workflow -c "\dt"
# Reference: memory pgvector search already works here
PGPASSWORD=postgres psql -h localhost -p 54341 -U postgres -d masday_workflow -c \
  'SELECT id, "memoryType", embedding IS NOT NULL FROM "Memory" LIMIT 3;'
# Test current MCP code_search via RPC (confirms baseline)
cat <<'EOF' | ./target/release/masday mcp 2>/dev/null | grep '"id":10'
{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}
{"jsonrpc":"2.0","method":"notifications/initialized"}
{"jsonrpc":"2.0","id":10,"method":"tools/call","params":{"name":"semantic-search_code_search","arguments":{"query":"workflow transitions"}}}
EOF
```

## 9. Related memory / context

- `~/.claude/projects/.../memory/mcp-code-search-deadlock-fix.md` — the v0.3.72 fix + embed diagnostics details.
- `~/.claude/projects/.../memory/pgvector-search-upgrade.md` — prior "Task #3: upgrade search to pgvector" (check what was already done).
- Release process: tag push `v*` → CI auto-publishes (see `.claude/CLAUDE.md` + `docs/release-guide.md`). Bump all 6 `Cargo.toml` + `Cargo.lock`.
