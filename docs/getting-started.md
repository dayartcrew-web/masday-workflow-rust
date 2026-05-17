# Getting Started

Masday Workflow is designed to run **locally first**. Docker is an optional profile for isolation or remote-style execution, not the default developer path.

## Default local workflow

```bash
pnpm install
pnpm build
cd apps/agent-runner && pnpm start:mcp
```

## What this starts

- The local MCP stdio server from `apps/agent-runner`
- Workflow orchestration via OrchestratingEngine with full agent dispatch
- SQLite-backed runtime state for workflows, tasks, and config
- Project-local `.masday/` artifacts for cached research, plans, and notes
- 4 default agent workers: backend, frontend, qa, general-purpose

## Recommended reading order

1. [Architecture](./architecture.md) - actual runtime shape and feature maturity
2. [Workflow lifecycle](./workflows/lifecycle.md) - lifecycle states and execution model
3. [Local development](./workflows/local-development.md) - local-first commands and expectations
4. [MCP tools reference](./reference/mcp-tools.md) - tool surface summary
5. [CLI commands reference](./reference/cli-commands.md) - Claude/CLI command map
6. [State model](./reference/state-model.md) - SQLite vs `.masday` responsibilities

## Runtime profiles

- **Local** - default and primary supported path
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
