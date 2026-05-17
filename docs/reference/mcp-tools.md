# MCP Tools Reference

This page is the canonical contributor-facing reference for the documented MCP tool surface.

## Workflow tools

These are the workflow-oriented tools described across the project docs:

- `workflow.create`
- `workflow.execute`
- `workflow.getStatus`
- `workflow.addTask`
- `workflow.list`
- `workflow.get`
- `workflow.delete`

## Supporting skill/tool families referenced in docs

- `filesystem.*` — read, write, list, delete, stat
- `git.*` — status, log, diff, branch, commit
- `tests.*` — run, results
- `npm.*` — install, run, list
- `code.*` — analyze, generate, transform, search
- `docker.*` — build, run, ps
- `github.*` — PR management, issues
- `cicd.*` — pipeline status, trigger, view

## Workflow lifecycle behavior

The MCP server runs on `OrchestratingEngine` with full agent dispatch enabled:

- **VERIFY** phase checks for failed tasks before transitioning to DONE
- **FIX** phase resets failed tasks to pending and retries execution (configurable `maxFixRetries`)
- **Task output piping** — dependent tasks receive `dependencyOutputs` from completed prerequisites
- **Agent routing** — tasks dispatched through SkillRouter with 3-tier fallback to appropriate agent worker

## Event types

| Event                       | Payload                    |
| --------------------------- | -------------------------- |
| `workflow.started`          | `{ workflow }`             |
| `workflow.completed`        | `{ workflow }`             |
| `workflow.failed`           | `{ workflow, error }`      |
| `workflow.fixing`           | `{ workflow, retryCount }` |
| `workflow.state.transition` | `{ from, to, workflowId }` |
| `task.started`              | `{ task }`                 |
| `task.completed`            | `{ task }`                 |
| `task.failed`               | `{ task, error }`          |

## Important status note

This page is intentionally conservative: it should describe the tool surface contributors can rely on in the current repo narrative. If a package implements more capabilities than the runtime currently exposes, the docs should label that explicitly instead of presenting them as universally active.

## Related docs

- [CLI commands](./cli-commands.md)
- [State model](./state-model.md)
- [Architecture](../architecture.md)
