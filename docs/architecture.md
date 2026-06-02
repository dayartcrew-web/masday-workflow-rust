# Masday Workflow - Architecture

> **Status:** This document describes the Rust runtime architecture. PostgreSQL is the operational source of truth for all workflow, task, memory, and session state.

## Overview

Masday Workflow is a unified AI coding agent platform built on the Model Context Protocol (MCP). Rust workspace with 6 crates, 20 MCP tool domains, 4-layer memory system, and state machine workflow engine — all backed by PostgreSQL via deadpool-postgres.

## System Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                     Claude Code / AI Client                 │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐       │
│  │ .claude/     │  │ Slash        │  │ Auto-loaded  │       │
│  │ skills/      │  │ Commands     │  │ agents/      │       │
│  │ commands/    │  │ /masday-*    │  │ hooks/       │       │
│  └──────────────┘  └──────────────┘  └──────────────┘       │
└────────────────────────────┬────────────────────────────────┘
                             │ MCP Protocol (stdio)
                             ▼
┌─────────────────────────────────────────────────────────────┐
│                     MCP Server (masday-mcp)                 │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐       │
│  │ stdio        │  │ 20 Tool      │  │ serde_json   │       │
│  │ transport    │  │ Domains      │  │ validation   │       │
│  └──────────────┘  └──────────────┘  └──────────────┘       │
└────────────────────────────┬────────────────────────────────┘
                             │
                             ▼
┌─────────────────────────────────────────────────────────────┐
│                   Service Layer (masday-service)            │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐       │
│  │ Workflow      │  │ Task         │  │ Memory       │       │
│  │ Service       │  │ Service      │  │ Service      │       │
│  │ (state machine)│  │ (auto-       │  │ (4-layer)    │       │
│  │               │  │  transition) │  │              │       │
│  └──────────────┘  └──────────────┘  └──────────────┘       │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐       │
│  │ Policy        │  │ Review       │  │ Reminder     │       │
│  │ Service       │  │ Service      │  │ Service      │       │
│  └──────────────┘  └──────────────┘  └──────────────┘       │
└────────────────────────────┬────────────────────────────────┘
                             │
                    ┌────────┴────────┐
                    ▼                 ▼
┌──────────────────────────┐  ┌──────────────────────────────┐
│   Skill Layer            │  │   Persistence Layer          │
│  ┌────────────────────┐  │  │  ┌────────────────────────┐  │
│  │ filesystem.*       │  │  │  │ PostgreSQL (port 54341)  │  │
│  │ git.*              │  │  │  │ deadpool-postgres pool  │  │
│  │ tests.*            │  │  │  │ 15 repo modules         │  │
│  │ npm.*              │  │  │  │ 16 tables               │  │
│  │ docker.*           │  │  │  └────────────────────────┘  │
│  │ github.*           │  │  │  ┌────────────────────────┐  │
│  │ cicd.*             │  │  │  │ .masday/ (per-project) │  │
│  └────────────────────┘  │  │  │ research/ context/     │  │
│                          │  │  │ plans/ notes/          │  │
│                          │  │  └────────────────────────┘  │
└──────────────────────────┘  └──────────────────────────────┘
```

## Tech Stack

| Layer           | Technology                                    | Purpose                              |
| --------------- | --------------------------------------------- | ------------------------------------ |
| Language        | Rust 2021 edition                             | Memory safety, performance           |
| Async Runtime   | tokio                                         | Async I/O, task spawning            |
| HTTP            | Axum 0.8                                      | REST API server                      |
| Database        | PostgreSQL via deadpool-postgres + tokio-postgres | Connection pool, raw SQL queries |
| MCP             | Custom implementation (serde_json)            | 20 tool domains, stdio transport     |
| CLI             | clap 4.5                                      | Command-line interface               |
| Error Handling  | thiserror (lib) + anyhow (app)                | Typed errors                         |
| Serialization   | serde + serde_json                            | JSON for API/MCP boundaries          |
| Testing         | cargo test + #[tokio::test]                   | Unit + integration tests             |

## Workspace Structure

```
masday-workflow-rust/
├── masday-core/                # Shared types, error types, enums
│   └── src/types.rs            # WorkflowState, TaskState, PlanState, etc.
├── masday-db/                  # Database layer
│   ├── src/repos/              # 15 repo modules (workflow, task, memory, ...)
│   └── src/schema.rs           # Row types, NewXxx structs
├── masday-service/             # Business logic (10 services)
│   ├── workflow_service.rs     # State machine, transitions, auto-advance
│   ├── task_service.rs         # Task lifecycle, auto-transition to DONE
│   ├── memory_service.rs       # 4-layer memory, scoring, BM25
│   ├── policy_service.rs       # Validation, audit, drift detection
│   └── ...                     # plan, review, reminder, search, capability, context
├── masday-api/                 # Axum HTTP server
│   └── src/routes/             # REST endpoints (workflow, task, health, ...)
├── masday-mcp/                 # MCP server (stdio)
│   └── src/tools/              # 20 tool domain modules
├── masday-cli/                 # CLI installer binary
│   └── src/installer/          # Template embedding, local/remote modes
├── scripts/                    # Git hooks, release, setup
├── docs/                       # Documentation
├── .claude/                    # Claude Code integration
│   ├── skills/                 # 25+ workflow and builder skills
│   ├── agents/                 # 22+ agent definitions
│   └── hooks/                  # Validation hooks
├── .masday/                    # Project-local artifacts
│   ├── research/               # Cached codebase analysis
│   ├── context/                # Project context + summaries
│   └── plans/                  # Task plans
├── CLAUDE.md                   # Claude Code project instructions
└── Cargo.toml                  # Workspace root
```

## Service Layer (10 Services)

| Service | File | Key Responsibilities |
|---------|------|---------------------|
| WorkflowService | `workflow_service.rs` | State machine, valid transitions, auto-advance through INIT→ANALYZE→PLAN→EXECUTE |
| TaskService | `task_service.rs` | Task lifecycle (PENDING→RUNNING→DONE), auto-transition workflow when all tasks done |
| PlanService | `plan_service.rs` | Plan CRUD, versioning |
| MemoryService | `memory_service.rs` | 4-layer memory (working/episodic/long-term/graph), scoring, BM25 |
| PolicyService | `policy_service.rs` | Completion validation, drift detection, audit |
| ReviewService | `review_service.rs` | Review decisions (APPROVED/REWORK_REQUIRED/BLOCKED) |
| ReminderService | `reminder_service.rs` | Stale/stuck workflow detection |
| SearchService | `search_service.rs` | Semantic search, fingerprinting |
| CapabilityService | `capability_service.rs` | Agent/skill registry, system readiness |
| ContextService | `context_service.rs` | Context pack building, fingerprint computation |

## Status Conventions

All status values stored in **UPPERCASE** in PostgreSQL:

| Entity   | Valid Statuses                                                         |
| -------- | ---------------------------------------------------------------------- |
| Workflow | INIT, ANALYZE, PLAN, EXECUTE, VERIFY, FIX, DONE, FAILED, PAUSED        |
| Task     | PENDING, RUNNING, DONE, FAILED                                         |
| Plan     | ACTIVE, PENDING, READY, DONE                                           |
| Review   | APPROVED, REWORK_REQUIRED, BLOCKED                                     |

## 16 Database Tables

| Table             | Repo Module              | Primary Triggers                                    |
| ----------------- | ------------------------ | --------------------------------------------------- |
| Workflow          | workflow_repo            | create, execute, delete, transition                 |
| Task              | task_repo                | addTask, startTask, completeTask                     |
| Plan              | plan_repo                | createPlan                                          |
| Memory            | memory_repo              | store, search, recall                               |
| ReviewDecision    | review_repo              | review_submit                                       |
| SessionState      | session_repo             | session_patch_state                                 |
| ParallelBranch    | branch_repo              | createParallelBranches, completeParallelBranch      |
| ContextDocument   | context_document_repo    | store_research                                      |
| TaskProgressLog   | progress_log_repo        | saveProgress                                        |
| RetrievalLog      | retrieval_log_repo       | memory_search, code_search                          |
| TokenUsage        | token_usage_repo         | saveProgress, store_research                        |
| EpisodicMemory    | episodic_memory_repo     | Episodic add (all MCP tool calls)                   |
| GraphNode         | graph_repo               | memory_store, workflow_create, addTask              |
| GraphEdge         | graph_repo               | Jaccard auto-link (threshold 0.3)                   |
| WorkflowReminder  | reminder_repo            | Startup + 15min interval check                      |
| LlmProviderConfig | llm_provider_config_repo | LLM provider configuration                          |

## Workflow States

```
INIT ──> ANALYZE ──> PLAN ──> EXECUTE ──> VERIFY ──> DONE
  │                    │    │      │          │
  └──> DONE            │    │      └──> FIX ──┤
  └──> FAILED          │    └──> PAUSED       └──> FIX ──> EXECUTE
                       └──> FAILED    │
                                      └──> FAILED
                                         FIX ──> DONE
                                         FIX ──> FAILED
```

**Auto-transition:** When all tasks in a workflow reach DONE status, `TaskService::auto_transition_if_all_done()` automatically transitions the workflow to DONE (or through VERIFY first if in EXECUTE state).

## Memory Stack

```
  ┌──────────────────────────────────────────────────────────┐
  │                   WORKING MEMORY                         │
  │              In-process RAM, per session                 │
  └──────────────────────────────────────────────────────────┘
                            │
  ┌──────────────────────────────────────────────────────────┐
  │                  EPISODIC MEMORY                         │
  │            Last N messages per session                   │
  │         Persisted to EpisodicMemory table                │
  └──────────────────────────────────────────────────────────┘
                            │
  ┌──────────────────────────────────────────────────────────┐
  │                 LONG-TERM MEMORY                         │
  │   Scoring: similarity*0.6 + importance*0.2               │
  │            + recency*0.1 + usage*0.1                      │
  │   Memory table (PostgreSQL)                              │
  └──────────────────────────────────────────────────────────┘
                            │
  ┌──────────────────────────────────────────────────────────┐
  │                 KNOWLEDGE GRAPH                          │
  │             Nodes & edges, auto-linked                   │
  │        GraphNode + GraphEdge tables                      │
  └──────────────────────────────────────────────────────────┘
```

## Build Profile

```toml
[profile.release]
debug = 0
lto = "thin"
strip = true
opt-level = "s"        # Optimize for size
panic = "abort"
codegen-units = 1
```

Result: ~3-8MB binaries (masday-api, masday-mcp, masday-cli).
