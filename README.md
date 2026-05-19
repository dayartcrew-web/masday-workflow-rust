# masday-workflow-rebuild

[![TypeScript](https://img.shields.io/badge/TypeScript-5.6-blue?logo=typescript&logoColor=white)](https://www.typescriptlang.org/)
[![pnpm](https://img.shields.io/badge/pnpm-9.0-F69220?logo=pnpm&logoColor=white)](https://pnpm.io/)
[![MCP](https://img.shields.io/badge/MCP-1.29-green?logo=data:image/svg+xml;base64,PHN2ZyB4bWxucz0iaHR0cDovL3d3dy53My5vcmcvMjAwMC9zdmciIHdpZHRoPSIyNCIgaGVpZ2h0PSIyNCIgdmlld0JveD0iMCAwIDI0IDI0IiBmaWxsPSJub25lIiBzdHJva2U9IndoaXRlIiBzdHJva2Utd2lkdGg9IjIiPjxwYXRoIGQ9Ik0xMiAydjIwTTIgMTJoMjAiLz48L3N2Zz4=)](https://modelcontextprotocol.io/)
[![PostgreSQL](https://img.shields.io/badge/PostgreSQL-16-4169E1?logo=postgresql&logoColor=white)](https://www.postgresql.org/)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

Unified AI coding agent platform built on Model Context Protocol (MCP).

Merges the best of two projects:
- **msd-mcp** -- Official MCP SDK, 5 domain servers, Prisma/PostgreSQL persistence
- **masday-workflow-reborn** -- 4-layer memory, 3-tier workflow engine, code skills, agent dispatch

The result is a modular monorepo of 12 packages and a single unified MCP server exposing 83 tools over stdio to any MCP-compatible client.

---

## Quick Start

```bash
# Install dependencies
pnpm install

# Generate Prisma client
pnpm db:generate

# Build all packages (Turbo)
pnpm build

# Start the unified MCP server (all 83 tools)
npx tsx apps/agent-runner/src/runtime/mcp.ts
```

### Docker (PostgreSQL + pgvector)

```bash
docker-compose up -d
```

This starts a PostgreSQL 16 instance with pgvector on port 5432. See [Configuration](#configuration) for connection details.

---

## Architecture

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

## Packages (12)

| Package | Scope | Description |
|---------|-------|-------------|
| `packages/core` | `@mcp-rebuild/core` | Shared types, logger, EventBus, tracing, metrics |
| `packages/shared-utils` | `@mcp-rebuild/shared-utils` | Logger, IDs, hash, env utilities (from msd-mcp) |
| `packages/db` | `@mcp-rebuild/db` | Prisma schema (14 models + pgvector), client singleton |
| `packages/store` | `@mcp-rebuild/store` | StorageBackend, SQLite, JSON, Prisma adapters |
| `packages/llm` | `@mcp-rebuild/llm` | Multi-provider LLM (Anthropic, OpenAI, Custom), circuit breaker |
| `packages/memory` | `@mcp-rebuild/memory` | 4-layer memory (working, episodic, long-term, graph), scoring, BM25 |
| `packages/workflow-engine` | `@mcp-rebuild/workflow-engine` | Pure functions + state machine, DAG, session, review, parallel, drift |
| `packages/intelligence` | `@mcp-rebuild/intelligence` | SemanticSearcher, CodeIndexer, ReAct agent, Guardrails |
| `packages/policy` | `@mcp-rebuild/policy` | PolicyValidator, WorkflowAuditor, drift detection |
| `packages/capability` | `@mcp-rebuild/capability` | Registry, Scaffolder, SystemHealth |
| `packages/code-skills` | `@mcp-rebuild/code-skills` | Git, tests, npm, code, docker, github, CI/CD (plain functions + Zod) |
| `packages/cli` | `@mcp-rebuild/cli` | CLI entry point + setup templates |

---

## MCP Server

| App | Tools | Description |
|-----|-------|-------------|
| `apps/agent-runner` | 83 | Unified MCP server, all namespaces, DualWriteStore + PostgreSQL |

### Tool Namespaces (83 tools)

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

### MCP Pattern

Uses official `McpServer` from `@modelcontextprotocol/sdk` with DualWriteStore for PostgreSQL persistence:

```typescript
import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { StdioServerTransport } from "@modelcontextprotocol/sdk/server/stdio.js";
import { DualWriteWorkflowStore, setDualWritePrisma } from "@mcp-rebuild/store";
import { setPrismaClient as setTokenPrisma, trackTokens } from "@mcp-rebuild/core";
import { setEpisodicPrisma, setGraphPrisma } from "@mcp-rebuild/memory";
import { saveProgress as saveProgressDb, logRetrieval } from "@mcp-rebuild/workflow-engine";

const server = new McpServer({ name: "masday", version: "0.1.0" });
// After Prisma connects:
setDualWritePrisma(prisma);
setTokenPrisma(prisma);
setEpisodicPrisma(prisma);
setGraphPrisma(prisma);
```

### Persistence

All 14 Prisma tables are actively populated via DualWriteStore pattern:

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

Status values are ALL UPPERCASE in PostgreSQL:
- Workflow: INIT, ANALYZE, PLAN, EXECUTE, VERIFY, FIX, DONE, FAILED, PAUSED
- Task: PENDING, RUNNING, DONE, FAILED
- Plan: ACTIVE, PENDING, READY, DONE
- Review: APPROVED, REWORK_REQUIRED, BLOCKED

### Starting the Server

```bash
# Unified MCP server (all 83 tools)
npx tsx apps/agent-runner/src/runtime/mcp.ts
```

---

## Platform Support

| Platform | Agents | Skills | MCP Config |
|----------|--------|--------|------------|
| **Claude Code** | `.claude/agents/*.md` | `.claude/skills/*/SKILL.md` | `.claude/settings.json` |
| **Codex CLI** | `.agents/agents/*.toml` | `.agents/skills/*/SKILL.md` | `.codex/config.toml` |
| **Gemini CLI** | `.gemini/agents/` | `.gemini/skills/` | `.gemini/settings.json` |
| **Continue** | `.continue/agents/` | `.continue/skills/` | `.continue/config.json` |
| **GitHub Copilot** | N/A | N/A | `.github/copilot.yml` |

Run the setup script to install to all platforms:

```bash
bash scripts/setup.sh
```

---

## Commands Reference

| Command | Description |
|---------|-------------|
| `pnpm install` | Install all dependencies |
| `pnpm build` | Build all packages (Turbo, cached) |
| `pnpm dev` | Start all packages in dev mode (parallel) |
| `pnpm test` | Run tests (Vitest) |
| `pnpm test:watch` | Run tests in watch mode |
| `pnpm test:coverage` | Run tests with coverage report |
| `pnpm lint` | Run type checking across all packages |
| `pnpm typecheck` | Alias for lint |
| `pnpm db:generate` | Generate Prisma client from schema |
| `pnpm db:push` | Push schema to database (no migration) |
| `pnpm db:migrate` | Create and apply Prisma migration |
| `docker-compose up -d` | Start PostgreSQL + pgvector |

---

## Configuration

### Environment Variables

Create a `.env` file in the project root:

```env
# Database
DATABASE_URL="postgresql://postgres:postgres@localhost:5432/masday_workflow?schema=public"

# LLM Providers (optional, per-provider)
ANTHROPIC_API_KEY="sk-ant-..."
OPENAI_API_KEY="sk-..."
```

### Database

The project uses Prisma with PostgreSQL and pgvector. The schema is at `packages/db/prisma/schema.prisma` and includes 14 models with pgvector support for semantic search.

```bash
# Start the database
docker-compose up -d

# Generate the Prisma client
pnpm db:generate

# Push schema (development)
pnpm db:push

# Or run migrations
pnpm db:migrate
```

---

## Tech Stack

| Layer | Technology |
|-------|-----------|
| Language | TypeScript (strict mode, ESM modules) |
| Runtime | Node.js with `tsx` for TypeScript execution |
| Database | PostgreSQL 16 + pgvector (via Prisma ORM) |
| Protocol | Model Context Protocol (MCP) over stdio |
| Validation | Zod schemas for all inputs |
| Logging | Pino structured logging |
| Build | Turborepo with pnpm workspaces |
| Testing | Vitest with globals enabled |

---

## Contributing

1. Fork the repository
2. Create a feature branch (`git checkout -b feat/my-feature`)
3. Follow project conventions:
   - TypeScript strict mode
   - ESM modules with `.js` import extensions
   - Zod for all validation
   - Functions under 50 lines, files under 400 lines
   - No `any` types
4. Write tests (Vitest, 80%+ coverage)
5. Run `pnpm build && pnpm test`
6. Submit a pull request

### Conventions

- **Module system**: ESM (`"type": "module"`, NodeNext resolution)
- **Imports**: All relative imports use `.js` extensions
- **Validation**: Zod schemas for all inputs
- **Logging**: Pino structured logging
- **Events**: EventBus for pub/sub
- **IDs**: UUID for workflow/task IDs
- **Data**: Immutable patterns (spread operators, never mutate)
- **Tools**: Handler format `async (args) => ({content: [{type: "text", text: JSON.stringify(result)}]})`
- **Code skills**: Plain async functions (not class-based)
- **Naming**: Tool names use camelCase dot-namespaced format: `workflow.getActive`, `memory.store`
- **MCP SDK**: Resolves dots to underscores: `mcp__masday__workflow_getActive`
- **Status**: ALL UPPERCASE in PostgreSQL
- **Package scope**: All packages use `@mcp-rebuild/*`

---

## License

[MIT](https://opensource.org/licenses/MIT)
