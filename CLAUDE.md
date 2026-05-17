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
 | (Dashboard/  |   stdio       |  (56+ tools)      |
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

## Packages (12)

| Package | Description |
|---------|-------------|
| `packages/core` | Shared types, logger, EventBus, tracing, metrics |
| `packages/shared-utils` | Logger, IDs, hash, env (from msd-mcp) |
| `packages/db` | Prisma schema (14 models + pgvector), client singleton |
| `packages/store` | StorageBackend, SQLite, JSON, Prisma adapters |
| `packages/llm` | Multi-provider LLM (Anthropic, OpenAI, Custom), circuit breaker |
| `packages/memory` | 4-layer memory (working, episodic, long-term, graph), scoring, BM25 |
| `packages/workflow-engine` | Pure functions + state machine, DAG, session, review, parallel, drift |
| `packages/intelligence` | SemanticSearcher, CodeIndexer, ReAct agent, Guardrails |
| `packages/policy` | PolicyValidator, WorkflowAuditor, drift detection |
| `packages/capability` | Registry, Scaffolder, SystemHealth |
| `packages/code-skills` | Git, tests, npm, code, docker, github, CI/CD (plain functions + Zod) |
| `packages/cli` | CLI entry point + setup templates |

## MCP Server Apps (6)

| App | Tools | Description |
|-----|-------|-------------|
| `apps/workflow-orchestrator-mcp` | 26 | Workflow CRUD, plans, tasks, sessions, reviews, parallel, progress |
| `apps/memory-mcp` | 9 | Memory CRUD, research storage, recall, search |
| `apps/semantic-search-mcp` | 2 | Hybrid context pack, fingerprinting |
| `apps/policy-mcp` | 6 | Session readiness, execution/completion validation, drift detection |
| `apps/capability-mcp` | 10 | Agent/skill/command registry, scaffolding, health check |
| `apps/unified-mcp` | 70 | All tools in one server + code skills |

## MCP Pattern

Uses official `McpServer` from `@modelcontextprotocol/sdk`:

```typescript
import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { StdioServerTransport } from "@modelcontextprotocol/sdk/server/stdio.js";

const server = new McpServer({ name: "my-server", version: "1.0.0" });

server.tool("tool_name", "Description", { param: z.string() }, async (args) => ({
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
- Start individual MCP servers: `npx tsx apps/<app>/src/index.ts`

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
