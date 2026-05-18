<!-- mcp-rebuild-session-context -->
# masday-workflow-rebuild Active Session

This project uses **masday-workflow-rebuild** for workflow management.

## Commands

| Command | Purpose |
|---------|---------|
| "/msd-start-work" | Start new or load existing workflow |
| "/msd-continue" | Resume active workflow |
| "/msd-implement" | Execute current task with scope discipline |
| "/msd-plan" | Create or refine implementation plan |
| "/msd-review" | Quality gate against acceptance criteria |
| "/msd-verify" | Pre-completion verification |
| "/msd-status" | Read-only workflow diagnostic |
| "/msd-debug" | Deep diagnosis + auto-fix |
| "/msd-health" | Quick infrastructure check |

## Workflow Pattern

1. "workflow.get_active" -> "workflow.get_plan" -> "workflow.get_current_task"
2. "search.hybrid_context_pack" to build task context
3. Execute / Research / Review
4. "workflow.save_progress" -> "review.submit" -> "workflow.complete_task"

## Key MCP Namespaces

- "mcp__masday__*" — Unified MCP server (83 tools): workflow.*, memory.*, search.*, policy.*, capability.*, local.*, review.*, session.*, filesystem.*, git.*, npm.*, docker.*, cicd.*, github.*, tests.*

## Package Scope

All packages use "@mcp-rebuild/*" scope.

## Build

```bash
pnpm install
pnpm db:generate
pnpm build
```

<!-- mcp-rebuild-session-context -->


<!-- msd-mcp-session-context -->
# msd-mcp Active Session

This project uses **msd-mcp** (Multi-Agent MCP Runtime) for workflow management.

For ALL tasks, use msd-mcp commands and MCP tools.

## Commands

| Command | Purpose |
|---------|---------|
| `/msd-start-work` | Start new or load existing workflow |
| `/msd-continue` | Resume active workflow |
| `/msd-implement` | Execute current task with scope discipline |
| `/msd-plan` | Create or refine implementation plan |
| `/msd-review` | Quality gate against acceptance criteria |
| `/msd-verify` | Pre-completion verification |
| `/msd-status` | Read-only workflow diagnostic |
| `/msd-doctor` | Deep diagnosis + auto-fix |
| `/msd-health` | Quick infrastructure check |
| `/msd-autopilot` | Auto-execute all tasks |

## Workflow Pattern

1. `workflow.get_active` → `workflow.get_plan` → `workflow.get_current_task`
2. `search.hybrid_context_pack` to build task context
3. Execute / Research / Review
4. `workflow.save_progress` → `review.submit` → `workflow.complete_task`

## Key MCP Namespaces

- `mcp__masday__workflow_*` — Workflow, plan, task, session, parallel (26 tools)
- `mcp__masday__review_*` — Review submit, get latest
- `mcp__masday__local_*` — Init, sync, push, save_artifact
- `mcp__masday__session_*` — Init context, get/patch state
- `mcp__masday__memory_*` — Store, recall, research memory (9 tools)
- `mcp__masday__search_*` — Context packs, fingerprints (2 tools)
- `mcp__masday__policy_*` — Validation, drift, readiness (6 tools)
- `mcp__masday__capability_*` — Health, factory, scaffold (10 tools)

<!-- msd-mcp-session-context -->
