# Getting Started

Masday Workflow is designed to run **locally first**. Rust workspace with 6 crates, PostgreSQL persistence, and stdio MCP transport.

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
| `masday` binary (with embedded templates) | Root project source code |
| 28 agent .md files (extracted from binary) | Cargo workspace / Rust source |
| 30+ skill directories (extracted from binary) | PostgreSQL schema (remote mode) |
| Hooks (global + project, extracted from binary) | Dashboard frontend |
| MCP configs (generated per platform) | |
| settings.json updates (statusline, autoCompact) | |

**Local mode** requires Rust toolchain (builds from source).
**Remote mode** only needs the binary — no Rust, no source code.

## Default local workflow (Developer)

```bash
source ~/.cargo/env

# Build all crates
cargo build --workspace

# Start API server (port 30101)
DATABASE_URL=postgresql://trader:traderpass@localhost:54341/masday_workflow \
  cargo run -p masday-api

# Start MCP stdio server
DATABASE_URL=postgresql://trader:traderpass@localhost:54341/masday_workflow \
  cargo run -p masday-mcp
```

### Database Setup

The runtime requires PostgreSQL on port 54341 for persistent state.

```bash
# Start PostgreSQL + Redis
docker compose up -d

# Database: masday_workflow (user: trader, pass: traderpass, port: 54341)
# Tables are created by the Rust application on first run
```

## What this starts

- **MCP stdio server** (`masday-mcp`) with 20 tool domains
- **REST API server** (`masday-api`) on port 30101 with 243 routes
- **PostgreSQL-backed state** — 16 tables for workflow, task, memory, review, session, graph, etc.
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
