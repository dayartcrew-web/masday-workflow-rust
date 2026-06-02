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

**One-line install (Linux):**
```bash
curl -fsSL https://github.com/dayartcrew-web/masday-workflow-rust/releases/latest/download/install.sh | bash
```

**Manual download:** [CLI Linux](https://github.com/dayartcrew-web/masday-workflow-rust/releases/latest/download/masday-linux-x86_64) · [CLI Windows](https://github.com/dayartcrew-web/masday-workflow-rust/releases/latest/download/masday-windows-x86_64.exe) · [MCP Linux](https://github.com/dayartcrew-web/masday-workflow-rust/releases/latest/download/masday-mcp-linux-x86_64) · [MCP Windows](https://github.com/dayartcrew-web/masday-workflow-rust/releases/latest/download/masday-mcp-windows-x86_64.exe) · [All releases](https://github.com/dayartcrew-web/masday-workflow-rust/releases)

📖 **Full install guide:** [docs/install-guide.md](docs/install-guide.md)

```bash
masday --version          # Check version
masday install            # Install into current project (local mode)
masday uninstall          # Remove from project
```

---

## Quick Start

### Prerequisites

- **Rust** 1.85+ ([install](https://rustup.rs/))
- **PostgreSQL** 16 with pgvector
- **mingw-w64** (for Windows cross-compile, optional)

### Setup

```bash
# Clone and setup
git clone <repo-url>
cd masday-workflow-rust
bash scripts/setup.sh

# Start infrastructure (PostgreSQL + pgvector)
docker-compose up -d

# Configure environment
cp .env.example .env
# Edit .env with your DATABASE_URL

# Build Rust crates
cargo build --workspace

# Run MCP server (exposes tools via stdio)
DATABASE_URL=postgresql://USER:PASS@localhost:54341/masday_workflow \
  cargo run -p masday-mcp

# Run API server (REST endpoints)
DATABASE_URL=postgresql://USER:PASS@localhost:54341/masday_workflow \
  cargo run -p masday-api

# Build release binaries
cargo build --release --workspace
```

---

## Architecture

### MVC Layer Structure

```
┌──────────────┐     ┌──────────────┐     ┌─────────────┐
│  MCP Client  │────>│  Rust API    │────>│  PostgreSQL │
│  (Claude/etc)│ HTTP│  (Axum)      │ sqlx│             │
└──────────────┘     │              │     └─────────────┘
                     │  ┌────────┐  │
┌──────────────┐     │  │Service │  │     ┌─────────────┐
│  Dashboard   │────>│  │ Layer  │  │────>│ Redis Cache │
│  (Next.js)   │ HTTP│  └────────┘  │     └─────────────┘
└──────────────┘     │  ┌────────┐  │
                     │  │Repo    │  │     ┌─────────────┐
                     │  │ Layer  │  │────>│  Vector DB  │
                     │  └────────┘  │     │  (pgvector) │
                     └──────────────┘     └─────────────┘
```

### Rust Crates

| Crate | Description |
|-------|-------------|
| **masday-core** | Shared types, errors, constants |
| **masday-db** | Repository layer (sqlx, PostgreSQL, deadpool) |
| **masday-service** | Business logic (workflow, memory, policy, capability) |
| **masday-api** | HTTP API layer (Axum, REST endpoints) |
| **masday-mcp** | MCP server (stdio protocol, 89 tools) |
| **masday-cli** | Command-line interface |

### Memory Stack

```
  +----------------------------------------------------------+
  |                   WORKING MEMORY                         |
  |              In-process RAM, per session                 |
  +----------------------------------------------------------+
                           |
  +----------------------------------------------------------+
  |                  EPISODIC MEMORY                         |
  |            Last N messages per session                   |
  +----------------------------------------------------------+
                           |
  +----------------------------------------------------------+
  |                 LONG-TERM MEMORY                         |
  |   Scoring: similarity*0.6 + importance*0.2               |
  |            + recency*0.1 + usage*0.1                     |
  +----------------------------------------------------------+
                           |
  +----------------------------------------------------------+
  |                 KNOWLEDGE GRAPH                          |
  |             Nodes & edges, auto-linked                   |
  +----------------------------------------------------------+
```

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

## MCP Tools (89 tools across 16 namespaces)

The `masday-mcp` crate exposes 89 MCP tools via stdio. Each tool corresponds to an HTTP endpoint in `masday-api`.

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

## Commands Reference

### Rust Commands

```bash
# Build
cargo build --workspace              # Build all crates
cargo build --release               # Optimized release build
cargo build -p masday-mcp           # Build specific crate

# Run
cargo run -p masday-mcp             # Start MCP server
cargo run -p masday-api             # Start API server
cargo run -p masday-cli             # Run CLI

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
# Build release binaries (CLI + MCP server)
cargo build --release --workspace

# Cross-compile for Windows (CLI + MCP)
cargo build -p masday-cli --release --target x86_64-pc-windows-gnu --no-default-features
cargo build -p masday-mcp --release --target x86_64-pc-windows-gnu --no-default-features

# Create GitHub Release (builds 4 binaries: CLI+MCP for Linux+Windows)
bash scripts/release.sh v0.3.0
bash scripts/release.sh v0.3.0 --dry-run  # test without uploading
```

### Release Artifacts

| Binary | Linux | Windows | Size |
|--------|-------|---------|------|
| **masday** (CLI installer) | `masday-linux-x86_64` | `masday-windows-x86_64.exe` | ~7.6MB |
| **masday-mcp** (MCP server) | `masday-mcp-linux-x86_64` | `masday-mcp-windows-x86_64.exe` | ~2.4MB |

---

## Configuration

### Environment Variables

Create a `.env` file in the project root:

```env
# Database
DATABASE_URL="postgresql://USER:PASS@localhost:54341/masday_workflow"
POSTGRES_HOST="localhost"
POSTGRES_PORT="54341"
POSTGRES_USER="your-db-user"
POSTGRES_PASSWORD="your-secure-password"
POSTGRES_DB="masday_workflow"

# Redis (optional, for caching)
REDIS_URL="redis://localhost:63791"

# API
API_PORT="8080"
API_HOST="0.0.0.0"

# MCP
RUST_LOG="info"                    # debug, info, warn, error
RUST_BACKTRACE="1"                # Enable backtrace on panic

# Embeddings (semantic search)
EMBEDDING_PROVIDER="local"          # local (fastembed) | ollama | openai
EMBEDDING_MODEL="all-MiniLM-L6-v2"  # all-MiniLM-L6-v2 (384d) | bge-base-en-v1.5 (768d)
EMBEDDING_DIMENSIONS="384"           # must match model output
FASTEMBED_CACHE_DIR=".cache/fastembed"  # optional: model cache directory
```

### MCP Configuration

The `scripts/setup.sh` script generates MCP configuration files for different platforms:

- **Claude Code**: `.mcp.json`
- **Gemini CLI**: `.gemini/settings.json`
- **VS Code Copilot**: `.vscode/mcp.json`

All configurations point to the Rust MCP binary: `target/debug/masday-mcp` (or `target/release/masday-mcp` for release builds).

#### MCP Binary Distribution

The `masday-mcp` binary is distributed separately from the CLI in GitHub Releases:

```bash
# Download MCP server binary (Linux)
curl -fsSL -o ~/.masday/bin/masday-mcp \
  https://github.com/dayartcrew-web/masday-workflow-rust/releases/latest/download/masday-mcp-linux-x86_64
chmod +x ~/.masday/bin/masday-mcp

# Download MCP server binary (Windows PowerShell)
Invoke-WebRequest -Uri "https://github.com/dayartcrew-web/masday-workflow-rust/releases/latest/download/masday-mcp-windows-x86_64.exe" -OutFile "masday-mcp.exe"
```

For **stdio mode**, configure your MCP client to point to the binary:

```json
{
  "mcpServers": {
    "masday": {
      "type": "stdio",
      "command": "/path/to/masday-mcp",
      "env": {
        "MASDAY_API_URL": "http://localhost:30101",
        "MASDAY_API_KEY": "local-mode",
        "DATABASE_URL": "postgresql://USER:PASS@localhost:54341/masday_workflow"
      }
    }
  }
}
```

For **HTTP/SSE mode** (no binary needed on client), point directly to the API server:

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

### PostgreSQL + pgvector

The project uses PostgreSQL 16 with pgvector for semantic search.

```bash
# Start PostgreSQL with pgvector
docker-compose up -d

# Run migrations (if using migrations)
cargo run -p masday-db -- migrate

# Or use SQLx auto-migration (development)
# sqlx-cli will create tables on first run
```

### Schema

The database schema is defined in `masday-db/src/schema.rs` using sqlx compile-time checked queries. Key tables:

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
| **Database** | PostgreSQL 16 + pgvector (via deadpool-postgres) |
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

### Database Connection

```bash
# Check PostgreSQL is running
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
