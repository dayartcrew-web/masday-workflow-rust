# Getting Started

Masday Workflow is designed to run **locally first**. Docker is an optional profile for isolation or remote-style execution, not the default developer path.

## Default local workflow

```bash
pnpm install
pnpm build
cd apps/agent-runner && npx tsx src/runtime/mcp.ts
```

### Database Setup

The runtime requires PostgreSQL with pgvector for persistent state. DualWriteStore replicates all workflow, task, and memory operations to PostgreSQL in real-time via Drizzle, with JSON cache fallback when the database is unavailable.

```bash
# Start PostgreSQL with pgvector

docker-compose up -d

# Generate Drizzle client
pnpm db:generate

# Push schema to database
pnpm db:push

# Set up pgvector columns and indexes (PostgreSQL only)
pnpm db:pgvector
```

## What this starts

- The local MCP stdio server from `apps/agent-runner`
- **89 MCP tools** across 18 namespaces (workflow, memory, policy, semantic-search, capability, filesystem, review, session, local, git, npm, docker, cicd, github, tests, reminder, projectRules, use_masday)
- Workflow orchestration via OrchestratingEngine with full agent dispatch
- **PostgreSQL-backed runtime state via DualWriteStore** -- all 16 Drizzle tables actively populated (Workflow, Task, Plan, Memory, ReviewDecision, SessionState, ParallelBranch, ContextDocument, TaskProgressLog, RetrievalLog, TokenUsage, EpisodicMemory, GraphNode, GraphEdge, WorkflowReminder, LlmProviderConfig)
- DualWrite pattern: PostgreSQL primary + JSON cache fallback for resilience
- Project-local `.masday/` artifacts for cached research, plans, and notes
- 4 default agent workers: backend, frontend, qa, general-purpose

## Recommended reading order

1. [Architecture](./architecture.md) - actual runtime shape and feature maturity
2. [Workflow lifecycle](./workflows/lifecycle.md) - lifecycle states and execution model
3. [Local development](./workflows/local-development.md) - local-first commands and expectations
4. [MCP tools reference](./reference/mcp-tools.md) - tool surface summary
5. [CLI commands reference](./reference/cli-commands.md) - Claude/CLI command map
6. [State model](./reference/state-model.md) - PostgreSQL vs `.masday` responsibilities

## Runtime profiles

- **Local** - default and primary supported path; uses PostgreSQL via DualWriteStore with JSON cache fallback
- **Docker** - optional profile for isolation or parity testing
- **Remote** - future/advanced profile documented as a planned direction

You can select a profile explicitly:

```bash
export MASDAY_RUNTIME_PROFILE=local   # default
export MASDAY_RUNTIME_PROFILE=docker
export MASDAY_RUNTIME_PROFILE=remote
```

> Note: Only the **local** profile is currently implemented by the runtime in this repository. Selecting `docker` or `remote` will fail fast.

See also:

- [Local development](./workflows/local-development.md)
- [Architecture](./architecture.md)
