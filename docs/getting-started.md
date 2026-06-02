# Getting Started

Masday Workflow is designed to run **locally first**. Rust workspace with 6 crates, SQLite-backed stdio MCP server (no external database needed), and optional PostgreSQL-backed API server.

## Quick Start (Rust CLI)

```bash
# From project root — build the CLI binary
cargo build --release -p masday-cli

# Install masday into current project (local mode)
./target/release/masday install

# Or connect to a remote API server
./target/release/masday install --remote https://api.example.com:30101 --api-key <key>
```

See [CLI commands reference](./reference/cli-commands.md) for full command documentation.

### What `masday install` distributes

The `masday` binary is **self-contained** (~7.6MB). It embeds all templates at compile time:

| What user gets | What user does NOT get |
|----------------|----------------------|
| `masday` CLI binary (with embedded templates) | Root project source code |
| `masday-mcp` MCP server binary (~2.4MB) | Cargo workspace / Rust source |
| 28 agent .md files (extracted from binary) | PostgreSQL schema (remote mode) |
| 30+ skill directories (extracted from binary) | Dashboard frontend |
| Hooks (global + project, extracted from binary) | |
| MCP configs (generated per platform) | |
| settings.json updates (statusline, autoCompact) | |

**Local mode** requires Rust toolchain (builds from source).
**Remote mode** only needs the binary — no Rust, no source code.
**Standalone mode** extracts templates only (no build, no API server).

## Default local workflow (Developer)

```bash
source ~/.cargo/env

# Build all crates
cargo build --workspace

# Start MCP stdio server (SQLite — no database setup needed)
cargo run -p masday-mcp

# Or start API server (requires PostgreSQL)
DATABASE_URL=postgresql://USER:PASS@localhost:54341/masday_workflow \
  cargo run -p masday-api
```

### Database Setup

**MCP stdio server (local mode):** Uses SQLite at `~/.masday/data.db` — auto-created, zero config. No environment variables needed.

**API server / remote mode:** Requires PostgreSQL on port 54341 and environment variables.

```bash
# Start PostgreSQL + Redis (API server only)
docker compose up -d

# Database: masday_workflow (see .env for credentials, port: 54341)
# Tables are created by the Rust application on first run
```

**Environment variables for remote mode (API server only):**

```env
# ── API Server ──
DATABASE_URL="postgresql://USER:PASS@localhost:54341/masday_workflow"
MASDAY_API_KEY="your-api-key"
```

Set these on the server running `masday-api`. MCP clients connect directly via HTTP/SSE — no env vars needed on the client side.

## What this starts

- **MCP stdio server** (`masday-mcp`) with 20 tool domains, SQLite-backed (zero config)
- **REST API server** (`masday-api`, optional) on port 30101 with 243 routes, PostgreSQL-backed
- **SQLite-backed state** (stdio mode) — 16 tables auto-created at `~/.masday/data.db`
- **PostgreSQL-backed state** (API mode) — 16 tables for workflow, task, memory, review, session, graph, etc.
- **Workflow state machine** — auto-transition to DONE when all tasks complete
- **4-layer memory** — working, episodic, long-term, knowledge graph
- **Project-local `.masday/`** artifacts for cached research, plans, and notes

## Recommended reading order

1. [Architecture](./architecture.md) — runtime shape and crate structure
2. [Workflow lifecycle](./workflows/lifecycle.md) — lifecycle states and execution model
3. [Local development](./workflows/local-development.md) — local-first commands and expectations
4. [MCP tools reference](./reference/mcp-tools.md) — tool surface summary
5. [CLI commands reference](./reference/cli-commands.md) — Claude/CLI command map
6. [State model](./reference/state-model.md) — PostgreSQL vs `.masday` responsibilities
