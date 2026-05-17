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

The result is a modular monorepo of 12 packages and 6 MCP server apps, exposing 70+ tools over stdio to any MCP-compatible client.

---

## Quick Start

```bash
# Install dependencies
pnpm install

# Generate Prisma client
pnpm db:generate

# Build all packages (Turbo)
pnpm build

# Start the unified MCP server (all 70 tools)
npx tsx apps/unified-mcp/src/index.ts
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
 | (Dashboard/  |   stdio       |  (70+ tools)      |
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

## MCP Server Apps (6)

| App | Tools | Description |
|-----|-------|-------------|
| `apps/unified-mcp` | 70 | All tools in one server + code skills |
| `apps/workflow-orchestrator-mcp` | 26 | Workflow CRUD, plans, tasks, sessions, reviews, parallel, progress |
| `apps/memory-mcp` | 9 | Memory CRUD, research storage, recall, search |
| `apps/semantic-search-mcp` | 2 | Hybrid context pack, fingerprinting |
| `apps/policy-mcp` | 6 | Session readiness, execution/completion validation, drift detection |
| `apps/capability-mcp` | 10 | Agent/skill/command registry, scaffolding, health check |

### MCP Pattern

All servers use the official `McpServer` from `@modelcontextprotocol/sdk`:

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

### Starting Individual Servers

```bash
# Unified (all tools)
npx tsx apps/unified-mcp/src/index.ts

# Domain servers
npx tsx apps/workflow-orchestrator-mcp/src/index.ts
npx tsx apps/memory-mcp/src/index.ts
npx tsx apps/semantic-search-mcp/src/index.ts
npx tsx apps/policy-mcp/src/index.ts
npx tsx apps/capability-mcp/src/index.ts
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

---

## License

[MIT](https://opensource.org/licenses/MIT)
