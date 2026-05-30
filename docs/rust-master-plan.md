# Rust Masday Workflow — Master Plan

> Convert the TypeScript MCP monorepo to a Rust MVC stack with API-based storage.

**Branch:** `rust-masday-workflow`
**Date:** 2026-05-31
**Reference:** `docs/prd-best-arc-postgres.md`

---

## 1. Why Rust?

| Problem (Current TS) | Rust Solution |
|---|---|
| DualWriteStore writes to PostgreSQL directly from MCP process | Rust API server is the **single authority** — MCP clients call HTTP endpoints |
| Stale connections, pool exhaustion, MaxListenersExceededWarning | `sqlx` with compile-time checked queries, deadpool with health checks |
| Runtime type errors in tool handlers | Compile-time type safety, `serde` validation |
| Single-threaded event loop bottleneck | Tokio async runtime, true multi-core |
| 13 packages, blurry boundaries | Clean MVC layers with enforced module visibility |
| JSON fallback when DB down | API returns proper HTTP errors — no silent degradation |

---

## 2. Architecture Overview

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

**Key change:** MCP server no longer touches PostgreSQL directly. It calls the Rust REST API. The API server owns all data access.

---

## 3. MVC Layer Structure

```
masday-rust/
├── Cargo.toml                    # workspace root
├── .env                          # DATABASE_URL, API_PORT, etc.
│
├── masday-core/                  # Shared types, errors, constants
│   ├── src/
│   │   ├── lib.rs
│   │   ├── types.rs              # WorkflowState, TaskState, etc.
│   │   ├── error.rs              # AppError enum (NotFound, Validation, Db, Auth)
│   │   └── constants.rs          # Status strings, limits
│   └── Cargo.toml
│
├── masday-db/                    # Repository layer (data access only)
│   ├── src/
│   │   ├── lib.rs
│   │   ├── schema.rs             # sqlx::FromRow models + query! macros
│   │   ├── pool.rs               # deadpool-postgres with health checks
│   │   ├── repos/
│   │   │   ├── mod.rs
│   │   │   ├── workflow_repo.rs   # CRUD workflows
│   │   │   ├── task_repo.rs       # CRUD tasks
│   │   │   ├── plan_repo.rs       # CRUD plans
│   │   │   ├── memory_repo.rs     # CRUD memories + search
│   │   │   ├── review_repo.rs     # CRUD review decisions
│   │   │   ├── session_repo.rs    # CRUD session state
│   │   │   ├── branch_repo.rs     # CRUD parallel branches
│   │   │   ├── reminder_repo.rs   # CRUD reminders
│   │   │   └── graph_repo.rs      # CRUD graph nodes/edges
│   │   └── migrations/            # SQL migrations (sqlx-cli)
│   └── Cargo.toml
│
├── masday-service/               # Business logic layer
│   ├── src/
│   │   ├── lib.rs
│   │   ├── workflow_service.rs    # State machine, DAG, transitions
│   │   ├── task_service.rs        # Task lifecycle, dependencies
│   │   ├── plan_service.rs        # Plan creation, validation
│   │   ├── memory_service.rs      # 4-layer memory, scoring, BM25
│   │   ├── review_service.rs      # Review pipeline, decisions
│   │   ├── policy_service.rs      # Validation, drift detection
│   │   ├── capability_service.rs  # Agent/skill registry
│   │   ├── context_service.rs     # Context packs, fingerprinting
│   │   └── reminder_service.rs    # Stale/stuck detection
│   └── Cargo.toml
│
├── masday-api/                   # Controller layer (Axum HTTP)
│   ├── src/
│   │   ├── lib.rs
│   │   ├── main.rs                # Server bootstrap
│   │   ├── routes/
│   │   │   ├── mod.rs
│   │   │   ├── workflow.rs        # /api/workflows/*
│   │   │   ├── task.rs            # /api/workflows/:id/tasks/*
│   │   │   ├── plan.rs            # /api/workflows/:id/plan
│   │   │   ├── memory.rs          # /api/memories/*
│   │   │   ├── review.rs          # /api/reviews/*
│   │   │   ├── session.rs         # /api/sessions/*
│   │   │   ├── policy.rs          # /api/policy/*
│   │   │   ├── capability.rs      # /api/capabilities/*
│   │   │   ├── context.rs         # /api/context/*
│   │   │   ├── reminder.rs        # /api/reminders/*
│   │   │   ├── graph.rs           # /api/graph/*
│   │   │   └── health.rs          # /api/health
│   │   ├── middleware/
│   │   │   ├── mod.rs
│   │   │   ├── auth.rs            # API key validation
│   │   │   ├── logging.rs         # Request/response logging
│   │   │   └── error_handler.rs   # AppError → HTTP response
│   │   └── extractors/
│   │       ├── mod.rs
│   │       └── pagination.rs      # Query param extraction
│   └── Cargo.toml
│
├── masday-mcp/                   # MCP stdio server (thin HTTP client)
│   ├── src/
│   │   ├── lib.rs
│   │   ├── main.rs                # Stdio transport
│   │   └── tools/
│   │       ├── mod.rs
│   │       ├── workflow.rs        # Calls /api/workflows/*
│   │       ├── memory.rs          # Calls /api/memories/*
│   │       ├── policy.rs          # Calls /api/policy/*
│   │       ├── capability.rs      # Calls /api/capabilities/*
│   │       ├── review.rs          # Calls /api/reviews/*
│   │       └── ...                # One file per namespace
│   └── Cargo.toml
│
└── masday-cli/                   # CLI tool
    ├── src/
    │   ├── lib.rs
    │   └── main.rs                # setup, db:migrate, etc.
    └── Cargo.toml
```

---

## 4. Technology Stack

| Layer | Rust Crate | Purpose |
|-------|-----------|---------|
| **HTTP Framework** | `axum` 0.8 | Routing, middleware, extractors |
| **Async Runtime** | `tokio` 1.x | Multi-threaded scheduler |
| **Database** | `sqlx` 0.8 + `deadpool-postgres` | Compile-time checked queries, connection pool |
| **Serialization** | `serde` + `serde_json` | JSON request/response |
| **Validation** | `validator` + `garde` | Input validation (replaces Zod) |
| **Logging** | `tracing` + `tracing-subscriber` | Structured logging (replaces Pino) |
| **UUID** | `uuid` 1.x | v4 generation |
| **Auth** | `jsonwebtoken` | API key / JWT |
| **MCP Protocol** | Custom over `tokio` stdin/stdout | JSON-RPC 2.0 via stdio |
| **HTTP Client** | `reqwest` | MCP server calls API |
| **Vector Search** | `sqlx` + pgvector extension | Semantic search |
| **Cache** | `redis` (optional) | Session caching |
| **Error Handling** | `thiserror` + `anyhow` | Typed error chains |
| **Testing** | `sqlx::test` + `tokio::test` | Integration + unit |
| **Migrations** | `sqlx-cli` | Database schema management |

---

## 5. Database Schema (sqlx)

The 16 existing Drizzle tables map 1:1 to sqlx models. Key difference: **API owns all queries**, not MCP.

```rust
// masday-db/src/schema.rs
use sqlx::FromRow;

#[derive(Debug, FromRow, serde::Serialize, serde::Deserialize)]
pub struct Workflow {
    pub id: String,           // UUID v4
    pub name: String,
    pub description: Option<String>,
    pub status: WorkflowStatus,
    pub metadata: Option<serde_json::Value>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, sqlx::Type, serde::Serialize, serde::Deserialize)]
#[sqlx(type_name = "text", rename_all = "UPPERCASE")]
pub enum WorkflowStatus {
    Init, Analyze, Plan, Execute, Verify, Fix, Done, Failed, Paused,
}
```

---

## 6. API Endpoints (89 MCP tools → REST)

The MCP server becomes a **thin HTTP client**. Every `mcp__masday__*` tool maps to an API call:

### Workflow (23 tools → endpoints)

| MCP Tool | HTTP Method | Endpoint |
|----------|-------------|----------|
| `workflow_create` | POST | `/api/workflows` |
| `workflow_execute` | POST | `/api/workflows/:id/execute` |
| `workflow_getStatus` | GET | `/api/workflows/:id/status` |
| `workflow_get` | GET | `/api/workflows/:id` |
| `workflow_list` | GET | `/api/workflows` |
| `workflow_addTask` | POST | `/api/workflows/:id/tasks` |
| `workflow_startTask` | POST | `/api/workflows/:id/tasks/:taskId/start` |
| `workflow_completeTask` | POST | `/api/workflows/:id/tasks/:taskId/complete` |
| `workflow_saveProgress` | POST | `/api/workflows/:id/tasks/:taskId/progress` |
| `workflow_listTasks` | GET | `/api/workflows/:id/tasks` |
| `workflow_getCurrentTask` | GET | `/api/workflows/:id/tasks/current` |
| `workflow_getPlan` | GET | `/api/workflows/:id/plan` |
| `workflow_getActive` | GET | `/api/workflows/active` |
| `workflow_createPlan` | POST | `/api/workflows/:id/plan` |
| `workflow_createParallelBranches` | POST | `/api/workflows/:id/branches` |
| `workflow_completeParallelBranch` | POST | `/api/workflows/:id/branches/:key/complete` |
| `workflow_listParallelBranches` | GET | `/api/workflows/:id/branches` |
| `workflow_delete` | DELETE | `/api/workflows/:id` |
| `workflow_set_execution_mode` | PATCH | `/api/workflows/:id/mode` |
| `workflow_mark_synthesis_ready` | PATCH | `/api/workflows/:id/synthesis` |
| `workflow_mark_verification_ready` | PATCH | `/api/workflows/:id/verification` |
| `workflow_resume_suggestion` | GET | `/api/workflows/:id/resume` |
| `workflow_ping` | GET | `/api/health` |

### Memory (11 tools → endpoints)

| MCP Tool | HTTP Method | Endpoint |
|----------|-------------|----------|
| `memory_store` | POST | `/api/memories` |
| `memory_store_research` | POST | `/api/memories/research` |
| `memory_recall_recent` | GET | `/api/memories/recent` |
| `memory_recall_documents` | GET | `/api/memories/documents` |
| `memory_recall_document_by_type` | GET | `/api/memories/documents/:type` |
| `memory_recall_by_task` | GET | `/api/memories/task/:taskId` |
| `memory_update` | PATCH | `/api/memories/:id` |
| `memory_delete` | DELETE | `/api/memories/:id` |
| `memory_delete_by_workflow` | DELETE | `/api/memories/workflow/:workflowId` |
| `memory_search` | POST | `/api/memories/search` |
| `memory_stats` | GET | `/api/memories/stats` |

### Policy (6), Review (2), Session (3), Capability (11), Context (4), Reminder (3), Graph (2), Health (1)

Same pattern — each MCP tool becomes one HTTP endpoint.

**Total: ~70 API endpoints** (filesystem, git, npm, docker, cicd, github, tests stay local to MCP — not proxied via API).

---

## 7. Anti-Stale Connection Strategy (Rust)

Following the PRD reference (`prd-best-arc-postgres.md`):

```rust
// masday-db/src/pool.rs
use deadpool_postgres::{Config, Pool, Runtime};
use tokio_postgres::NoTls;

pub fn create_pool(database_url: &str) -> Pool {
    let mut cfg = Config::new();
    cfg.url = Some(database_url.to_string());
    cfg.max_size = 20;                    // max connections
    cfg.idle_timeout = Some(30_000);      // 30s idle → close (anti-stale)
    cfg.connection_timeout = Some(2_000); // 2s connect timeout
    cfg.wait_timeout = Some(5_000);       // 5s wait for available conn

    // TCP keepalive — kernel-level ping every 10s
    let mgr = deadpool_postgres::Manager::new(cfg, NoTls);
    Pool::builder(mgr, Runtime::Tokio1)
        .max_size(20)
        .build()
        .expect("Failed to create pool")
}
```

**Why this is better than current TS approach:**
- `deadpool` proactively removes idle connections (no stale)
- `sqlx` compile-time checks prevent broken queries from deploying
- No `MaxListenersExceededWarning` — Rust has no event emitter limits
- No `process.setMaxListeners(20)` hacks needed
- Connection health checks happen at pool level, not application level

---

## 8. Workflow State Machine (Service Layer)

Business logic moves from `packages/workflow-engine` to `masday-service/src/workflow_service.rs`:

```rust
pub enum WorkflowState {
    Init, Analyze, Plan, Execute, Verify, Fix, Done, Failed, Paused,
}

impl WorkflowState {
    pub fn can_transition_to(&self, target: &Self) -> bool {
        match (self, target) {
            (Init, Analyze | Done | Failed) => true,
            (Analyze, Plan | Done | Failed) => true,
            (Plan, Execute | Paused | Failed) => true,
            (Execute, Verify | Fix | Paused | Failed) => true,
            (Verify, Done | Fix) => true,
            (Fix, Done | Execute | Failed) => true,
            (Paused, Execute | Failed) => true,
            _ => false,
        }
    }
}
```

---

## 9. MCP Server (Thin HTTP Client)

The MCP server no longer imports `@mcp-rebuild/db`. It calls the API:

```rust
// masday-mcp/src/tools/workflow.rs
pub async fn workflow_create(args: Value) -> Result<Value> {
    let client = reqwest::Client::new();
    let resp = client.post("http://localhost:3001/api/workflows")
        .header("Authorization", format!("Bearer {}", API_KEY))
        .json(&args)
        .send()
        .await?;
    let result: Value = resp.json().await?;
    Ok(result)
}
```

**Benefits:**
- MCP server can crash/restart without losing data
- API server is the single source of truth
- Multiple MCP clients can connect simultaneously
- Horizontal scaling: run multiple API instances behind a load balancer

---

## 10. Migration Phases

### Phase 1: Foundation (Week 1-2)
- [ ] Initialize Rust workspace with 6 crates
- [ ] Set up `masday-db` with sqlx, deadpool, migrations
- [ ] Implement all 16 table schemas as `FromRow` structs
- [ ] Connection pool with anti-stale config
- [ ] Basic CRUD repos for all tables

### Phase 2: API Server (Week 3-4)
- [ ] `masday-api` with Axum, all routes, middleware
- [ ] `masday-service` business logic (workflow state machine, task lifecycle)
- [ ] Error handling chain (AppError → HTTP responses)
- [ ] Auth middleware (API key)
- [ ] Request/response logging with tracing

### Phase 3: MCP Client (Week 5-6)
- [ ] `masday-mcp` stdio transport (JSON-RPC 2.0)
- [ ] All 89 tool handlers → HTTP calls to API
- [ ] Local-only tools (filesystem, git, npm, docker) stay as direct `tokio::process::Command`
- [ ] Integration tests against running API server

### Phase 4: Memory and Intelligence (Week 7-8)
- [ ] 4-layer memory (working, episodic, long-term, graph)
- [ ] Semantic search with pgvector + sqlx
- [ ] Context packs and fingerprinting
- [ ] BM25 text search

### Phase 5: Dashboard and Polish (Week 9-10)
- [ ] Update Next.js dashboard to call Rust API (replace `/api/*` proxy)
- [ ] CI/CD pipeline (cargo test, clippy, rustfmt)
- [ ] Performance benchmarks vs TypeScript version
- [ ] Documentation and migration guide

---

## 11. Skills, Agents, Hooks — Claude Code Ecosystem

These components are **not application code** — they are Claude Code infrastructure (markdown instructions, JS hooks). They stay as-is but their **tool call targets change** from direct MCP to HTTP-backed MCP.

### 11.1 Skills (`.claude/skills/` — 50+ files)

Skills are markdown files that tell Claude Code *how* to do things. They reference MCP tool names like `mcp__masday__workflow_create`. These tool names **do not change** — only the MCP server implementation behind them changes.

| Skill Category | Count | What Changes |
|---------------|-------|-------------|
| `masday-workflow-*` | 13 | Tool calls now go through HTTP, but skill content unchanged |
| `masday-create-*` | 4 | Unchanged — reads `.claude/` directory locally |
| `masday-research` / `masday-web-research` | 2 | Unchanged — uses WebSearch, not MCP |
| `masday-parallel-*` | 2 | Tool calls go through HTTP |
| `masday-tdd` | 1 | Unchanged — triggers step guard hooks |
| `masday-e2e` | 1 | Unchanged — uses Playwright |
| `masday-autopilot` | 1 | Tool calls go through HTTP |
| Other `masday-*` | 12 | Mix — shell tools stay local, data tools go HTTP |

**Skill migration strategy:** None needed. Skills call `mcp__masday__*` tool names. The Rust MCP server registers the same tool names. Skills don't know or care that the backing implementation changed from TypeScript DualWriteStore to Rust HTTP calls.

### 11.2 Agents (`.claude/agents/` — 10+ files)

Agents are markdown role descriptions with `## Step Checkpoint Protocol` sections. Same story — they reference tool names, not implementation.

| Agent | Role | Change Required |
|-------|------|----------------|
| `masday-orchestrator` | Full lifecycle coordinator | None — calls `mcp__masday__workflow_*` tools |
| `masday-executor` | Code implementation | None — uses Bash, Read, Write, Edit |
| `masday-reviewer` | Quality gate reviews | None — calls `mcp__masday__review_submit` |
| `masday-verifier` | Final validation | None — calls `mcp__masday__policy_validate_completion` |
| `masday-planner` | Task decomposition | None — calls `mcp__masday__workflow_createPlan` |
| `masday-debugger` | Root cause investigation | None — uses Bash, Grep, Read |
| `masday-frontend` | UI implementation | None — uses Read, Write, Edit |
| `masday-qa` | Testing | None — calls `mcp__masday__tests_run` |
| `masday-researcher` | External research | None — uses WebSearch |
| `masday-tdd-guide` | TDD with pipeline | None — calls `mcp__masday__review_submit` |

### 11.3 Step Enforcement Hooks (`.claude/settings.json`)

Two JS hooks enforce skill/agent step ordering. These **stay as JavaScript** because they run inside the Claude Code Node.js runtime, not in our application.

| Hook | File | Purpose | Change |
|------|------|---------|--------|
| `masday-skill-checkpoint.js` | PreToolUse | Tracks MCP tool call sequences, blocks `workflow_execute` without steps 1-6 | **None** — hooks intercept tool calls at Claude Code level, before they reach MCP |
| `skill-step-guard.cjs` | PreToolUse | Validates TDD RED→GREEN→REFACTOR, workflow GATE transitions | **None** — same mechanism |

Hooks don't need updating because:
1. They run in Claude Code's process, not in our MCP server
2. They intercept tool calls by name (e.g., `mcp__masday__workflow_execute`) — names don't change
3. They write state to `os.tmpdir()` — filesystem access is independent of Rust/TS

### 11.4 MCP Server Config (`.claude.json`, `.mcp.json`, etc.)

Currently:
```json
{
  "mcpServers": {
    "masday": {
      "command": "node",
      "args": ["apps/agent-runner/dist/runtime/mcp.js"],
      "cwd": "/path/to/project",
      "env": { "DATABASE_URL": "postgresql://..." }
    }
  }
}
```

After Rust migration:
```json
{
  "mcpServers": {
    "masday": {
      "command": "masday-mcp",
      "args": [],
      "env": {
        "MASDAY_API_URL": "http://localhost:3001",
        "MASDAY_API_KEY": "PLACEHOLDER-change-in-prod"
      }
    }
  }
}
```

Key differences:
- No `cwd` needed — binary is self-contained
- No `DATABASE_URL` in MCP config — API server handles DB
- `MASDAY_API_URL` tells MCP where the API is
- `MASDAY_API_KEY` for auth between MCP and API

### 11.5 Code Skills (`packages/code-skills`)

The code skills package contains plain async functions for git, npm, docker, tests, etc. These split into two categories:

| Skill Type | Current (TS) | Rust Approach |
|-----------|-------------|---------------|
| Shell wrappers (git, npm, docker, gh) | `execSync()` in MCP | `tokio::process::Command` — stays local in MCP, no HTTP needed |
| Data skills (workflow, memory, review) | DualWriteStore calls | Move to `masday-service` crate, exposed via API |

The shell wrapper skills (`git_status`, `npm_run`, `docker_ps`, etc.) **must stay in the MCP process** because they operate on the local filesystem where Claude Code is running. The API server can't access the developer's local machine.

### 11.6 Summary: What Moves vs What Stays

```
┌─────────────────────────────────────────────────────────┐
│  STAYS IN CLAUDE CODE LAYER (not converted to Rust)     │
│                                                         │
│  • Skills (.md) — instructions for Claude               │
│  • Agents (.md) — role descriptions                     │
│  • Hooks (.js) — PreToolUse step enforcement            │
│  • .claude/ config — settings.json, CLAUDE.md           │
│  • Shell tools — git, npm, docker, gh (local fs access) │
│  • Filesystem tools — read, write, list, delete, stat   │
│  • Capability tools — reads .claude/ directory locally  │
└─────────────────────────────────────────────────────────┘
                          │
                          │  mcp__masday__* tool calls
                          ▼
┌─────────────────────────────────────────────────────────┐
│  MOVES TO RUST                                          │
│                                                         │
│  • MCP stdio server (masday-mcp crate)                  │
│    - Registers same 89 tool names                       │
│    - Data tools → HTTP calls to API                     │
│    - Shell/tools → tokio::process::Command (local)      │
│                                                         │
│  • API server (masday-api crate)                        │
│    - All /api/* endpoints                               │
│    - Owns all PostgreSQL access                         │
│                                                         │
│  • Business logic (masday-service crate)                │
│    - Workflow state machine                             │
│    - Memory scoring, search                             │
│    - Policy validation                                  │
│                                                         │
│  • Database layer (masday-db crate)                     │
│    - All repos, migrations, pool                        │
└─────────────────────────────────────────────────────────┘
```

### 11.7 Local-Only Tools (Stays in MCP Process)

These 27 tools operate on the developer's local machine and **never go through the API**:

| Namespace | Tools | Why Local |
|-----------|-------|-----------|
| filesystem | read, write, list, delete, stat | Reads/writes local files |
| git | status, diff, commit | Runs `git` CLI locally |
| npm | install, run | Runs `pnpm` CLI locally |
| docker | build, run, ps | Runs `docker` CLI locally |
| cicd | pipeline_status, pipeline_trigger, runs_view | Runs `gh` CLI locally |
| github | pr_create, pr_list, issue_list | Runs `gh` CLI locally |
| tests | run | Runs `pnpm test` locally |
| local | init, sync, push, save_artifact | Manages `.masday/` state dir |
| capability | list_agents, list_skills, list_templates, match_agent, create_agent, create_skill, scaffold_feature, scaffold_mcp_server, system_readiness, workflow_audit, ping | Reads `.claude/` directory |

In Rust, these become `tokio::process::Command` calls inside `masday-mcp` — no HTTP involved.

---

## 12. What Stays in TypeScript

Not everything needs to convert:

| Component | Keep in TS | Reason |
|-----------|-----------|--------|
| Dashboard (`apps/dashboard`) | Yes | Next.js frontend, calls Rust API |
| Desktop app (`apps/desktop`) | Yes | Electron wrapper |
| `.claude/` skills & agents | Yes | Markdown instructions consumed by Claude Code |
| `.claude/` hooks | Yes | JS PreToolUse hooks running in Claude Code runtime |
| `.claude/` config files | Yes | settings.json, CLAUDE.md — not app code |

---

## 12. Performance Targets

| Metric | Current (TS) | Target (Rust) |
|--------|-------------|----------------|
| Cold start | ~3s (Drizzle init) | <500ms |
| Tool call latency | ~50ms (DualWrite + Drizzle) | <20ms (sqlx + API) |
| Memory usage | ~200MB (Node.js) | ~30MB (release build) |
| Max concurrent connections | Pool exhaustion at ~20 | 1000+ with deadpool |
| Stale connection recovery | Background reconnect every 15s | Automatic pool recycling |
| Binary size | N/A (runtime) | ~15MB (statically linked) |

---

## 13. Disk Leak Prevention (CRITICAL)

Rust builds accumulate massive artifacts in `target/` — easily 5-10 GB after repeated `cargo build`/`cargo test`. Without mitigation, disk fills silently.

### 13.1 The Problem

| Source | Size | Growth Pattern |
|--------|------|---------------|
| `target/debug/` | 2-5 GB | Grows on every `cargo build` — incremental artifacts, debug symbols |
| `target/release/` | 1-3 GB | Each `cargo build --release` adds full artifacts |
| `target/debug/deps/` | 1-3 GB | Dependency compile cache — rarely cleaned automatically |
| `target/debug/incremental/` | 500 MB-2 GB | Incremental compilation state — accumulates across sessions |
| `.sqlx/` | 5-20 MB | Offline query metadata — regenerated by `cargo sqlx prepare` |

### 13.2 `.gitignore` Rules

```gitignore
# Rust build artifacts — NEVER commit these
target/
**/*.rs.bk
*.pdb

# sqlx offline cache (regenerated)
.sqlx/

# Cargo.lock for libraries (commit for binaries only)
# masday-core/Cargo.lock
# masday-db/Cargo.lock
```

### 13.3 `Cargo.toml` Workspace Settings

```toml
[workspace]
members = ["masday-core", "masday-db", "masday-service", "masday-api", "masday-mcp", "masday-cli"]
resolver = "2"

# Shared build profile — minimize disk usage
[profile.dev]
debug = 1              # line-tables-only instead of full debug info → saves ~60%
incremental = true     # keep incremental for dev speed

[profile.release]
debug = 0              # no debug symbols in release
lto = "thin"           # smaller binaries, some compile-time cost
strip = true           # strip symbols from binary
opt-level = "s"        # optimize for size (not speed) — saves disk on binary
panic = "abort"        # smaller binary, no unwind tables
codegen-units = 1      # better optimization, smaller output
```

### 13.4 Automated Cleanup (CI/CD + Dev Scripts)

**`scripts/clean-rust.sh`** — run weekly or when disk > 80%:
```bash
#!/bin/bash
# Safe cleanup — preserves dependency cache
cargo clean --release              # remove release artifacts only
cargo clean --doc                  # remove generated docs
# DO NOT run full `cargo clean` — it wipes dependency cache too (slow rebuild)

# Nuclear option (when disk critical):
# cargo clean                       # removes EVERYTHING in target/
```

**`.github/workflows/ci.yml`** — prevent CI disk leak:
```yaml
# Cache target/ but with size limit
- uses: Swatinem/rust-cache@v2
  with:
    cache-on-failure: false
    cache-all-crates: false        # only cache workspace crates
    job-key: rust-build            # per-job isolation
    # Auto-evicts when cache > 2GB
```

### 13.5 `cargo-sweep` Integration (Recommended)

```bash
# Install once
cargo install cargo-sweep

# Remove build artifacts older than 7 days
cargo sweep --time 7

# Remove all build artifacts (keep source)
cargo sweep --installed

# Check disk usage before/after
du -sh target/
cargo sweep --time 30
du -sh target/
```

### 13.6 Pre-commit Hook — Disk Guard

Add to `.husky/pre-commit` or equivalent:

```bash
# Warn if target/ exceeds 5GB
TARGET_SIZE=$(du -sm target/ 2>/dev/null | cut -f1)
if [ "$TARGET_SIZE" -gt 5120 ]; then
  echo "WARNING: target/ is ${TARGET_SIZE}MB — run 'cargo sweep --time 7' to clean"
fi
```

### 13.7 `.editorconfig` / IDE Settings

```ini
# Prevent IDE from indexing target/
# VS Code: already excluded via Files:Exclude
# IntelliJ: mark target/ as Excluded in Project Structure
# rust-analyzer: set rust-analyzer.files.excludeDirs = ["target"]
```

### 13.8 TL;DR Checklist

- [ ] `target/` in `.gitignore` (first priority)
- [ ] `debug = 1` in dev profile (saves 60% disk)
- [ ] `strip = true` + `lto = "thin"` in release profile
- [ ] `cargo-sweep --time 7` in weekly cron or CI cleanup job
- [ ] rust-analyzer exclude `target/` from indexing
- [ ] CI cache with `Swatinem/rust-cache@v2` + size limits
- [ ] Disk guard pre-commit hook (warn at 5GB)

---

## 14. Testing Strategy

```bash
# Unit tests (per crate)
cargo test -p masday-service
cargo test -p masday-db

# Integration tests (API + DB)
cargo test -p masday-api -- --test-threads=1

# sqlx offline mode (CI without DB)
cargo sqlx prepare --database-url $DATABASE_URL

# Lint
cargo clippy -- -D warnings
cargo fmt --check
```

---

## 14. Quick Start (After Phase 1)

```bash
# 1. Start PostgreSQL
docker compose up -d postgres

# 2. Run migrations
cd masday-rust
cargo sqlx migrate run

# 3. Start API server
cargo run -p masday-api
# → Listening on http://localhost:3001

# 4. Start MCP server (in another terminal)
cargo run -p masday-mcp
# → Connected to http://localhost:3001

# 5. Test
curl http://localhost:3001/api/health
curl -X POST http://localhost:3001/api/workflows \
  -H "Authorization: Bearer $API_KEY" \
  -d '{"name": "test", "description": "Test workflow"}'
```
