# Masday Workflow - Architecture

> **Status note:** This document describes the actual runtime behavior in this repository. The local-first MCP path is the canonical runtime. PostgreSQL via DualWriteStore is the operational source of truth for all workflow, task, memory, and session state.

## Overview

Masday Workflow is a unified AI coding agent platform built on the Model Context Protocol (MCP). It merges a 5-domain MCP server architecture with a 4-layer memory system, 3-tier workflow engine, and code skills -- all backed by PostgreSQL (Drizzle) with a JSON/SQLite cache fallback. The repository is a pnpm monorepo with 13 packages and a single unified MCP server app exposing 87 tools.

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
│                     MCP Server Layer                        │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐       │
│  │ Server       │  │ Registry     │  │ Schema       │       │
│  │ (stdio)      │  │ (Skill DB)   │  │ (Zod/Valid.) │       │
│  └──────────────┘  └──────────────┘  └──────────────┘       │
└────────────────────────────┬────────────────────────────────┘
                             │
                             ▼
┌─────────────────────────────────────────────────────────────┐
│                   Workflow Orchestrator                     │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐       │
│  │ State        │  │ Workflow     │  │ Task Manager │       │
│  │ Machine      │  │ Engine       │  │              │       │
│  └──────────────┘  └──────────────┘  └──────────────┘       │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐       │
│  │ DAG Executor │  │ Planner      │  │ Task Queue   │       │
│  │ (Parallel)   │  │ (Rule-based) │  │ (Priority)   │       │
│  └──────────────┘  └──────────────┘  └──────────────┘       │
└────────────────────────────┬────────────────────────────────┘
                             │
                    ┌────────┴────────┐
                    ▼                 ▼
┌──────────────────────────┐  ┌──────────────────────────────┐
│     Agent Layer          │  │   Intelligence Layer         │
│  ┌────────────────────┐  │  │  ┌────────────────────────┐  │
│  │ Agent Coordinator  │  │  │  │ Code Indexer           │  │
│  └────────────────────┘  │  │  └────────────────────────┘  │
│  ┌────────────────────┐  │  │  ┌────────────────────────┐  │
│  │ Skill Router       │  │  │  │ Semantic Searcher      │  │
│  └────────────────────┘  │  │  └────────────────────────┘  │
│  ┌────────────────────┐  │  │  ┌────────────────────────┐  │
│  │ Agent Workers      │  │  │  │ Code Graph Analyzer    │  │
│  │ (be/fe/qa/gen)     │  │  │  └────────────────────────┘  │
│  └────────────────────┘  │  │  ┌────────────────────────┐  │
│                          │  │  │ Learning System        │  │
│                          │  │  └────────────────────────┘  │
└──────────────────────────┘  └──────────────────────────────┘
                             │
                    ┌────────┴────────┐
                    ▼                 ▼
┌──────────────────────────┐  ┌──────────────────────────────┐
│      Skill Layer         │  │   Persistence Layer          │
│  ┌────────────────────┐  │  │  ┌────────────────────────┐  │
│  │ filesystem.*       │  │  │  │ DualWriteStore         │  │
│  │ git.*              │  │  │  │ (PostgreSQL via Drizzle │  │
│  │ code.*             │  │  │  │  + JSON cache fallback)│  │
│  │ tests.*            │  │  │  └────────────────────────┘  │
│  │ npm.*              │  │  │  ┌────────────────────────┐  │
│  │ docker.*           │  │  │  │ .masday/ (per-project) │  │
│  │ github.*           │  │  │  │ research/ context/     │  │
│  │ cicd.*             │  │  │  │ plans/ notes/ exports  │  │
│  └────────────────────┘  │  │  └────────────────────────┘  │
│                          │  │  ┌────────────────────────┐  │
│                          │  │  │ Event Bus (pub/sub)    │  │
│                          │  │  │ Logger (Pino)          │  │
│                          │  │  └────────────────────────┘  │
└──────────────────────────┘  └──────────────────────────────┘
```

## Tech Stack

| Layer           | Technology                                    | Purpose                              |
| --------------- | --------------------------------------------- | ------------------------------------ |
| Language        | TypeScript 5.6                                | Strict typing, all packages          |
| Runtime         | Node.js 20+                                   | Server execution                     |
| Package Manager | pnpm (workspaces)                             | Monorepo management                  |
| Build           | Turbo                                         | Parallel build orchestration         |
| Protocol        | @modelcontextprotocol/sdk                     | MCP server/client (official McpServer) |
| Validation      | Zod                                           | Schema validation                    |
| Logging         | Pino                                          | Structured logging                   |
| Events          | EventEmitter3                                 | Pub/sub event bus                    |
| Storage         | DualWriteStore (PostgreSQL via Drizzle + JSON/SQLite cache) | Workflow state persistence |
| ORM             | Drizzle                                       | Database access, schema, migrations  |
| Semantic Search | pgvector                                      | Vector similarity in PostgreSQL      |
| Module System   | ESM (`"type": "module"`, NodeNext resolution) | All packages use ESM                 |
| Testing         | Vitest (globals enabled)                      | Unit, integration, benchmarks        |

## Monorepo Structure

```
masday-workflow-rebuild/
├── packages/
│   ├── core/                  # Shared types, logger, EventBus, tracing, metrics
│   ├── shared-utils/          # Logger, IDs, hash, env utilities
│   ├── db/                    # Drizzle schema (16 models + pgvector), client singleton (packages/db/src/schema.ts)
│   ├── store/                 # StorageBackend, SQLite, JSON, Drizzle adapters
│   ├── llm/                   # Multi-provider LLM (Anthropic, OpenAI, Custom), circuit breaker
│   ├── memory/                # 4-layer memory (working, episodic, long-term, graph), scoring, BM25
│   ├── workflow-engine/       # Pure functions + state machine, DAG, session, review, parallel, drift
│   ├── intelligence/          # SemanticSearcher, CodeIndexer, ReAct agent, Guardrails
│   ├── policy/                # PolicyValidator, WorkflowAuditor, drift detection
│   ├── capability/            # Registry, Scaffolder, SystemHealth
│   ├── code-skills/           # Git, tests, npm, code, docker, github, CI/CD (plain functions + Zod)
│   ├── project-rules/         # Refactor rules engine, 14 automated checks, checklist validator
│   └── cli/                   # CLI entry point + setup templates
├── apps/
│   └── agent-runner/          # Single unified MCP server (87 tools)
├── docs/                      # Documentation
├── .claude/                   # Claude Code integration (project-level)
│   ├── skills/                # 25+ workflow and builder skills
│   ├── commands/              # 9+ slash commands
│   ├── agents/                # 22+ agent definitions
│   └── hooks/                 # Validation hooks
├── .masday/                   # Project-local human-readable artifacts
│   ├── research/              # Cached codebase analysis
│   ├── context/               # Project context + summaries
│   ├── plans/                 # Task plans and exported summaries
│   └── notes/                 # Execution notes
├── .mcp.json                  # MCP server config for Claude Code
├── CLAUDE.md                  # Claude Code project instructions
└── package.json               # Root workspace config
```

## Claude Code Integration

### Global Setup (`~/.claude/`)

| Component  | Location                              | Scope        |
| ---------- | ------------------------------------- | ------------ |
| MCP Server | `settings.json` -> mcpServers         | All projects |
| Commands   | `~/.claude/commands/*.md` (9+)        | All projects |
| Skills     | `~/.claude/skills/*/SKILL.md` (25+)   | All projects |
| Agents     | `~/.claude/agents/*.md` (22+)         | All projects |

### Per-Project Setup

When using masday-workflow in any project:

1. `/masday-workflow-init` -- creates `.masday/` data directory
2. Auto-analyzes codebase -> caches in `.masday/research/`
3. All subsequent commands use cached context (60-80% token savings)

### Data Flow

```
Claude Code Session
  ├── Reads .masday/context/ -> instant project knowledge (no scan)
  ├── Calls MCP tools -> workflow execution
  └── Writes human-readable artifacts to .masday/ when commands or skills export summaries

DualWriteStore (PostgreSQL + JSON cache)     .masday/ (Local)
  ├─ workflow state (Workflow table)           ├─ research/ (analysis cache)
  ├─ task results (Task table)                 ├─ context/ (project summary)
  ├─ runtime metadata (TaskProgressLog)        ├─ plans/ (pre-execution plans)
  ├─ memory (Memory table)                     └─ notes/ (execution logs / summaries)
  ├─ review decisions (ReviewDecision table)
  ├─ session state (SessionState table)
  ├─ parallel branches (ParallelBranch table)
  ├─ context documents (ContextDocument table)
  ├─ retrieval logs (RetrievalLog table)
  ├─ token usage (TokenUsage table)
  ├─ episodic memory (EpisodicMemory table)
  └─ knowledge graph (GraphNode + GraphEdge tables)
```

### State Ownership

- **PostgreSQL is the operational source of truth** for active workflows, task state, task results, memory, review decisions, session state, and all runtime execution metadata. All writes go through DualWriteStore which replicates to PostgreSQL in real-time via Drizzle.
- **JSON cache is the fallback** when PostgreSQL is unavailable. Memory operations use hybrid mode: Drizzle first, JSON cache fallback.
- **`.masday/` is the project-local artifact space** for human-readable outputs such as research snapshots, context summaries, plans, and notes. If a command exports workflow information into `.masday/`, that export is a convenience artifact, not the authoritative runtime state store.

## Status Conventions

All status values are stored in **UPPERCASE** in PostgreSQL:

| Entity   | Valid Statuses                                                         |
| -------- | ---------------------------------------------------------------------- |
| Workflow | INIT, ANALYZE, PLAN, EXECUTE, VERIFY, FIX, DONE, FAILED, PAUSED        |
| Task     | PENDING, RUNNING, DONE, FAILED                                         |
| Plan     | ACTIVE, PENDING, READY, DONE                                           |
| Review   | APPROVED, REWORK_REQUIRED, BLOCKED                                     |

DualWriteStore maps in-memory lowercase `TaskState` values to UPPERCASE for Drizzle persistence.

## 16 Drizzle Tables

All 16 Drizzle models are actively populated by the MCP server. Each table is wired through a specific persistence mechanism:

| Table             | Wired Via              | Trigger                                    |
| ----------------- | ---------------------- | ------------------------------------------ |
| Workflow          | DualWriteStore         | workflow_create, execute, delete           |
| Task              | DualWriteStore         | addTask, startTask, completeTask           |
| Plan              | DualWriteStore         | createPlan                                 |
| Memory            | persistToDb()      | memory_store, store_research               |
| ReviewDecision    | Drizzle direct          | review_submit                              |
| SessionState      | Drizzle direct          | session_patch_state                        |
| ParallelBranch    | Drizzle direct          | workflow_createParallelBranches            |
| ContextDocument   | Drizzle direct          | memory_store_research                      |
| TaskProgressLog   | saveProgressDb()       | workflow_saveProgress                      |
| RetrievalLog      | logRetrieval()         | memory_search, semantic-search_code_search, search_hybrid_context_pack |
| TokenUsage        | trackTokens()          | workflow_saveProgress, memory_store_research |
| EpisodicMemory    | setEpisodicDb()    | EpisodicMemory.add()                       |
| GraphNode         | setGraphDb()       | GraphStore.addNode()                       |
| GraphEdge         | setGraphDb()       | GraphStore.addEdge()                       |
| WorkflowReminder  | setReminderDb()    | reminder_check                             |
| LlmProviderConfig | Drizzle direct          | LLM provider configuration storage         |

## Workflow States

```
INIT ──> ANALYZE ──> PLAN ──> EXECUTE ──> VERIFY ──> DONE
  │                    │    │      │          │
  └──> DONE            │    │      └──> FIX ──┤
  └──> FAILED          │    └──> PAUSED       └──> FIX ──> EXECUTE
                       └──> FAILED    │
                                      └──> FAILED
                                         FIX ──> DONE
                                         FIX <──> FAILED
```

| State   | Purpose                                               |
| ------- | ----------------------------------------------------- |
| INIT    | Workflow created, awaiting planning                   |
| ANALYZE | Understanding requirements and codebase               |
| PLAN    | Generating task breakdown                             |
| EXECUTE | Running tasks through agents                          |
| VERIFY  | Validating outputs (checks for failed tasks)          |
| FIX     | Correcting failed tasks (configurable retry)          |
| DONE    | Workflow complete                                     |
| PAUSED  | Execution suspended, can resume to EXECUTE            |
| FAILED  | Unrecoverable error (reachable from any state)        |

## Request Lifecycle

```
  User Input
      │
      ▼
 ┌──────────────┐                ┌──────────────────┐
 │   Client     │ ────────────►  │   MCP Server     │
 │ (Dashboard/  │   stdio        │  (87 tools)      │
 │  CLI/MCP)    │                └────────┬─────────┘
 └──────────────┘                         │
                                          ▼
                             ┌────────────────────────┐
                             │    ORCHESTRATOR        │
                             │                        │
                             │  1. Working Memory     │── Session state (RAM)
                             │  2. Episodic Memory    │── Chat history
                             │  3. Memory Search      │── Top 10, importance >= 0.2
                             │  4. Context Builder    │── Assembles prompt
                             └───────────┬────────────┘
                                         │
                             ┌───────────┴───────────┐
                             │    AGENT ROUTER       │
                             │  scoring-based match  │
                             └───┬───────────────┬───┘
                                 │               │
                    ┌────────────▼───┐   ┌───────▼────────────┐
                    │  ReAct Agent   │   │  Autonomous Loop   │
                    │  Observe       │   │  (max 3 iterations)│
                    │  Think         │   │  Plan -> Execute   │
                    │  Act (tools)   │   │  -> Evaluate       │
                    └───────┬────────┘   └────────┬───────────┘
                            │                     │
                            └──────────┬──────────┘
                                       │
                                       ▼
                             ┌───────────────────────┐
                             │   POST-PROCESSING     │
                             │  5. Episodic save     │
                             │  6. Auto-classify     │── importance & type
                             │     └─> Memory Store  │
                             │  7. Eval + Reward     │── Quality scoring
                             │  8. Reflection        │── Conflict & merge
                             └──────────┬────────────┘
                                        │
                                        ▼
                             ┌───────────────────────┐
                             │    EVENT BUS          │
                             │  (Pino + EventEmitter)│── Subscribers
                             └───────────────────────┘
```

## Memory Stack

```
  ┌──────────────────────────────────────────────────────────┐
  │                   WORKING MEMORY                         │
  │              In-process RAM, per session                 │
  │          Fastest -- current task state & context         │
  └──────────────────────────────────────────────────────────┘
                            │
  ┌──────────────────────────────────────────────────────────┐
  │                  EPISODIC MEMORY                         │
  │            Last N messages per session                   │
  │         Chat history -- recent conversation context      │
  │         Persisted to EpisodicMemory table via Drizzle     │
  └──────────────────────────────────────────────────────────┘
                            │
  ┌──────────────────────────────────────────────────────────┐
  │                 LONG-TERM MEMORY                         │
  │                                                          │
  │   Scoring: similarity*0.6 + importance*0.2                │
  │            + recency*0.1 + usage*0.1                      │
  │                                                          │
  │   Memory table (PostgreSQL via Drizzle)                   │
  │   + BM25 + fastembed vector search (pgvector)            │
  └──────────────────────────────────────────────────────────┘
                            │
  ┌──────────────────────────────────────────────────────────┐
  │                 KNOWLEDGE GRAPH                          │
  │             Nodes & edges, auto-linked                   │
  │        Traversal, subgraph, relationship queries         │
  │        Persisted to GraphNode + GraphEdge tables         │
  └──────────────────────────────────────────────────────────┘
```

## Engine Tier Hierarchy

All engines inherit from `BaseWorkflowEngine` which provides shared logic for workflow storage, UUID generation, `getStatus`, `addTask`, `getWorkflow`, and `listWorkflows`.

| Engine                  | Inherits From           | Adds                                              |
| ----------------------- | ----------------------- | ------------------------------------------------- |
| WorkflowEngine          | BaseWorkflowEngine      | Linear state machine, sequential task execution   |
| EnhancedWorkflowEngine  | BaseWorkflowEngine      | Planner + DAGExecutor, retry logic, VERIFY + FIX  |
| OrchestratingEngine     | EnhancedWorkflowEngine  | AgentCoordinator, SkillRouter, TaskQueue, agent dispatch |

The MCP server uses `OrchestratingEngine` with `coordinator: false` and `enableSkillRouting: false`, making the full multi-agent dispatch path available at runtime.

### FIX Retry Logic

When tasks fail during EXECUTE or VERIFY:
1. Engine transitions to FIX state
2. Failed tasks are reset to PENDING status
3. Tasks are re-executed up to `maxFixRetries` (configurable)
4. `workflow.fixing` event emitted on each retry attempt
5. If retries are exhausted, the workflow fails

### Task Output Piping

The DAGExecutor enriches dependent task inputs with `dependencyOutputs` containing outputs from completed prerequisite tasks. This enables sequential task chains where downstream tasks can use upstream results.

### Agent Dispatch Flow

```
DAGExecutor.executeTask()
  → SkillExecutor (custom function)
    → AgentCoordinator.dispatchTask()
      → AgentWorker with matching capabilities
        → SkillRegistry.execute()
    → Fallback: direct SkillRegistry.execute()
```

4 default workers are registered on startup: `backend`, `frontend`, `qa`, `general-purpose`. The SkillRouter uses a 3-tier fallback: preferred agent override > SKILL_TO_AGENT default mapping > general-purpose.

### Event Emissions

| Event                       | Emitter              | Payload                    |
| --------------------------- | -------------------- | -------------------------- |
| `workflow.started`          | All engines          | `{ workflow }`             |
| `workflow.completed`        | All engines          | `{ workflow }`             |
| `workflow.failed`           | All engines          | `{ workflow, error }`      |
| `workflow.fixing`           | Enhanced, Orchestrating | `{ workflow, retryCount }` |
| `workflow.state.transition` | StateMachine         | `{ from, to, workflowId }` |
| `task.started`              | All engines          | `{ task }`                 |
| `task.completed`            | All engines          | `{ task }`                 |
| `task.failed`               | All engines          | `{ task, error }`          |

### Additional Runtime Behaviors

- **Auto-task creation**: `createPlan` auto-creates tasks from `plan.tasks[]` entries
- **Memory persistence**: Memory store persists to PostgreSQL after each add (calls `persistToDb()`), with JSON cache fallback
- **Startup initialization**: Session, Review, and Parallel tables are initialized at startup
- **DualWrite replication**: All workflow/task/plan operations replicate to PostgreSQL in real-time via DualWriteStore wrapping WorkflowStore
