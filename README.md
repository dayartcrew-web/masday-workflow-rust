# masday-workflow-rust

[![Rust](https://img.shields.io/badge/Rust-1.85-orange?logo=rust&logoColor=white)](https://www.rust-lang.org/)
[![PostgreSQL](https://img.shields.io/badge/PostgreSQL-16-4169E1?logo=postgresql&logoColor=white)](https://www.postgresql.org/)
[![MCP](https://img.shields.io/badge/MCP-1.29-green?logo=data:image/svg+xml;base64,PHN2ZyB4bWxucz0iaHR0cDovL3d3dy53My5vcmcvMjAwMC9zdmciIHdpZHRoPSIyNCIgaGVpZ2h0PSIyNCIgdmlld0JveD0iMCAwIDI0IDI0IiBmaWxsPSJub25lIiBzdHJva2U9IndoaXRlIiBzdHJva2Utd2lkdGg9IjIiPjxwYXRoIGQ9Ik0xMiAydjIwTTIgMTJoMjAiLz48L3N2Zz4=)](https://modelcontextprotocol.io/)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

**Unified AI coding agent platform built on Model Context Protocol (MCP) — Rust Implementation**

> **Migration Status:** Complete. Backend migrated from TypeScript to Rust.

This is the Rust implementation of masday-workflow, providing a robust, type-safe backend with the MCP protocol. The project combines a multi-agent workflow system with 4-layer memory (working, episodic, long-term, graph) and exposes MCP tools for AI agents.

---

## Install

**One-line install (Linux/macOS/Windows Git Bash):**
```bash
curl -fsSL https://github.com/dayartcrew-web/masday-workflow-rust/releases/latest/download/install.sh | bash
```

Auto-detects: **Linux** / **macOS** / **Windows** × **x86_64** / **aarch64**

**Manual download:** [Linux x86_64](https://github.com/dayartcrew-web/masday-workflow-rust/releases/latest/download/masday-linux-x86_64) · [Windows x86_64](https://github.com/dayartcrew-web/masday-workflow-rust/releases/latest/download/masday-windows-x86_64.exe) · [All releases](https://github.com/dayartcrew-web/masday-workflow-rust/releases)

📖 **Full install guide:** [docs/install-guide.md](docs/install-guide.md)

```bash
masday quickstart          # One-command setup (db + agents + MCP)
masday --version           # Check version
masday status              # Check health
```

---

## Quick Start

### Prerequisites

- **Rust** 1.85+ ([install](https://rustup.rs/))
- **mingw-w64** (for Windows cross-compile, optional)

### Setup

```bash
# Clone and setup
git clone <repo-url>
cd masday-workflow-rust
bash scripts/setup.sh

# Build Rust crates
cargo build --workspace

# Run MCP server (exposes tools via stdio, uses SQLite — no database setup needed)
cargo run -p masday-mcp

# Run API server (REST endpoints, requires PostgreSQL)
DATABASE_URL=postgresql://USER:PASS@localhost:54341/masday_workflow \
  cargo run -p masday-api

# Build release binaries
cargo build --release --workspace
```

> **Note:** The MCP stdio server (`masday-mcp`) runs in **SQLite-only mode** — it creates `~/.masday/data.db` automatically. No PostgreSQL or API server required for local use. PostgreSQL is only needed for the API server (`masday-api`) or remote deployments.

---

## Architecture

### Architecture — Local Mode (stdio, SQLite)

```
┌──────────────┐     ┌──────────────────────────────────┐
│  MCP Client  │────>│  masday-mcp (standalone binary)   │
│  (Claude/etc)│stdio│  ┌──────────┐  ┌──────────────┐  │
└──────────────┘     │  │ 20 Tool  │  │   SQLite     │  │
                     │  │ Domains  │──│  ~/.masday/  │  │
                     │  └──────────┘  │  data.db     │  │
                     └──────────────────────────────────┘
```

### Architecture — Remote Mode (HTTP/SSE → API → PostgreSQL)

```
┌──────────────┐     ┌──────────────────────────────┐     ┌─────────────┐
│  MCP Client  │────>│  API Server (masday-api)     │────>│  PostgreSQL │
│  (Claude/etc)│HTTP │  (Axum)                      │      │             │
└──────────────┘     │                              │     └─────────────┘
                     │  ┌────────┐  ┌───────────┐  │
┌──────────────┐     │  │Service │  │ 20 Tool   │  │     ┌─────────────┐
│  Dashboard   │────>│  │ Layer  │  │ Domains   │  │────>│ Redis Cache │
│  (Next.js)   │HTTP │  └────────┘  └───────────┘  │     └─────────────┘
└──────────────┘     └──────────────────────────────┘
```

**Remote mode env vars (API server only):**
```env
DATABASE_URL="postgresql://USER:PASS@localhost:54341/masday_workflow"
MASDAY_API_KEY="your-api-key"
```

### Rust Crates

| Crate | Description |
|-------|-------------|
| **masday-core** | Shared types, errors, constants |
| **masday-db** | Repository layer (sqlx, PostgreSQL, deadpool) |
| **masday-service** | Business logic (workflow, memory, policy, capability) |
| **masday-api** | HTTP API layer (Axum, REST endpoints) |
| **masday-mcp** | MCP server (stdio protocol, SQLite-backed, 90 tools) |
| **masday-cli** | Command-line interface |

### Memory Stack

Masday uses a **3-layer persistence** architecture:

#### Layer 1: File-based Memory (Claude Code native)

```
~/.claude/projects/<project>/memory/
├── MEMORY.md              ← Index (auto-loaded every session)
├── decision-name.md       ← Individual memory files
└── another-fact.md
```

- Each file = 1 fact with frontmatter (`name`, `description`, `metadata.type`)
- `MEMORY.md` = table of contents loaded into context at every session start
- Written by Claude via `Write` tool, not MCP
- Best for: user preferences, critical decisions, project constraints

#### Layer 2: SQLite/PostgreSQL Memory (MCP `memory_*` tools)

```
Local:  ~/.masday/data.db → table: memories
Remote: PostgreSQL        → table: memories (with pgvector)
```

- Store/search via `mcp__masday__memory_store`, `memory_search`, `memory_recall`
- 4-layer model: working → episodic → long-term → graph
- **Local mode (SQLite):** Feature hashing vectorizer (not neural embeddings)
  - `hash(text) → [f32; 768]` — non-cryptographic hash functions
  - Vectors stored as BLOB in SQLite
  - Cosine similarity computed in **Rust code** (brute-force, not SQL)
  - Suitable for hundreds of memories, not thousands
- **Remote mode (PostgreSQL):** pgvector extension for proper vector search
  - `ORDER BY embedding <=> $query_vector` — indexable with IVFFlat/HNSW
  - Scales to millions of vectors
- Scoring: `similarity×0.6 + importance×0.2 + recency×0.1 + usage×0.1`

#### Layer 3: Knowledge Graph (MCP `graph_*` tools)

```
Local:  ~/.masday/data.db → tables: graph_nodes, graph_edges
Remote: PostgreSQL        → tables: graph_nodes, graph_edges
```

- Concepts as nodes, relationships as edges
- Jaccard similarity auto-link (threshold 0.3)

#### 4-Layer Memory Model

```
  +----------------------------------------------------------+
  |                   WORKING MEMORY                         |
  |              In-process RAM, per session                 |
  +----------------------------------------------------------+
                           |
  +----------------------------------------------------------+
  |                  EPISODIC MEMORY                         |
  |            Last N messages per session                   |
  |         Persisted to EpisodicMemory table                |
  +----------------------------------------------------------+
                           |
  +----------------------------------------------------------+
  |                 LONG-TERM MEMORY                         |
  |   Scoring: similarity×0.6 + importance×0.2               |
  |            + recency×0.1 + usage×0.1                     |
  |   Memory table (SQLite BLOB or PostgreSQL pgvector)      |
  +----------------------------------------------------------+
                           |
  +----------------------------------------------------------+
  |                 KNOWLEDGE GRAPH                          |
  |             Nodes & edges, auto-linked                   |
  |        GraphNode + GraphEdge tables                      |
  +----------------------------------------------------------+
```

#### Embedding Implementation

| Mode | Vectorizer | Storage | Search | Scale |
|------|-----------|---------|--------|-------|
| Local (SQLite) | Feature hashing (768-dim) | BLOB column | Rust brute-force cosine | ~100s |
| Remote (PostgreSQL) | ONNX nomic-embed-text or OpenAI | pgvector column | `ORDER BY <=>` (IVFFlat/HNSW index) | ~millions |

### Workflow States

```
INIT --> ANALYZE --> PLAN --> EXECUTE --> VERIFY --> DONE
  |                    |    |      |          |
  |--> DONE            |    |      |--> FIX --|
  |--> FAILED          |    |--> PAUSED       |--> FIX --> EXECUTE
                       |--> FAILED    |
                                      |--> FAILED
                                         FIX --> DONE
                                         FIX --> FAILED
```

---

## MCP Tools (90 tools across 20 domains)

The `masday-mcp` crate exposes 90 MCP tools via stdio. Use `masday mcp` to start the server.

### Tool Namespaces

| Namespace | Tools | Description |
|-----------|-------|-------------|
| **workflow** | 23 | Workflow lifecycle, tasks, plans, parallel branches |
| **memory** | 11 | 4-layer memory (working, episodic, long-term, graph) |
| **semantic-search** | 3 | Context packs, code search, fingerprinting |
| **policy** | 6 | Validation, drift detection, workflow audit |
| **capability** | 11 | Agent/skill registry, system health, scaffolding |
| **filesystem** | 5 | File operations (read, write, list, delete) |
| **review** | 2 | Review submission, decision tracking |
| **session** | 3 | Session state management |
| **local** | 4 | Local file-based state (.masday/) |
| **git** | 3 | Git operations |
| **npm** | 2 | Package manager operations |
| **docker** | 3 | Docker operations |
| **cicd** | 3 | CI/CD operations |
| **github** | 3 | GitHub operations |
| **tests** | 1 | Test runner |
| **reminder** | 3 | Stale/stuck workflow detection |

---

## CLI Commands Reference

### Setup & Install

```bash
masday setup                # Interactive setup wizard (first-time)
masday quickstart           # One-command setup: db + migrate + install + ready
masday install              # Install masday into current project
masday install --force      # Force overwrite existing configs
masday install --standalone # Standalone mode — templates only, no build
masday install --remote URL # Connect to remote API server
masday install --skip-build # Skip cargo build, use existing binaries
masday uninstall            # Remove masday from current project
```

### Update

```bash
masday update               # Download latest binary from GitHub Releases
                            # Copies to ~/.masday/bin/, re-syncs config
```

### Services

```bash
masday status               # Check health of all services
masday serve                # Start API server + dashboard (port 30101)
masday serve --port 8080    # Start on custom port
masday mcp                  # Start MCP server (stdio — used by AI platforms)
```

### Database

```bash
masday db start             # Start PostgreSQL and Redis containers
masday db stop              # Stop all containers
masday db reset             # Delete data and recreate containers
```

### Local Embeddings

```bash
masday embed setup                    # Download ONNX Runtime + default model
masday embed setup --model bge-small-en-v1.5  # Download specific model
masday embed status                   # Show embedding setup status
masday embed remove                   # Remove all embedding artifacts
masday embed remove --models-only     # Remove models, keep ONNX Runtime
```

**Supported models:**

| Model | Dimensions | Description |
|-------|-----------|-------------|
| `all-MiniLM-L6-v2` (default) | 384 | Fast, good quality |
| `bge-small-en-v1.5` | 384 | BGE small variant |
| `bge-base-en-v1.5` | 768 | BGE base, higher quality |

### Developer Commands (from source)

```bash
# Build
cargo build --workspace              # Build all crates
cargo build --release               # Optimized release build
cargo build -p masday-mcp           # Build specific crate

# Run
cargo run -p masday-mcp             # Start MCP server
cargo run -p masday-api             # Start API server

# Test
cargo test --workspace              # Run all tests
cargo test -p masday-service        # Test specific crate
cargo test -- --nocapture          # Show test output

# Lint
cargo clippy --workspace -- -D warnings  # Lint with warnings as errors
cargo fmt --all                     # Format code
cargo fmt --all -- --check         # Check formatting

# Clean
cargo clean                         # Remove build artifacts
```

### Release Commands

```bash
# Build and release from project root (Linux + Windows)
bash scripts/release.sh v0.3.39

# Build Linux only
bash scripts/release.sh v0.3.39 --linux-only

# Test without uploading
bash scripts/release.sh v0.3.39 --dry-run
```

### Release Artifacts

| Binary | Linux | Windows | Size |
|--------|-------|---------|------|
| **masday** (CLI) | `masday-linux-x86_64` (39MB) | `masday-windows-x86_64.exe` (15MB) | CLI + MCP server wrapper |
| **masday-mcp** (standalone) | `masday-mcp-linux-x86_64` (3.3MB) | `masday-mcp-windows-x86_64.exe` (2.9MB) | Lightweight MCP server |

> **Note:** Windows binary is built without local ONNX embeddings. Use remote embedding provider (Ollama/OpenAI) on Windows.

---

## Configuration

All config lives under `~/.masday/`:

```
~/.masday/
├── config.toml       ← Configuration (auto-created by quickstart)
├── bin/
│   └── masday       ← CLI binary
└── data.db          ← SQLite database (local mode, auto-created)
```

### Quickstart Wizard

Run `masday quickstart` for an interactive setup wizard:

- **Local mode** — Docker PostgreSQL + migrations + API server
- **Remote mode** — Connect to existing API server (URL + API key + optional DB URL)
- **Standalone mode** — Agents & skills only, no DB/API

The wizard automatically:
- Detects installed platforms (Claude Code, Gemini, VS Code, OpenCode)
- Registers MCP servers for all selected platforms
- Syncs agents, skills, and hooks
- Saves config to `~/.masday/config.toml`

### Config File (`~/.masday/config.toml`)

```toml
mode = "local"                          # local | remote | standalone
api_url = "http://localhost:30101"
api_key = "***"
database_url = "postgresql://USER:PASS@localhost:54341/masday_workflow"
embedding_provider = "local"             # local | ollama | openai
embedding_model = "all-MiniLM-L6-v2"
embedding_dimensions = 384
port = 30101
platforms = ["claude-code"]
```

### Environment Variables

**Local mode (default)** — the MCP stdio server requires **no configuration**. It uses SQLite at `~/.masday/data.db` automatically.

**Remote mode** — only the API server needs env vars. MCP clients connect directly via HTTP/SSE:

```env
# ── API Server (masday-api) ──────────────────────────────
DATABASE_URL="postgresql://USER:PASS@localhost:54341/masday_workflow"
MASDAY_API_KEY="your-api-key"           # Auth key for MCP clients

# ── Optional (both modes) ────────────────────────────────
RUST_LOG="info"                         # debug, info, warn, error
RUST_BACKTRACE="1"                      # Enable backtrace on panic
```

> **Note:** The MCP binary in local mode needs zero env vars. For remote deployments, skip the binary entirely and point MCP clients directly at the API server via HTTP/SSE.

### MCP Stdio Binary (Zero Config)

The `masday-mcp` binary runs standalone:
- **Database:** SQLite at `~/.masday/data.db` (auto-created on first run)
- **No PostgreSQL needed** for local development
- **No API server needed** — the binary contains all 20 tool domains directly

### MCP Configuration

The `scripts/setup.sh` script generates MCP configuration files for different platforms:

- **Claude Code**: `.mcp.json`
- **Gemini CLI**: `.gemini/settings.json`
- **VS Code Copilot**: `.vscode/mcp.json`

All configurations point to the Rust MCP binary: `target/debug/masday-mcp` (or `target/release/masday-mcp` for release builds).

#### MCP Binary Distribution

The `masday` binary wraps the MCP server — run `masday mcp` to start. A separate lightweight `masday-mcp` binary is also available.
- **Database:** SQLite at `~/.masday/data.db` (auto-created on first run)
- **No PostgreSQL needed** for local development
- **No API server needed** — the binary contains all 20 tool domains directly

Download from [GitHub Releases](https://github.com/dayartcrew-web/masday-workflow-rust/releases):

```bash
# Linux — full CLI (includes MCP server)
curl -fsSL -o masday \
  https://github.com/dayartcrew-web/masday-workflow-rust/releases/latest/download/masday-linux-x86_64
chmod +x masday

# Linux — standalone MCP server only
curl -fsSL -o masday-mcp \
  https://github.com/dayartcrew-web/masday-workflow-rust/releases/latest/download/masday-mcp-linux-x86_64
chmod +x masday-mcp

# Windows PowerShell
Invoke-WebRequest -Uri "https://github.com/dayartcrew-web/masday-workflow-rust/releases/latest/download/masday-windows-x86_64.exe" -OutFile "masday.exe"
```

For **stdio mode** (recommended — SQLite, no external services needed):

```json
{
  "mcpServers": {
    "masday": {
      "type": "stdio",
      "command": "/path/to/masday-mcp"
    }
  }
}
```

No environment variables needed — the binary creates `~/.masday/data.db` on first run.

For **remote deployments** (no MCP binary needed — point directly to API server):

```json
{
  "mcpServers": {
    "masday": {
      "url": "http://localhost:30101/mcp"
    }
  }
}
```

---

## Database

### Local Mode — SQLite (default for stdio)

The MCP stdio binary (`masday-mcp`) uses **SQLite** for persistence. No setup required.

- **Location:** `~/.masday/data.db`
- **Auto-created** on first run with all required tables
- **Zero config** — works out of the box

```bash
# Check your database
sqlite3 ~/.masday/data.db ".tables"
```

### Remote Mode — PostgreSQL + pgvector (for API server)

The API server (`masday-api`) uses PostgreSQL 16 with pgvector for multi-user deployments.

```bash
# Start PostgreSQL with pgvector
docker-compose up -d

# Run migrations
cargo run -p masday-db -- migrate
```

### Schema (16 tables, shared between SQLite and PostgreSQL)

- `workflows` — Workflow instances
- `tasks` — Task instances
- `plans` — Workflow plans
- `memories` — Long-term memories
- `episodic_memories` — Episodic memories
- `graph_nodes` / `graph_edges` — Knowledge graph
- `review_decisions` — Review decisions
- `session_states` — Session state
- `workflow_reminders` — Stale/stuck reminders
- `parallel_branches` — Parallel execution branches

---

## Tech Stack

| Layer | Technology |
|-------|-----------|
| **Language** | Rust (2021 edition) |
| **Runtime** | Tokio async runtime |
| **Database** | SQLite (stdio, local mode) / PostgreSQL 16 + pgvector (API, remote mode) |
| **API** | Axum (REST HTTP) |
| **Protocol** | Model Context Protocol (MCP) over stdio |
| **Cache** | Redis (optional) |
| **Validation** | Serde + serde_json |
| **Logging** | tracing |
| **Testing** | built-in `cargo test` |
| **CLI** | clap 4.5 |
| **Error Handling** | thiserror (lib) + anyhow (app) |

---

## Development Workflow

### Adding New MCP Tools

1. **Define types** in `masday-core/src/types.rs`
2. **Add repository methods** in `masday-db/src/repos/`
3. **Add service logic** in `masday-service/src/`
4. **Add HTTP endpoint** in `masday-api/src/routes/`
5. **Add MCP tool handler** in `masday-mcp/src/tools/`
6. **Register tool** in `masday-mcp/src/main.rs`
7. **Add tests** in each crate's `tests/` module

### Running Tests

```bash
# Unit tests (all crates)
cargo test --workspace

# Integration tests
cargo test --workspace --test '*_integration'

# With output
cargo test --workspace -- --nocapture

# Specific test
cargo test -p masday-service test_workflow_create
```

### Linting and Formatting

```bash
# Format all code
cargo fmt --all

# Check formatting (CI)
cargo fmt --all -- --check

# Lint with clippy
cargo clippy --workspace -- -D warnings

# Fix clippy warnings
cargo clippy --workspace --fix
```

---

## Platform Support

| Platform | Agents | Skills | MCP Config | Location |
|----------|--------|--------|------------|----------|
| **Claude Code** | `.claude/agents/*.md` | `.claude/skills/*/SKILL.md` | `.mcp.json` | Project root |
| **Gemini CLI** | `.gemini/agents/` | `.gemini/skills/` | `.gemini/settings.json` | `~/.gemini/` |
| **VS Code Copilot** | `.github/agents/` | N/A | `.vscode/mcp.json` | Project root |
| **OpenCode** | `.opencode/agent/` | `.opencode/skills/` | Custom | `~/.config/opencode/` |

Run `bash scripts/setup.sh` to install to all platforms.

---

## Migration from TypeScript

This project was migrated from a TypeScript monorepo to Rust. The migration is **complete**:

### Removed (TypeScript)

- ❌ `packages/*` (13 TypeScript packages)
- ❌ `apps/agent-runner` (TypeScript MCP server)
- ❌ `apps/api` (Express API)
- ❌ `apps/desktop` (Electron desktop app)
- ❌ `pnpm-workspace.yaml` (multi-package workspace)
- ❌ `turbo.json` (Turborepo build)
- ❌ `vitest.config.ts` (Vitest tests)

### Replaced by Rust

- ✅ `masday-core` — replaces `packages/core`, `packages/shared-utils`
- ✅ `masday-db` — replaces `packages/db`, `packages/store`
- ✅ `masday-service` — replaces `packages/workflow-engine`, `packages/memory`, `packages/policy`, etc.
- ✅ `masday-api` — replaces `apps/api` (Express)
- ✅ `masday-mcp` — replaces `apps/agent-runner` (MCP server)
- ✅ `masday-cli` — replaces `packages/cli`

### Kept (TypeScript)

- 🟡 `apps/dashboard` — Next.js frontend (still actively used)

### Migration Benefits

| Benefit | TypeScript → Rust |
|---------|-------------------|
| **Type Safety** | Runtime type errors → Compile-time guarantees |
| **Performance** | Single-threaded event loop → Multi-core Tokio |
| **Database** | Drizzle runtime checks → sqlx compile-time checks |
| **Memory** | JSON fallback → Proper API error handling |
| **Connections** | Stale connections → deadpool with health checks |

---

## Contributing

1. Fork the repository
2. Create a feature branch (`git checkout -b feat/my-feature`)
3. Follow Rust conventions:
   - `cargo fmt` must pass
   - `cargo clippy` must pass with no warnings
   - Write tests (`cargo test`)
   - Keep functions under 50 lines
   - Use `Result<T, E>` for error handling
4. Submit a pull request

### Conventions

- **Naming**: `snake_case` for functions/modules, `PascalCase` for types
- **Error Handling**: `thiserror` for library errors, `anyhow` for app errors
- **Async**: Use `tokio` runtime, `#[tokio::test]` for async tests
- **Database**: sqlx compile-time checked queries only
- **Serialization**: `serde` derive for all types crossing boundaries

---

## Troubleshooting

### Build Errors

```bash
# Clear build cache
cargo clean

# Update dependencies
cargo update

# Check Rust version
rustc --version  # Should be 1.85+
```

### Database Connection (API Server Only)

```bash
# Check PostgreSQL is running (API server mode only)
docker-compose ps

# Check DATABASE_URL in .env
echo $DATABASE_URL

# Test connection
psql $DATABASE_URL
```

### MCP Server Not Starting

```bash
# Check binary exists
ls -la target/debug/masday-mcp

# Build if missing
cargo build -p masday-mcp

# Check logs
RUST_LOG=debug cargo run -p masday-mcp

# Verify SQLite database
sqlite3 ~/.masday/data.db ".tables"
```

---

## License

[MIT](https://opensource.org/licenses/MIT)

---

## Credits

Built on top of outstanding open source software:

### Core

- **Rust** — Systems programming language
- **Tokio** — Async runtime
- **Axum** — Web framework
- **sqlx** — Database toolkit
- **Serde** — Serialization framework

### Database

- **PostgreSQL** — Advanced database
- **pgvector** — Vector similarity search
- **deadpool** — Connection pool

### Protocol

- **Model Context Protocol** — Agent communication standard

### Frontend

- **Next.js** — React framework
- **Zustand** — State management
- **Tailwind CSS** — Styling
