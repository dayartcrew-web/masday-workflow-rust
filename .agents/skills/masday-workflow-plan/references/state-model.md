# Workflow State Machine Reference

## States
```
INIT ──> ANALYZE ──> PLAN ──> EXECUTE ──> VERIFY ──> DONE
  │                    │    │      │          │
  └──> DONE            │    │      └──> FIX ──┤
  └──> FAILED          │    └──> PAUSED       └──> FIX ──> EXECUTE
                       └──> FAILED    │
                                      └──> FAILED
                                         FIX ──> DONE
                                         FIX ──> FAILED
```

## State Transitions

| From | To | Condition |
|------|----|-----------|
| INIT | ANALYZE | Workflow created, starting analysis |
| INIT | DONE | Workflow completed immediately |
| INIT | FAILED | Unrecoverable error during init |
| ANALYZE | PLAN | Analysis complete, requirements gathered |
| ANALYZE | FAILED | Unrecoverable error during analysis |
| PLAN | EXECUTE | Plan created with tasks |
| PLAN | FAILED | Unrecoverable error during planning |
| EXECUTE | VERIFY | All tasks completed |
| EXECUTE | FIX | Task failed (up to maxFixRetries) |
| EXECUTE | PAUSED | Execution suspended (e.g., waiting on external input) |
| EXECUTE | PAUSED | Execution suspended |
| PAUSED | EXECUTE | Resuming execution |
| PAUSED | FAILED | Unrecoverable error while paused |
| VERIFY | DONE | No failed tasks |
| VERIFY | FIX | Failed tasks found |
| FIX | EXECUTE | Failed tasks reset, retry |
| FIX | DONE | All fixes applied successfully |
| FIX | FAILED | Retries exhausted |

## Task States
- `pending` - Not yet started
- `in_progress` - Currently executing
- `completed` - Successfully finished
- `failed` - Errored out
- `blocked` - Waiting on dependency

## Key Events
- `workflow.started` - Workflow execution started
- `workflow.state.transition` - State machine transition
- `workflow.fixing` - Fix retry cycle
- `workflow.completed` - Successfully done
- `workflow.failed` - Unrecoverable failure
- `task.started` - Individual task begins
- `task.completed` - Individual task succeeds
- `task.failed` - Individual task fails

## Runtime Behaviors
- `createPlan` auto-creates tasks from `plan.tasks[]` entries
- Memory store persists to file after each add (calls `save()`)
- Session, Review, and Parallel tables are initialized at startup
