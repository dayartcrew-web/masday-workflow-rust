# CLAUDE.md - masday-workflow-rebuild

## Project Overview

Unified AI coding agent platform built on Model Context Protocol (MCP).
Merges msd-mcp (official MCP SDK, 5 domain servers, Prisma/PostgreSQL) with masday-workflow-reborn (4-layer memory, 3-tier workflow engine, code skills).
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
 | (Dashboard/  |   stdio       |  (83 tools)       |
 |  CLI/MCP)    |               +--------+----------+
 +-------------+                         |
                                         v
                             +-----------------------+
                             |    WORKFLOW ENGINE     |
                             |                       |
                             |  Pure functions (msd) |
                             |  + State machine (reborn) |
                             |  + Session/Review/Parallel |
                             +-----------+-----------+
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
  |              In-process RAM, per session                  |
  +----------------------------------------------------------+
                            |
  +----------------------------------------------------------+
  |                  EPISODIC MEMORY                         |
  |            Last N messages per session                    |
  +----------------------------------------------------------+
                            |
  +----------------------------------------------------------+
  |                 LONG-TERM MEMORY                         |
  |   Scoring: similarity*0.6 + recency*0.15                |
  |            + importance*0.15 + usage*0.1                 |
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
| `packages/db` | Prisma schema (15 models + pgvector), client singleton |
| `packages/store` | StorageBackend, SQLite, JSON, Prisma adapters |
| `packages/llm` | Multi-provider LLM (Anthropic, OpenAI, Custom), circuit breaker |
| `packages/memory` | 4-layer memory (working, episodic, long-term, graph), scoring, BM25 |
| `packages/workflow-engine` | Pure functions + state machine, DAG, session, review, parallel, drift |
| `packages/intelligence` | SemanticSearcher, CodeIndexer, ReAct agent, Guardrails |
| `packages/policy` | PolicyValidator, WorkflowAuditor, drift detection |
| `packages/capability` | Registry, Scaffolder, SystemHealth |
| `packages/code-skills` | Git, tests, npm, code, docker, github, CI/CD (plain functions + Zod) |
| `packages/project-rules` | Refactor rules engine, 37 automated checks, checklist validator |
| `packages/cli` | CLI entry point + setup templates |

## MCP Server — `apps/agent-runner` (87 tools)

Single unified MCP server at `apps/agent-runner/src/runtime/mcp.ts`. All 87 tools are real implementations connected to PostgreSQL via DualWriteStore.

| Namespace | Tools | Implementation |
|-----------|-------|----------------|
| workflow | 23 | DualWriteStore + OrchestratingEngine (PostgreSQL real-time replication) |
| memory | 11 | Prisma-first with JSON cache fallback (hybrid mode) |
| semantic-search | 3 | Context pack, fingerprinting, code search |
| policy | 6 | Real Prisma validation (workflow status, review decisions, branch status, fingerprints) |
| capability | 11 | Real `.claude/` directory reads with frontmatter parsing |
| filesystem | 5 | Real fs.readFileSync / writeFileSync / readdirSync / unlinkSync / statSync |
| review | 2 | Real Prisma writes to ReviewDecision table |
| session | 3 | Real Prisma reads/writes to SessionState table |
| local | 4 | File-based `.masday/` state dir + Prisma sync/push |
| git | 3 | Real `execSync` calls to git CLI |
| npm | 2 | Real `execSync` calls to pnpm CLI |
| docker | 3 | Real `execSync` calls to docker CLI |
| cicd | 3 | Real `execSync` calls to `gh` CLI |
| github | 3 | Real `execSync` calls to `gh` CLI |
| tests | 1 | Real `execSync` calls to pnpm test runner |
| reminder | 3 | Stale/stuck workflow detection, reminder listing, acknowledgment (Prisma WorkflowReminder table) |
| projectRules | 1 | Refactor rules validation (37 checks: naming, patterns, tools, docs, TypeScript, security, imports) |

**Persistence:** DualWriteWorkflowStore wraps WorkflowStore and replicates all workflow operations to PostgreSQL in real-time via Prisma. Memory uses hybrid mode: Prisma first, JSON cache fallback when PostgreSQL is unavailable. All 15 Prisma models are actively populated:

| Table | Wired Via | Trigger |
|-------|-----------|---------|
| Workflow | DualWriteStore | workflow.create, execute, delete |
| Task | DualWriteStore | addTask, startTask, completeTask |
| Plan | DualWriteStore | createPlan |
| Memory | persistToPrisma() | memory.store, store_research |
| ReviewDecision | Prisma direct | review.submit |
| SessionState | Prisma direct | session.patch_state |
| ParallelBranch | Prisma direct | workflow.createParallelBranches |
| ContextDocument | Prisma direct | memory.store_research |
| TaskProgressLog | saveProgressDb() | workflow.saveProgress |
| RetrievalLog | logRetrieval() | memory.search, semantic-search.code_search, search_hybrid_context_pack |
| TokenUsage | trackTokens() | workflow.saveProgress, memory.store_research |
| EpisodicMemory | setEpisodicPrisma() | EpisodicMemory.add() |
| GraphNode | setGraphPrisma() | GraphStore.addNode() |
| GraphEdge | setGraphPrisma() | GraphStore.addEdge() |
| WorkflowReminder | setReminderPrisma() | reminder.check |

**Status Conventions (ALL UPPERCASE in PostgreSQL):**
- Workflow: INIT, ANALYZE, PLAN, EXECUTE, VERIFY, FIX, DONE, FAILED, PAUSED
- Task: PENDING, RUNNING, DONE, FAILED
- Plan: ACTIVE, PENDING, READY, DONE
- Review: APPROVED, REWORK_REQUIRED, BLOCKED
- DualWriteStore maps in-memory lowercase TaskState to UPPERCASE for Prisma

## MCP Pattern

Uses official `McpServer` from `@modelcontextprotocol/sdk` with DualWriteStore for PostgreSQL persistence:

```typescript
import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { StdioServerTransport } from "@modelcontextprotocol/sdk/server/stdio.js";
import { DualWriteWorkflowStore, setDualWritePrisma } from "@mcp-rebuild/store";
import { setPrismaClient as setTokenPrisma, trackTokens } from "@mcp-rebuild/core";
import { saveProgress as saveProgressDb, logRetrieval } from "@mcp-rebuild/workflow-engine";

const server = new McpServer({ name: "masday", version: "0.1.0" });
const primaryStore = new WorkflowStore(backend);
const workflowStore = new DualWriteWorkflowStore(primaryStore);

// After Prisma connects:
setDualWritePrisma(prisma);      // workflow/task/plan replication
setTokenPrisma(prisma);          // token usage tracking

server.registerTool("workflow.create", { description: "...", inputSchema: {...} }, async (args) => ({
  content: [{ type: "text", text: JSON.stringify(result) }]
}));

const transport = new StdioServerTransport();
await server.connect(transport);
```

## Commands

- `pnpm build` - Build all packages (Turbo)
- `pnpm test` - Run tests (Vitest)
- `pnpm db:generate` - Generate Prisma client
- `pnpm db:push` - Push schema to database
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

## Testing

- Vitest with globals enabled
- Integration tests in `tests/integration/`
- Performance benchmarks in `tests/benchmarks/`

## Multi-LLM Setup

```bash
bash scripts/setup.sh
```

Installs to Claude Code, OpenCode, and Codex CLI in one pass.
