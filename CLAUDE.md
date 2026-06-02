# CLAUDE.md - masday-workflow-rust

## Project Overview

Unified AI coding agent platform built on Model Context Protocol (MCP).
Rust workspace with 6 crates: core types, PostgreSQL database layer, business services, Axum API server, MCP server (20 tool domains), and CLI installer.

## Architecture

```
User -> MCP Protocol (stdio) -> MCP Server -> Service Layer -> PostgreSQL
                                        \
User -> HTTP API (Axum) -----> API Server -> Service Layer -> PostgreSQL
```

### Workspace Crates (6)

| Crate | Binary | Description |
|-------|--------|-------------|
| `masday-core` | lib | Shared types (WorkflowState, TaskState, PlanState, etc.), error types (AppError) |
| `masday-db` | lib | PostgreSQL via deadpool-postgres + tokio-postgres, 15 repo modules |
| `masday-service` | lib | Business logic layer (10 services), state machine, auto-transition |
| `masday-api` | `masday-api` | Axum HTTP server, REST API (243 routes), WebSocket streaming |
| `masday-mcp` | `masday-mcp` | MCP server (20 tool domains), stdio transport, PostgreSQL persistence |
| `masday-cli` | `masday` | CLI installer — local mode (build+install) + remote mode (download MCP) |

### Request Lifecycle

```
  User Input
      |
      v
 +-------------+                +------------------+
 |   Client     | ------------> |   MCP Server      |
 | (Dashboard/  |   stdio       |  (20 domains)     |
 |  CLI/MCP)    |               +--------+----------+
 +-------------+                         |
                                        v
                            +----------------------------+
                            |    SERVICE LAYER           |
                            |                            |
                            |  WorkflowService (state    |
                            |    machine + transitions)  |
                            |  TaskService (auto-        |
                            |    transition to DONE)     |
                            |  MemoryService (4-layer)   |
                            |  PolicyService (audit)     |
                            |  ...10 services total      |
                            +-----------+----------------+
                                        |
                                        v
                            +----------------------------+
                            |    DATABASE LAYER          |
                            |  PostgreSQL (port 54341)    |
                            |  15 repos, deadpool pool   |
                            |  16 tables                 |
                            +----------------------------+
```

### Dependency Graph

```
masday-cli --> masday-core
masday-mcp  --> masday-service --> masday-db --> masday-core
masday-api  --> masday-service --> masday-db --> masday-core
```

## MCP Server — 20 Tool Domains

Binary: `masday-mcp` (stdio transport)

| Domain | Module | Description |
|--------|--------|-------------|
| workflow | `workflow.rs` | CRUD, execute, status transitions, task lifecycle |
| memory | `memory.rs` | Store, search, recall (4-layer: working/episodic/long-term/graph) |
| graph | `graph.rs` | Knowledge graph nodes & edges, Jaccard auto-link |
| context | `context.rs` | Context packs, fingerprints |
| session | `session.rs` | Session state management |
| policy | `policy.rs` | Workflow audit, completion validation, drift detection |
| review | `review.rs` | Review decisions (APPROVED/REWORK_REQUIRED/BLOCKED) |
| capability | `capability.rs` | Agent/skill registry, system readiness |
| reminder | `reminder.rs` | Stale/stuck workflow detection |
| local | `local.rs` | File-based `.masday/` state sync |
| filesystem | `filesystem.rs` | Read/write/list/delete/stat |
| git | `git.rs` | Git CLI operations |
| npm | `npm.rs` | pnpm CLI operations |
| docker | `docker.rs` | Docker CLI operations |
| cicd | `cicd.rs` | GitHub Actions via `gh` CLI |
| github | `github.rs` | GitHub operations via `gh` CLI |
| tests | `tests.rs` | Test runner via pnpm |
| project_rules | `project_rules.rs` | Refactor rules validation (14 checks) |
| use_masday | `use_masday.rs` | Universal entry point — parses intent, routes to tool |
| search | `search.rs` | Semantic search, BM25, code indexing |

## Database — 15 Repos, 16 Tables

Connection: `deadpool-postgres` pool via `DATABASE_URL` env var.

| Repo Module | Table(s) |
|-------------|----------|
| `workflow_repo` | Workflow |
| `task_repo` | Task |
| `plan_repo` | Plan |
| `memory_repo` | Memory |
| `episodic_memory_repo` | EpisodicMemory |
| `graph_repo` | GraphNode, GraphEdge |
| `review_repo` | ReviewDecision |
| `session_repo` | SessionState |
| `branch_repo` | ParallelBranch |
| `context_document_repo` | ContextDocument |
| `progress_log_repo` | TaskProgressLog |
| `retrieval_log_repo` | RetrievalLog |
| `token_usage_repo` | TokenUsage |
| `reminder_repo` | WorkflowReminder |
| `llm_provider_config_repo` | LlmProviderConfig |

## Workflow States

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

**Auto-transition:** When all tasks complete, workflow auto-transitions to DONE via `TaskService::auto_transition_if_all_done()`.

Status conventions (ALL UPPERCASE in PostgreSQL):
- Workflow: INIT, ANALYZE, PLAN, EXECUTE, VERIFY, FIX, DONE, FAILED, PAUSED
- Task: PENDING, RUNNING, DONE, FAILED
- Plan: ACTIVE, PENDING, READY, DONE
- Review: APPROVED, REWORK_REQUIRED, BLOCKED

## Build & Run

```bash
source ~/.cargo/env

# Build all crates
cargo build
cargo build --release          # Optimized (LTO thin, strip, opt-level s)

# Build specific binary
cargo build -p masday-api --release
cargo build -p masday-mcp --release
cargo build -p masday-cli --release

# Run tests
cargo test
cargo test -p masday-service

# Run API server
DATABASE_URL=postgresql://USER:PASS@localhost:54341/masday_workflow \
  cargo run -p masday-api

# Run MCP server (stdio)
DATABASE_URL=postgresql://USER:PASS@localhost:54341/masday_workflow \
  cargo run -p masday-mcp

# Build for release (cross-compile)
cargo build -p masday-cli --release --target x86_64-unknown-linux-gnu
cargo build -p masday-cli --release --target x86_64-pc-windows-gnu
```

### Infrastructure

- **PostgreSQL 16** on port 54341 (see `.env` for credentials, db: masday_workflow)
- **Redis 7** on port 63791
- Start: `docker compose up -d`

### Release Pipeline

```bash
# Normal push — build + test only
git push origin master

# Tag push — build binaries + create GitHub Release
MASDAY_RELEASE=1 git push origin v0.2.0

# Manual release
bash scripts/release.sh v0.2.0

# Dry run (no upload)
bash scripts/release.sh v0.2.0 --dry-run
```

Builds 4 binaries: `masday` (CLI) + `masday-mcp` (MCP server) for Linux x86_64 + Windows x86_64 (cross-compile via mingw-w64).

| Binary | Linux | Windows | Size |
|--------|-------|---------|------|
| masday (CLI) | `masday-linux-x86_64` | `masday-windows-x86_64.exe` | ~7.6MB |
| masday-mcp (MCP server) | `masday-mcp-linux-x86_64` | `masday-mcp-windows-x86_64.exe` | ~2.4MB |

## Conventions

- Rust 2021 edition
- `cargo fmt` and `cargo clippy -- -D warnings` must pass
- `thiserror` for library errors, `anyhow` for application errors
- `tokio` async runtime with `#[tokio::test]` for async tests
- `serde` derive for all types crossing API/MCP boundaries
- `deadpool-postgres` connection pool, raw SQL queries
- UUID for workflow/task IDs
- All new service methods must have unit tests
- Module naming: `snake_case` files match `snake_case` modules
- Service pattern: standalone functions in `masday-service/src/`, repos in `masday-db/src/repos/`

## Hooks System

Source of truth: `scripts/global-hooks/` — edit source files first, then run `bash scripts/install-hooks.sh`.

### Statusline

Output: `⚡ Masday | DB:✓ | API:✓ | MCP:✓ | 🟢 ▓▓▓▓░░░░░░ 35% | ▶ 1 | masday-workflow-rust(N)`

| Segment | Method | States |
|---------|--------|--------|
| DB | `isPortOpen(54341)` | ✓/✗ |
| API | HTTP GET `/api/health` (1s) | ✓ healthy / ⚠ port open but failing / ✗ down |
| MCP | `pgrep -f masday-mcp` | ✓ running / ⚠ binary only / ✗ not built |
| Context % | Post-compact bytes / 4 + 18K overhead | 🟢 <50% / 🟡 50-75% / 🔴 ≥75% |
| Workflow | GET `/api/workflows` filtered by project | ▶ N active / ⛔ N stuck / hidden if none |
| Project | `path.basename()` + dirty count | `masday-workflow-rust(N)` |

### Git Hooks

- **pre-commit**: `cargo fmt --check` + `cargo clippy` + `eslint`
- **pre-push**: Multi-stack build + test (Rust: `cargo build --release`, TS: `pnpm build && pnpm test`)
- **Tag release**: `MASDAY_RELEASE=1 git push origin v*` triggers `scripts/release.sh`

### Claude Hooks (`.claude/settings.json`)

- **SessionStart**: Infrastructure health check (PostgreSQL, Redis, API, MCP, git status)
- **PreCompact/PostCompact**: Context preservation across compaction
- **UserPromptSubmit**: Context usage warning (50%/75%/90% thresholds)
- **PreToolUse (Bash)**: Blocks destructive DB operations

## Testing

- Unit tests: `#[cfg(test)] mod tests` in each module
- Integration tests: `tests/` directory
- E2E tests: `tests/integration_e2e_workflow.rs`
- Run all: `cargo test --workspace`
