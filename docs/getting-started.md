# Getting Started

Masday Workflow is designed to run **locally first**. Docker is an optional profile for isolation or remote-style execution, not the default developer path.

## Quick Start (Rust CLI)

```bash
# From project root — build the CLI binary
cargo build --release -p masday-cli

# Install masday into current project (local mode)
./target/release/masday install

# Or connect to a remote API server
./target/release/masday install --remote https://api.example.com:3010 --api-key <key>
```

See [CLI commands reference](./reference/cli-commands.md) for full command documentation.

### What `masday install` distributes

The `masday` binary is **self-contained** (7.6MB). It embeds all templates at compile time:

| What user gets | What user does NOT get |
|----------------|----------------------|
| `masday` binary (with embedded templates) | Root project source code |
| 28 agent .md files (extracted from binary) | Cargo workspace / Rust source |
| 30+ skill directories (extracted from binary) | pnpm monorepo / TypeScript source |
| Hooks (global + project, extracted from binary) | PostgreSQL schema (remote mode) |
| MCP configs (generated per platform) | Dashboard frontend |
| settings.json updates (statusline, autoCompact) | |

**Local mode** requires Rust toolchain (builds from source).
**Remote mode** only needs the binary — no Rust, no Node.js, no source code.

## Default local workflow (Developer)

```bash
cargo build --workspace
cargo run -p masday-api    # Start API server (port 3010)
cargo run -p masday-mcp    # Start MCP stdio server
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
