# Masday Workflow - Architecture

> **Status note:** This document should describe actual runtime behavior in this repository, not the full aspirational surface accumulated across phase docs. Treat the local-first MCP path as the canonical runtime. Docker is optional. Agent/intelligence subsystems should be read with feature maturity in mind rather than assumed universal runtime exposure.

## Overview

Masday Workflow is an AI-agent workflow project built on the Model Context Protocol (MCP). The repository contains an active local-first runtime path, supporting workflow documentation, and a wider set of experimental or phase-specific materials that should not be read as equally mature production behavior.

## System Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                     Claude Code / AI Client                  │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐      │
│  │ .claude/      │  │ Slash        │  │ Auto-loaded  │      │
│  │ skills/       │  │ Commands     │  │ agents/      │      │
│  │ commands/     │  │ /masday-*    │  │ hooks/       │      │
│  └──────────────┘  └──────────────┘  └──────────────┘      │
└────────────────────────────┬────────────────────────────────┘
                             │ MCP Protocol (stdio)
                             ▼
┌─────────────────────────────────────────────────────────────┐
│                     MCP Server Layer                         │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐      │
│  │ Server       │  │ Registry     │  │ Schema       │      │
│  │ (stdio)      │  │ (Skill DB)   │  │ (Zod/Valid.) │      │
│  └──────────────┘  └──────────────┘  └──────────────┘      │
└────────────────────────────┬────────────────────────────────┘
                             │
                             ▼
┌─────────────────────────────────────────────────────────────┐
│                   Workflow Orchestrator                     │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐      │
│  │ State        │  │ Workflow     │  │ Task Manager │      │
│  │ Machine      │  │ Engine       │  │              │      │
│  └──────────────┘  └──────────────┘  └──────────────┘      │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐      │
│  │ DAG Executor │  │ Planner      │  │ Task Queue   │      │
│  │ (Parallel)   │  │ (Rule-based) │  │ (Priority)   │      │
│  └──────────────┘  └──────────────┘  └──────────────┘      │
└────────────────────────────┬────────────────────────────────┘
                             │
                    ┌────────┴────────┐
                    ▼                 ▼
┌──────────────────────────┐  ┌──────────────────────────────┐
│     Agent Layer          │  │   Intelligence Layer        │
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
│  │ filesystem.*       │  │  │  │ SQLite Store           │  │
│  │ git.*              │  │  │  │ (workflow state, tasks,│  │
│  │ code.*             │  │  │  │ runtime metadata)      │  │
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

| Layer           | Technology                | Purpose                     |
| --------------- | ------------------------- | --------------------------- |
| Language        | TypeScript 5.3            | Strict typing, all packages |
| Runtime         | Node.js 20+               | Server execution            |
| Package Manager | pnpm (workspaces)         | Monorepo management         |
| Protocol        | @modelcontextprotocol/sdk | MCP server/client           |
| Validation      | Zod                       | Schema validation           |
| Logging         | Pino                      | Structured logging          |
| Events          | EventEmitter3             | Pub/sub event bus           |
| Storage         | SQLite (better-sqlite3)   | Workflow state persistence  |
| Module System   | CommonJS                  | Package output              |
| Testing         | Vitest (v8 coverage)      | 1017+ tests across 82+ files |

## Monorepo Structure

```
masday-workflow/
├── packages/
│   ├── core/                  # Types, event bus, logger, task model
│   ├── mcp-server/            # MCP server + skill registry
│   ├── orchestrator/          # Workflow engine + state machine (3-tier hierarchy)
│   ├── skills/                # Filesystem skills
│   ├── code-skills/           # Git, tests, npm, code, docker, CI/CD, GitHub
│   ├── store/                 # SQLite persistence (WorkflowStore, TaskResultStore, ConfigStore)
│   ├── agents/                # Multi-agent coordination + skill routing
│   ├── cli/                   # CLI binaries (masday-workflow, masday-init) + setup templates
│   └── intelligence/          # Repository intelligence
├── apps/
│   └── agent-runner/          # CLI + MCP server entry point
├── docs/                      # Documentation
├── .claude/                   # Claude Code integration (project-level)
│   ├── skills/                # 8 workflow and builder skills
│   ├── commands/              # 13 slash commands
│   ├── agents/                # 5 agent definitions (backend, frontend, qa, orchestrator, researcher)
│   └── hooks/                 # 3 validation hooks
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

| Component  | Location                          | Scope        |
| ---------- | --------------------------------- | ------------ |
| MCP Server | `settings.json` → mcpServers      | All projects |
| Commands   | `~/.claude/commands/*.md` (13)    | All projects |
| Skills     | `~/.claude/skills/*/SKILL.md` (8) | All projects |
| Agents     | `~/.claude/agents/*.md` (5)       | All projects |

### Per-Project Setup

When using masday-workflow in any project:

1. `/masday-workflow-init` — creates `.masday/` data directory
2. Auto-analyzes codebase → caches in `.masday/research/`
3. All subsequent commands use cached context (60-80% token savings)

### Data Flow

```
Claude Code Session
  ├── Reads .masday/context/ → instant project knowledge (no scan)
  ├── Calls MCP tools → workflow execution
  └── Writes human-readable artifacts to .masday/ when commands or skills export summaries

SQLite Store (MCP)     .masday/ (Local)
  ├─ workflow state      ├─ research/ (analysis cache)
  ├─ task results        ├─ context/ (project summary)
  ├─ runtime metadata    ├─ plans/ (pre-execution plans)
  └─ config              └─ notes/ (execution logs / summaries)
```

### State Ownership

- **SQLite is the operational source of truth** for active workflows, task state, task results, and runtime execution metadata.
- **`.masday/` is the project-local artifact space** for human-readable outputs such as research snapshots, context summaries, plans, and notes.
- If a command exports workflow information into `.masday/`, that export is a convenience artifact, not the authoritative runtime state store.

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
 ┌─────────────┐                ┌─────────────────┐
 │   Client     │ ────────────► │   MCP Server     │
 │ (Dashboard/  │   stdio       │  (71 tools)      │
 │  CLI/MCP)    │               └────────┬──────────┘
 └─────────────┘                         │
                                         ▼
                             ┌───────────────────────┐
                             │    ORCHESTRATOR        │
                             │                       │
                             │  1. Working Memory     │── Session state (RAM)
                             │  2. Episodic Memory    │── Chat history (10 msgs)
                             │  3. Memory Search      │── Top 10, importance >= 0.2
                             │  4. Context Builder    │── Assembles prompt
                             └───────────┬───────────┘
                                         │
                             ┌───────────┴───────────┐
                             │    AGENT ROUTER        │
                             │  scoring-based match   │
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
                             │  (Pino + EventEmitter) │── Subscribers
                             └───────────────────────┘
```

## Memory Stack

```
  ┌──────────────────────────────────────────────────────────┐
  │                   WORKING MEMORY                         │
  │              In-process RAM, per session                  │
  │          Fastest -- current task state & context           │
  └──────────────────────────────────────────────────────────┘
                            │
  ┌──────────────────────────────────────────────────────────┐
  │                  EPISODIC MEMORY                         │
  │            Last 10 messages per session                   │
  │         Chat history -- recent conversation context        │
  └──────────────────────────────────────────────────────────┘
                            │
  ┌──────────────────────────────────────────────────────────┐
  │                 LONG-TERM MEMORY                         │
  │                                                          │
  │   Scoring: similarity*0.6 + recency*0.15                │
  │            + importance*0.15 + usage*0.1                 │
  │                                                          │
  │   File Store (.masday/state/memories.json)               │
  │   + Jaccard similarity (embedding-ready for pgvector)    │
  └──────────────────────────────────────────────────────────┘
                            │
  ┌──────────────────────────────────────────────────────────┐
  │                 KNOWLEDGE GRAPH                          │
  │             Nodes & edges, auto-linked                   │
  │        Traversal, subgraph, relationship queries         │
  └──────────────────────────────────────────────────────────┘
```

## Engine Tier Hierarchy

All engines inherit from `BaseWorkflowEngine` which provides shared logic for workflow storage, UUID generation, `getStatus`, `addTask`, `getWorkflow`, and `listWorkflows`.

| Engine                  | Inherits From           | Adds                                              |
| ----------------------- | ----------------------- | ------------------------------------------------- |
| WorkflowEngine          | BaseWorkflowEngine      | Linear state machine, sequential task execution   |
| EnhancedWorkflowEngine  | BaseWorkflowEngine      | Planner + DAGExecutor, retry logic, VERIFY + FIX  |
| OrchestratingEngine     | EnhancedWorkflowEngine  | AgentCoordinator, SkillRouter, TaskQueue, agent dispatch |

The MCP server uses `OrchestratingEngine` with `coordinator: true` and `enableSkillRouting: true`, making the full multi-agent dispatch path available at runtime.

### FIX Retry Logic

When tasks fail during EXECUTE or VERIFY:
1. Engine transitions to FIX state
2. Failed tasks are reset to `pending` status
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
- **Memory persistence**: Memory store persists to file after each add (calls `save()`)
- **Startup initialization**: Session, Review, and Parallel tables are initialized at startup
