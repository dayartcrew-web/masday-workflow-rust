# CLAUDE.md - masday-workflow-rebuild

## Project Overview

Unified AI coding agent platform built on Model Context Protocol (MCP).
Merges msd-mcp (official MCP SDK, 5 domain servers, Drizzle/PostgreSQL) with masday-workflow-reborn (4-layer memory, 3-tier workflow engine, code skills).
Monorepo: pnpm workspaces with TypeScript, ESM modules, Turbo build.

Package scope: `@mcp-rebuild/*`

## Architecture

```
User -> MCP Protocol (stdio) -> Domain MCP Servers -> Workflow Engine -> Core Infra
```

### Request Lifecycle

```
  User Input
      |
      v
 +-------------+                +------------------+
 |   Client     | ------------> |   MCP Server      |
 | (Dashboard/  |   stdio       |  (89 tools)       |
 |  CLI/MCP)    |               +--------+----------+
 +-------------+                         |
                                        v
                            +----------------------------+
                            |    WORKFLOW ENGINE         |
                            |                            |
                            |  Pure functions (msd)      |
                            |  + State machine (reborn)  |
                            |  + Session/Review/Parallel |
                            +-----------+----------------+
                                        |
                   +--------------------+--------------------+
                   |                    |                    |
                   v                    v                    v
          +--------------+     +--------------+     +--------------+
          |   MEMORY     |     | INTELLIGENCE |     |   POLICY     |
          |  4-layer     |     | Search/Index |     | Validators   |
          |  (w/e/l/g)   |     | ReAct Agent  |     | Audit/Drift  |
          +--------------+     +--------------+     +--------------+
```

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
  |   importanceScore: type-based + content length bonus     |
  +----------------------------------------------------------+
                           |
  +----------------------------------------------------------+
  |                 KNOWLEDGE GRAPH                          |
  |             Nodes & edges, auto-linked                   |
  +----------------------------------------------------------+
```

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

## Packages (13)

| Package | Description |
|---------|-------------|
| `packages/core` | Shared types, logger, EventBus, tracing, metrics |
| `packages/shared-utils` | Logger, IDs, hash, env (from msd-mcp) |
| `packages/db` | Drizzle schema (`src/schema.ts`, 16 `pgTable()` exports + pgvector), client via `drizzle()` + `postgres-js` |
| `packages/store` | StorageBackend, SQLite, JSON, Drizzle adapters |
| `packages/llm` | Multi-provider LLM (Anthropic, OpenAI, Custom), circuit breaker |
| `packages/memory` | 4-layer memory (working, episodic, long-term, graph), scoring, BM25 |
| `packages/workflow-engine` | Pure functions + state machine, DAG, session, review, parallel, drift |
| `packages/intelligence` | SemanticSearcher, CodeIndexer, ReAct agent, Guardrails |
| `packages/policy` | PolicyValidator, WorkflowAuditor, drift detection |
| `packages/capability` | Registry, Scaffolder, SystemHealth |
| `packages/code-skills` | Git, tests, npm, code, docker, github, CI/CD (plain functions + Zod) |
| `packages/project-rules` | Refactor rules engine, 14 automated checks, checklist validator |
| `packages/cli` | CLI entry point + setup templates |

## MCP Server — `apps/agent-runner` (89 tools)

Single unified MCP server at `apps/agent-runner/src/runtime/mcp.ts`. All 89 tools are real implementations connected to PostgreSQL via DualWriteStore.

| Namespace | Tools | Implementation |
|-----------|-------|----------------|
| workflow | 23 | DualWriteStore + OrchestratingEngine (PostgreSQL real-time replication) |
| memory | 11 | Drizzle-first with JSON cache fallback (hybrid mode) |
| semantic-search | 3 | Context pack, fingerprinting, code search |
| policy | 6 | Real Drizzle validation (workflow status, review decisions, branch status, fingerprints) |
| capability | 11 | Real `.claude/` directory reads with frontmatter parsing |
| filesystem | 5 | Real fs.readFileSync / writeFileSync / readdirSync / unlinkSync / statSync |
| review | 2 | Real Drizzle writes to ReviewDecision table |
| session | 3 | Real Drizzle reads/writes to SessionState table |
| local | 4 | File-based `.masday/` state dir + Drizzle sync/push |
| git | 3 | Real `execSync` calls to git CLI |
| npm | 2 | Real `execSync` calls to pnpm CLI |
| docker | 3 | Real `execSync` calls to docker CLI |
| cicd | 3 | Real `execSync` calls to `gh` CLI |
| github | 3 | Real `execSync` calls to `gh` CLI |
| tests | 1 | Real `execSync` calls to pnpm test runner |
| reminder | 3 | Auto-run on startup + periodic 15min check. Detects stale/stuck/failed workflows. session_init_context returns reminders. Drizzle WorkflowReminder table |
| projectRules | 1 | Refactor rules validation (14 checks: naming, patterns, tools, docs, TypeScript, security, imports) |
| use_masday | 1 | Universal entry point — parses any user instruction, returns routing plan (intent, skill, agent, complexity) |

**Persistence:** DualWriteWorkflowStore wraps WorkflowStore and replicates all workflow operations to PostgreSQL in real-time via Drizzle. Memory uses hybrid mode: Drizzle first, JSON cache fallback when PostgreSQL is unavailable. All 16 Drizzle tables are actively populated:

| Table | Wired Via | Trigger |
|-------|-----------|---------|
| Workflow | DualWriteStore | workflow_create, execute, delete |
| Task | DualWriteStore | addTask, startTask, completeTask |
| Plan | DualWriteStore | createPlan |
| Memory | persistToDb() | memory_store, store_research |
| ReviewDecision | Drizzle direct | review_submit |
| SessionState | Drizzle direct | session_patch_state |
| ParallelBranch | Drizzle upsert/update | workflow_createParallelBranches, completeParallelBranch, listParallelBranches |
| ContextDocument | Drizzle direct | memory_store_research |
| TaskProgressLog | saveProgressDb() | workflow_saveProgress |
| RetrievalLog | logRetrieval() | memory_search, semantic-search_code_search, search_hybrid_context_pack |
| TokenUsage | trackTokens() | workflow_saveProgress, memory_store_research |
| EpisodicMemory | registerTool wrapper | All MCP tool calls (captured via monkey-patched server.registerTool) |
| GraphNode | GraphStore.addNode() | memory_store, memory_store_research, workflow_create, workflow_addTask |
| GraphEdge | GraphStore.addEdge() + autoLink() | Jaccard similarity (threshold 0.3) auto-edges + workflow→task contains edges |
| WorkflowReminder | checkReminders() | Startup + 15min interval, deduplicated, covers EXECUTE + FIX states |
| LlmProviderConfig | Drizzle direct | LLM provider configuration storage |

**Status Conventions (ALL UPPERCASE in PostgreSQL):**
- Workflow: INIT, ANALYZE, PLAN, EXECUTE, VERIFY, FIX, DONE, FAILED, PAUSED
- Task: PENDING, RUNNING, DONE, FAILED
- Plan: ACTIVE, PENDING, READY, DONE
- Review: APPROVED, REWORK_REQUIRED, BLOCKED
- DualWriteStore maps in-memory lowercase TaskState to UPPERCASE for Drizzle

## MCP Pattern

Uses official `McpServer` from `@modelcontextprotocol/sdk` with DualWriteStore for PostgreSQL persistence.

### Environment Resolution (Critical)

`import "dotenv/config"` only reads `.env` from `process.cwd()`. When Claude Code launches the MCP server from an arbitrary directory, `DATABASE_URL` is undefined and PostgreSQL features are disabled. Two-layer fix:

**Layer 1 — Explicit dotenv from script location** (`mcp.ts` lines 1-16):
```typescript
import { config as dotenvConfig } from "dotenv";
import * as path from "path";
import * as fs from "fs";
import { fileURLToPath } from "url";
import { createRequire } from "node:module";

const __scriptDir = path.dirname(fileURLToPath(import.meta.url));
const __projectRoot = path.resolve(__scriptDir, "..", "..", "..", ".."); // dist/runtime/ → root

const envPath = path.join(__projectRoot, ".env");
if (fs.existsSync(envPath)) {
  dotenvConfig({ path: envPath });  // explicit path
} else {
  dotenvConfig();                    // fallback to cwd
}
// ALL other imports come AFTER dotenv loads
import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
```

**Layer 2 — MCP config `cwd` + `env`** (`.claude.json`, `.mcp.json`, `.gemini/settings.json`, `.vscode/mcp.json`):
```json
{
  "mcpServers": {
    "masday": {
      "command": "node",
      "args": ["apps/agent-runner/dist/runtime/mcp.js"],
      "cwd": "/absolute/path/to/project-root",
      "env": {
        "DATABASE_URL": "postgresql://...",
        "NODE_ENV": "development"
      }
    }
  }
}
```

Both layers are required: `cwd` + `env` in config ensures `DATABASE_URL` is set even if `.env` is missing; explicit dotenv ensures the server works standalone (`node mcp.js`) from any directory.

### Server Initialization

```typescript
import { db } from "@mcp-rebuild/db";
import { DualWriteWorkflowStore, setDualWriteDb } from "@mcp-rebuild/store";
import { setPrismaClient as setTokenDb, trackTokens } from "@mcp-rebuild/core";
import { saveProgress as saveProgressDb, logRetrieval, setReminderDb, checkReminders } from "@mcp-rebuild/workflow-engine";
import { buildHybridContextPack, computeFingerprint } from "@mcp-rebuild/intelligence";
import { setEpisodicDb, setGraphDb, EpisodicMemory, GraphStore } from "@mcp-rebuild/memory";

const server = new McpServer({ name: "masday", version: "0.1.0" });
const episodicMemory = new EpisodicMemory(100);
const graphStore = new GraphStore({ autoLinkThreshold: 0.3 });
const primaryStore = new WorkflowStore(backend);
const workflowStore = new DualWriteWorkflowStore(primaryStore);

// After Drizzle db connects (3 retries, 8s health check timeout):
setDualWriteDb(db);             // workflow/task/plan replication
setTokenDb(db);                 // token usage tracking
setEpisodicDb(db);              // episodic memory persistence
setGraphDb(db);                 // knowledge graph persistence
setReminderDb(db);              // workflow reminders

server.registerTool("workflow_create", { description: "...", inputSchema: {...} }, async (args) => ({
  content: [{ type: "text", text: JSON.stringify(result) }]
}));

const transport = new StdioServerTransport();
await server.connect(transport);
```

When PostgreSQL is unreachable, `dbReady` flag is `false` and all `if (dbReady)` guards fall back to JSON-only mode. Background reconnect attempts every 15s via `initDb()`.

## Commands

- `pnpm build` - Build all packages (Turbo)
- `pnpm test` - Run tests (Vitest)
- `pnpm db:generate` - Generate Drizzle schema types (drizzle-kit)
- `pnpm db:push` - Push schema to database (drizzle-kit push)
- Start MCP server: `npx tsx apps/agent-runner/src/runtime/mcp.ts`

## Conventions

- TypeScript strict mode
- **ESM modules** (`"type": "module"`, NodeNext resolution)
- All relative imports use `.js` extensions
- Zod for validation
- Pino for logging
- EventBus for pub/sub
- No `any` types -- use `unknown` with Zod validation
- UUID for workflow/task IDs
- Immutable patterns -- spread operators, never mutate
- Functions under 50 lines, files under 400 lines
- Tool handler format: `async (args) => ({content: [{type: "text", text: JSON.stringify(result)}]})`
- Import renaming: `@masday-workflow-reborn/*` and `@cap/*` are both now `@mcp-rebuild/*`
- `listWorkflows` exported as `listWorkflowsDb` from workflow-engine
- Code skills are plain async functions (not class-based Skill objects)
- **All agents and skills enforce review pipeline:** review_submit → policy_validate_completion → workflow_completeTask → local_sync (with rework loop on REWORK_REQUIRED, max 2 attempts)
- **Reminder auto-run:** checkReminders() runs on startup after Drizzle db connects + every 15 minutes via setInterval. session_init_context returns active reminders.

## Step Enforcement Hooks

Two PreToolUse hooks enforce skill/agent step ordering by tracking real evidence (file creation, test execution, MCP tool calls):

**`masday-skill-checkpoint.js`** — Tracks MCP tool call sequences for `masday-workflow-new`:
- Blocks `workflow_execute` if steps 1-6 tools not called (capability_system_readiness, memory_search, memory_recall_recent, semantic-search_code_search, etc.)
- Warns on `workflow_getStatus` if `memory_store` not called (step 9)
- State stored in `os.tmpdir()/masday-skill-checkpoints/session-*.json`

**`skill-step-guard.cjs`** — Validates step transitions for multi-step skills using real evidence:
- **TDD enforcement**: Blocks source code writes before test files during RED phase. Tracks RED → RED_VERIFY → GREEN → GREEN_VERIFY → REFACTOR → COVERAGE transitions.
- **Workflow GATE**: Blocks `workflow_execute` until all pre-execution steps complete. Tracks READINESS → CONTEXT → CREATE → CONTEXT_PACK → AGENT_MATCH → SKILL_VERIFY → EXECUTE → STORE.
- **Planning enforcement**: Blocks `workflow_createPlan` without `semantic-search_code_search` evidence.
- **Research enforcement**: Tracks SEARCH → CODEBASE → STORE with MCP tool evidence.
- State stored in `os.tmpdir()/masday-step-guard/skill-<name>.json` — auto-created, session-scoped.
- Registered in `.claude/settings.json` PreToolUse matcher for Write, Edit, Bash, Skill, and all tracked MCP tools.
- Also registered in `.github/hooks/masday-hooks.json` for VS Code Copilot compatibility.

## Testing

- Vitest with globals enabled
- Integration tests in `tests/integration/`
- Performance benchmarks in `tests/benchmarks/`

## Multi-LLM Setup

```bash
bash scripts/setup.sh
```

Installs to Claude Code, OpenCode, and Codex CLI in one pass.
