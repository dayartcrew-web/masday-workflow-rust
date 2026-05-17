# State Model

This page captures the runtime state ownership model and the workflow engine architecture.

## Ownership model

- **SQLite** (or other runtime persistence) owns operational workflow/task state.
- **`.masday/`** owns project-local, human-readable artifacts such as research notes, plans, context summaries, and execution notes.

## `.masday/` contents

| Directory    | Contents                                      |
| ------------ | --------------------------------------------- |
| `research/`  | Cached codebase analysis (saves 60-80% tokens) |
| `context/`   | Project summary and cached context             |
| `plans/`     | Task plans and exported summaries              |
| `notes/`     | Execution notes and workflow summaries         |
| `state/`     | Per-workflow state snapshots (JSON)            |

## Runtime persistence

The SQLite store manages:

| Store            | Contents                                          |
| ---------------- | ------------------------------------------------- |
| WorkflowStore    | Active workflows, state transitions               |
| TaskResultStore  | Task status, execution metadata, outputs           |
| ConfigStore      | Runtime configuration                             |

Database location defaults to `./data/masday-workflow.db` (overridable via `MASDAY_DB_PATH`).

## Engine hierarchy

All workflow engines share a `BaseWorkflowEngine` base class providing:

- Workflow storage (`Map<string, Workflow>`)
- UUID-based ID generation
- `getStatus`, `addTask`, `getWorkflow`, `listWorkflows`

```
BaseWorkflowEngine
├── WorkflowEngine          # Basic: linear state machine, sequential execution
├── EnhancedWorkflowEngine  # Enhanced: + Planner, DAGExecutor, VERIFY, FIX retry
└── OrchestratingEngine     # Full: + AgentCoordinator, SkillRouter, TaskQueue
```

The MCP server instantiates `OrchestratingEngine` with `coordinator: true` and `enableSkillRouting: true`.

## Workflow state transitions

```
INIT → ANALYZE → PLAN → EXECUTE → VERIFY → DONE
  │                 │    │      │          │
  └──→ DONE         │    │      └──→ FIX ──┤
  └──→ FAILED       │    └──→ PAUSED       └──→ FIX → EXECUTE
                    └──→ FAILED    │
                                   └──→ FAILED
                                      FIX → DONE
                                      FIX → FAILED
```

- **VERIFY** — Checks for failed tasks. If all passed, transitions to DONE. If failures found, transitions to FIX.
- **FIX** — Resets failed tasks to pending, re-executes up to `maxFixRetries`. Emits `workflow.fixing` event. Can also transition directly to DONE or FAILED.
- **PAUSED** — Execution suspended (e.g., waiting on external input). Can resume back to EXECUTE.
- **FAILED** — Unrecoverable error. Any state can transition to FAILED.
- **StateMachine** emits `workflow.state.transition` events on every state change.

## Task execution features

| Feature           | Description                                                       |
| ----------------- | ----------------------------------------------------------------- |
| DAG execution     | Tasks with dependency ordering, parallel where possible            |
| Timeout           | AbortController-based (default 5 min), configurable via `taskTimeout` |
| Output piping     | Dependent tasks receive `dependencyOutputs` from prerequisites     |
| Auto-task creation| `createPlan` auto-creates tasks from `plan.tasks[]` entries        |
| Priority queue    | OrchestratingEngine uses TaskQueue with agent-type priorities      |
| Agent dispatch    | Tasks routed through SkillRouter to appropriate agent workers      |
| Memory persistence| Memory store persists to file after each add (calls `save()`)      |
| Startup init      | Session, Review, and Parallel tables are initialized at startup    |

## Why this matters

The project mixes documented concepts from several phases and experiments. Keeping the ownership model explicit prevents docs from implying that every state artifact serves the same purpose.

## Related docs

- [Architecture](../architecture.md)
- [Getting started](../getting-started.md)
- [MCP tools](./mcp-tools.md)
