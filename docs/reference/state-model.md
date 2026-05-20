# State Model

This page captures the runtime state ownership model and the workflow engine architecture.

## Ownership model

- **PostgreSQL** (via DualWriteStore + Prisma) owns operational workflow/task state. JSON cache is fallback.
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

The DualWriteStore manages operational state with PostgreSQL as primary and JSON cache as fallback:

| Store                      | Contents                                                |
| -------------------------- | ------------------------------------------------------- |
| DualWriteWorkflowStore     | Active workflows, state transitions (PostgreSQL + JSON) |
| DualWriteTaskResultStore   | Task status, execution metadata (PostgreSQL + JSON)     |
| MemoryStore                | Memories (Prisma first, JSON fallback)                  |

## 16 Prisma tables

All 16 Prisma models are actively populated. Each table is wired to the MCP tools that trigger writes.

| Table             | Wired Via            | Trigger                                              |
| ----------------- | -------------------- | ---------------------------------------------------- |
| Workflow          | DualWriteStore       | workflow.create, execute, delete                     |
| Task              | DualWriteStore       | addTask, startTask, completeTask                     |
| Plan              | DualWriteStore       | createPlan                                           |
| Memory            | persistToPrisma()    | memory.store, store_research                         |
| ReviewDecision    | Prisma direct        | review.submit                                        |
| SessionState      | Prisma direct        | session.patch_state                                  |
| ParallelBranch    | Prisma direct        | workflow.createParallelBranches                      |
| ContextDocument   | Prisma direct        | memory.store_research                                |
| TaskProgressLog   | saveProgressDb()     | workflow.saveProgress                                |
| RetrievalLog      | logRetrieval()       | memory.search, semantic-search.*                     |
| TokenUsage        | trackTokens()        | workflow.saveProgress, memory.store_research         |
| EpisodicMemory    | setEpisodicPrisma()  | EpisodicMemory.add()                                 |
| GraphNode         | setGraphPrisma()     | GraphStore.addNode()                                 |
| GraphEdge         | setGraphPrisma()     | GraphStore.addEdge()                                 |
| WorkflowReminder  | setReminderPrisma()  | reminder.check                                       |
| LlmProviderConfig | Prisma direct        | LLM provider configuration storage                   |

## Status conventions

All status values are UPPERCASE in PostgreSQL:

- **Workflow:** INIT, ANALYZE, PLAN, EXECUTE, VERIFY, FIX, DONE, FAILED, PAUSED
- **Task:** PENDING, RUNNING, DONE, FAILED
- **Plan:** ACTIVE, PENDING, READY, DONE
- **Review:** APPROVED, REWORK_REQUIRED, BLOCKED

DualWriteStore maps in-memory lowercase TaskState to UPPERCASE for Prisma.

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

The MCP server instantiates `OrchestratingEngine` with `coordinator: false` and `enableSkillRouting: false`.

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
- **FIX** — Failed tasks are reset to PENDING, re-executes up to `maxFixRetries`. Emits `workflow.fixing` event. Can also transition directly to DONE or FAILED.
- **PAUSED** — Execution suspended (e.g., waiting on external input). Can resume back to EXECUTE.
- **FAILED** — Unrecoverable error. Any state can transition to FAILED.
- **StateMachine** emits `workflow.state.transition` events on every state change.

## Task execution features

| Feature            | Description                                                       |
| ------------------ | ----------------------------------------------------------------- |
| DAG execution      | Tasks with dependency ordering, parallel where possible            |
| Timeout            | AbortController-based (default 5 min), configurable via `taskTimeout` |
| Output piping      | Dependent tasks receive `dependencyOutputs` from prerequisites     |
| Auto-task creation | `createPlan` auto-creates tasks from `plan.tasks[]` entries        |
| Priority queue     | OrchestratingEngine uses TaskQueue with agent-type priorities      |
| Agent dispatch     | Tasks routed through SkillRouter to appropriate agent workers      |
| Memory persistence | Memory persists to PostgreSQL (Prisma first) with JSON cache fallback |
| Startup init       | Session, Review, and Parallel tables are initialized at startup    |

## Why this matters

The project mixes documented concepts from several phases and experiments. Keeping the ownership model explicit prevents docs from implying that every state artifact serves the same purpose.

## Related docs

- [Architecture](../architecture.md)
- [Getting started](../getting-started.md)
- [MCP tools](./mcp-tools.md)
