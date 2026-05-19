---
name: masday-workflow-next
description: |
  Smart auto-detect skill that figures out what to do next.
  Checks for active workflows, pending tasks, failed tasks, or no workflow at all.
  Automatically routes to: continue, start next task, verify, or create new workflow.
  Use as the default "what should I work on" command.
allowed-tools:
  - workflow.getActive
  - workflow.getCurrentTask
  - workflow.getPlan
  - workflow.listTasks
  - workflow.list
  - workflow.startTask
  - workflow.execute
  - workflow.saveProgress
  - workflow.completeTask
  - workflow.create
  - workflow.createPlan
  - review.get_latest
  - memory.recall_recent
  - memory.recall_by_task
  - memory.search
  - semantic-search.search_hybrid_context_pack
  - policy.validate_execution
  - policy.validate_completion
---

# Masday Workflow Next

Smart auto-detect: figures out what you should work on next and does it. One command to rule them all.

## When to Use

- Start of a new session — "what should I work on?"
- After a break — "continue where I left off"
- When unsure about workflow state — "what's next?"
- As a daily driver instead of remembering specific workflow commands

## How It Works

```
┌─────────────────────────────────┐
│     workflow.getActive?        │
└──────────┬──────────────────────┘
           │
     ┌─────┴──────┐
     │ Found?     │
     ├─YES────────┴─┐
     │              │
     ▼              ▼
  Check task    ┌──────────────┐
  status        │ workflow.list│
     │          └──────┬───────┘
     │                 │
     │           Found any?
     │           ├─YES: pick most recent
     │           └─NO: ↓
     │                 │
     │          ┌──────┴──────┐
     │          │ No workflow │
     │          │ Ask user to │
     │          │ describe task│
     │          └─────────────┘
     ▼
  Route based on state:
  ├─ in_progress → resume task
  ├─ failed      → retry task
  ├─ completed   → start next pending
  ├─ all done    → verify workflow
  └─ no plan     → create plan
```

## Steps

### 1. Scan Workflow State

```
# Check for active workflow in current project
Call: workflow.getActive({ cwd: process.cwd() })

If active workflow found:
  Record: workflowId, status
  Go to Step 2

# No active workflow — check for any non-DONE workflows
Call: workflow.list({})

If any workflow with status in ["EXECUTE", "VERIFY", "BLOCKED", "PLAN"]:
  Pick the most recent one
  Record: workflowId
  Go to Step 2

# No workflows at all
Go to Step 7 (Create New)
```

### 2. Load Workflow Context

```
Call: workflow.getPlan({ workflow_id: workflowId })
Call: workflow.listTasks({ workflow_id: workflowId })
Call: workflow.getCurrentTask({ workflow_id: workflowId })

Record: plan, tasks[], currentTask
```

### 3. Route Based on State

```
If currentTask exists:
  If currentTask.status == "RUNNING":   Go to Step 4 (Resume)
  If currentTask.status == "FAILED":    Go to Step 5 (Retry)
  If currentTask.status == "DONE":
    Find next pending task → Go to Step 6 (Next Task)
    If no pending tasks → Go to Step 8 (Verify)

If no currentTask:
  Find first pending task → Go to Step 6 (Next Task)
  If no pending tasks → Go to Step 8 (Verify)
  If all tasks pending and no plan → Go to Step 9 (Create Plan)
```

### 4. Resume In-Progress Task

```
# Load prior context
Call: memory.recall_by_task({ task_id: currentTask.id, limit: 10 })
Call: review.get_latest({ workflow_id: workflowId, task_id: currentTask.id })

# Build fresh context
Call: semantic-search.search_hybrid_context_pack({
  workflow_id: workflowId,
  plan_id: planId,
  task_id: currentTask.id
})

# Validate and start
Call: policy.validate_execution({
  workflow_id: workflowId,
  task_id: currentTask.id,
  session_key: "next-" + Date.now()
})

Call: workflow.startTask({
  workflow_id: workflowId,
  task_id: currentTask.id,
  agent_name: currentTask.ownerAgent || "masday-executor"
})

# Execute task following masday-workflow-run pattern
Report: "Resumed task: {title}"
```

### 5. Retry Failed Task

```
# Check retry budget
If task retryCount >= maxRetries:
  Report: "Task '{title}' exceeded retry limit. Use /masday-workflow-fix for manual intervention."
  STOP

# Load error context
Call: memory.recall_by_task({ task_id: taskId, limit: 10 })
Call: review.get_latest({ workflow_id: workflowId, task_id: taskId })

# Rebuild context and retry
Call: semantic-search.search_hybrid_context_pack({
  workflow_id: workflowId,
  plan_id: planId,
  task_id: taskId
})

Call: workflow.startTask({
  workflow_id: workflowId,
  task_id: taskId,
  agent_name: currentTask.ownerAgent || "masday-executor"
})

Call: workflow.saveProgress({
  workflow_id: workflowId,
  task_id: taskId,
  agent_name: "masday-next",
  progress_note: "Retrying failed task after auto-detect"
})

Report: "Retrying failed task: {title} (attempt {n})"
```

### 6. Start Next Pending Task

```
# Find next task whose dependencies are met
Let nextTask = null
For each task with status "PENDING" (in plan order):
  deps = task.dependencies || []
  allDepsCompleted = deps.every(depId =>
    tasks.find(t => t.id === depId)?.status === "DONE"
  )
  If allDepsCompleted:
    nextTask = task
    Break

If no nextTask:
  Report: "All pending tasks have unmet dependencies. Check workflow plan."
  STOP

# Build context for new task
Call: semantic-search.search_hybrid_context_pack({
  workflow_id: workflowId,
  plan_id: planId,
  task_id: nextTask.id
})

Call: workflow.startTask({
  workflow_id: workflowId,
  task_id: nextTask.id,
  agent_name: nextTask.ownerAgent || "masday-executor"
})

Report: "Starting next task: {title}"
```

### 7. Create New Workflow (No Existing)

```
Ask user: "No active workflows found. What would you like to work on?"

After user provides description:
  Call: workflow.create({ name: "<short name>", description: "<user description>" })
  Record: workflowId

  Then create a plan for the workflow:
  Call: workflow.createPlan({
    workflow_id: workflowId,
    plan: { tasks: [{ title, agent, skill, dependencies, input }] }
  })

  Call: workflow.execute({ id: workflowId })
  Report: "Created and started new workflow: {name}"
```

### 8. Verify Completed Workflow

```
# Check last task review
Call: review.get_latest({ workflow_id: workflowId, task_id: lastTaskId })

If review.status == "APPROVED":
  Call: workflow.completeTask({
    workflow_id: workflowId,
    task_id: lastTaskId
  })
  Report: "Workflow complete! All tasks verified and approved."

If review.status == "REWORK_REQUIRED":
  Restart last task with reviewer feedback
  Report: "Review requested changes. Re-working: {title}"

If no review:
  Report: "All tasks completed but awaiting review. Run /masday-workflow-verify."
```

### 9. Save Progress

```
After every action, persist state:

Call: workflow.saveProgress({
  workflow_id: workflowId,
  task_id: taskId,
  agent_name: "masday-next",
  progress_note: "<what was done>",
  evidence: ["<files changed>"]
})
```

## Output Format

```
══════════════════════════════════════════
   Workflow Next: {auto-detected action}
══════════════════════════════════════════

Workflow: {name} ({id})
Action: RESUME | RETRY | NEXT_TASK | VERIFY | CREATE_NEW

┌─ Task Progress ────────────────────────┐
│ ✅ {task 1} — {status}                │
│ ✅ {task 2} — {status}                │
│ 🔄 {current task} — {status} ← HERE   │
│ ⏳ {task 4} — pending                 │
│ ⏳ {task 5} — pending                 │
└────────────────────────────────────────┘

Context: {count} memories loaded
Last progress: "{note}"

══════════════════════════════════════════
   EXECUTING: {specific next step}
══════════════════════════════════════════
```

## Error Handling

| Error | Recovery |
|-------|----------|
| Multiple active workflows | List all, pick most recently updated |
| Workflow stuck in `blocked` | Check blocker reason, suggest `masday-workflow-fix` |
| Policy validation denied | Display violation, suggest manual fix |
| Empty context pack | Proceed with cold start, warn user |
| No pending tasks, not all done | Check for parallel branches, suggest audit |

## What You NEVER Do

- NEVER skip the auto-detect — always check state before acting
- NEVER assume which task is next — check dependencies
- NEVER create a duplicate workflow for the same task
- NEVER mark tasks complete without review
- NEVER discard existing workflow state
- NEVER start a task without `policy.validate_execution`
- NEVER ignore failed tasks — always attempt retry first
- NEVER proceed without saving progress after each action
