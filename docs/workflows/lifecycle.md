# Workflow Lifecycle

This document is the canonical source of truth for the Masday Workflow lifecycle.

## Canonical lifecycle

```text
INIT ──> ANALYZE ──> PLAN ──> EXECUTE ──> VERIFY ──> DONE
  │                    │    │      │          │
  └──> DONE            │    │      └──> FIX ──┤
  └──> FAILED          │    └──> PAUSED       └──> FIX ──> EXECUTE
                       └──> FAILED    │
                                      └──> FAILED
                                         FIX ──> DONE
                                         FIX ──> FAILED
```

## State meanings

| State     | Purpose                                                   |
| --------- | --------------------------------------------------------- |
| `INIT`    | Workflow record exists and is ready for analysis/planning |
| `ANALYZE` | Requirements and codebase context are gathered            |
| `PLAN`    | Tasks, dependencies, and execution shape are defined      |
| `EXECUTE` | Tasks run through the active execution engine             |
| `VERIFY`  | Outputs are checked against expectations (failed task check) |
| `FIX`     | Failures are corrected and retried (configurable)         |
| `DONE`    | Workflow has completed successfully                       |
| `PAUSED`  | Execution suspended, can resume back to EXECUTE           |
| `FAILED`  | Unrecoverable error (reachable from any state)            |

## VERIFY phase

The VERIFY phase is implemented in `EnhancedWorkflowEngine` and `OrchestratingEngine`. It checks whether any tasks failed during execution:

- **All tasks passed** — transitions to DONE
- **Failed tasks found** — transitions to FIX

## FIX phase

The FIX phase implements configurable retry logic:

1. Engine transitions to `FIX` state
2. Failed tasks are reset to PENDING status
3. Tasks are re-executed through the DAG executor
4. `workflow.fixing` event is emitted with `{ workflow, retryCount }`
5. If all retries pass VERIFY, workflow transitions to DONE
6. If `maxFixRetries` is exhausted, the workflow fails

The `maxFixRetries` parameter is configurable in the engine config. Default varies by engine.

## Engine tiers

| Engine                  | Features                                                    |
| ----------------------- | ----------------------------------------------------------- |
| WorkflowEngine          | Linear execution, sequential tasks                          |
| EnhancedWorkflowEngine  | + Planner, DAGExecutor, retry, VERIFY, FIX                  |
| OrchestratingEngine     | + AgentCoordinator, SkillRouter, TaskQueue, agent dispatch  |

All engines inherit from `BaseWorkflowEngine` which provides shared workflow storage, UUID generation, and common operations.

## Events emitted during lifecycle

| Event                       | When emitted                      |
| --------------------------- | --------------------------------- |
| `workflow.started`          | Workflow enters EXECUTE           |
| `workflow.state.transition` | Every state machine transition    |
| `workflow.fixing`           | Entering FIX state for retry      |
| `workflow.completed`        | Workflow reaches DONE             |
| `workflow.failed`           | Workflow fails (retries exhausted)|
| `task.started`              | Individual task begins            |
| `task.completed`            | Individual task succeeds          |
| `task.failed`               | Individual task fails             |

## Additional runtime behaviors

- **Auto-task creation**: `createPlan` auto-creates tasks from `plan.tasks[]` entries
- **Memory persistence**: Memory store persists to file after each add (calls `save()`)
- **Startup initialization**: Session, Review, and Parallel tables are initialized at startup

## What this document covers

- Lifecycle vocabulary used across docs and commands
- The implemented state-machine model
- How contributors should talk about workflow execution in this repository

## Related docs

- [Getting started](../getting-started.md)
- [Local development](./local-development.md)
- [CLI commands](../reference/cli-commands.md)
- [Architecture](../architecture.md)
- [State model](../reference/state-model.md)
