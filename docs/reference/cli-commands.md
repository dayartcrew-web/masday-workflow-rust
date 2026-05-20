# CLI Commands Reference

This page consolidates the main commands and workflows referenced in the contributor-facing docs.

## Local project commands

```bash
pnpm install              # Install dependencies
pnpm build                # Build all packages
pnpm test                 # Run all tests (Vitest)
pnpm test:watch           # Watch mode
pnpm lint                 # Lint all packages
pnpm db:pgvector          # Create pgvector columns/indexes (PostgreSQL only)
cd apps/agent-runner && npx tsx src/runtime/mcp.ts    # Start MCP server
```

### Demo commands

```bash
cd apps/agent-runner
pnpm demo:basic          # Basic workflow engine demo
pnpm demo:enhanced       # Enhanced engine with planner demo
pnpm demo:orchestrated   # Multi-agent orchestration demo
pnpm demo:intelligence   # Repository intelligence demo
pnpm demo:production     # Production features demo
```

## Claude / workflow commands

### Workflow management

- `/masday-workflow-init` — Initialize `.masday/` data directory
- `/masday-workflow-new [prompt]` — Create + execute workflow in one shot
- `/masday-workflow-plan [id|prompt]` — Plan tasks for a workflow
- `/masday-workflow-run [id]` — Execute workflow
- `/masday-workflow-status` — Show all workflows
- `/masday-workflow-verify [id]` — Validate workflow results
- `/masday-workflow-fix [id]` — Fix workflow failures (retry logic)
- `/masday-workflow-add-task [id] [agent] [skill] [desc]` — Add task to workflow

### Research

- `/masday-research [topic]` — Research codebase with cached analysis

### Scaffolding

- `/create-agent [name]` — Create a new agent definition
- `/create-skill [name]` — Create a new skill
- `/create-command [name]` — Create a new slash command
- `/create-mcp-skill [name]` — Create a new MCP skill

All scaffolding commands auto-detect the project root via `git rev-parse`.

## Usage guidance

- Prefer the local-first setup path before trying optional Docker-based workflows
- Use the lifecycle terms from [Workflow lifecycle](../workflows/lifecycle.md)
- Treat archived or historical phase docs as background context only

## Related docs

- [Getting started](../getting-started.md)
- [MCP tools](./mcp-tools.md)
- [Workflow lifecycle](../workflows/lifecycle.md)
