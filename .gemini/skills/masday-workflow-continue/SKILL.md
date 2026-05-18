---
name: masday-workflow-continue
description: |
  Resume an interrupted or paused workflow from where it left off.
  Use when a workflow session was interrupted (context limit, user disconnect, error)
  and you need to pick up exactly where the last task stopped.
  Handles: paused workflows, failed tasks with retry budget, partially completed plans.
allowed-tools:
  - workflow.getActive
  - workflow.getCurrentTask
  - workflow.getPlan
  - workflow.listTasks
  - workflow.startTask
  - workflow.execute
  - workflow.saveProgress
  - workflow.completeTask
  - review.get_latest
  - memory.recall_recent
  - memory.recall_by_task
  - memory.recall_documents
  - semantic-search.search_hybrid_context_pack
  - semantic-search.search_context_fingerprint
  - policy.validate_execution
  - tests.run
---

# Masday Workflow Continue

Resume an interrupted workflow. Detects the last known state and picks up from the exact stopping point.

## When to Use

- Previous session ran out of context mid-workflow
- Workflow paused or suspended by user/system
- A task failed and needs retry from where it stopped
- You want to continue yesterday's work without re-creating the workflow

## Do NOT Use When

- Starting a brand new task (use `masday-workflow-new`)
- Workflow is already DONE (use `masday-workflow-status` to check)
- You want to fix a broken workflow (use `masday-workflow-fix`)

## Steps

### 1. Detect Active Workflow

```
Call: workflow.getActive({ cwd: process.cwd() })

If no active workflow:
  Call: workflow.list({ status: "executing" })

If still none:
  Call: workflow.list({ status: "reviewing" })

If no workflows found at all:
  STOP: "No active or paused workflows found. Use /masday-workflow-new to start one."

Record: workflowId, workflow status
```

### 2. Load Full State

```
Call: workflow.getPlan({ workflow_id: workflowId })
Call: workflow.listTasks({ workflow_id: workflowId })
Call: workflow.getCurrentTask({ workflow_id: workflowId })

Record: plan, all tasks with statuses, current task (if any)
```

### 3. Determine Resume Point

Based on task statuses, determine where to resume:

| Scenario | Action |
|----------|--------|
| Current task exists, status `in_progress` | Resume that task (Step 4) |
| Current task `failed` with retries left | Retry that task (Step 4) |
| Current task `completed`, next task `pending` | Start next task (Step 5) |
| All tasks `completed` | Move to VERIFY phase (Step 6) |
| No current task, some `pending` | Start first pending task (Step 5) |
| All tasks `pending` (plan never started) | Call `workflow.execute` (Step 7) |

### 4. Resume Interrupted Task

For a task that was `in_progress` or `failed`:

```
# Load prior context
Call: memory.recall_by_task({ task_id: taskId, limit: 10 })
Call: review.get_latest({ workflow_id: workflowId, task_id: taskId })

# Check fingerprint for context freshness
Call: semantic-search.search_context_fingerprint({
  workflow_id: workflowId,
  plan_id: planId,
  task_id: taskId
})

# Build fresh context pack
Call: semantic-search.search_hybrid_context_pack({
  workflow_id: workflowId,
  plan_id: planId,
  task_id: taskId
})

# Validate execution is allowed
Call: policy.validate_execution({
  workflow_id: workflowId,
  task_id: taskId,
  session_key: "resume-" + Date.now()
})

# Re-start the task
Call: workflow.startTask({
  workflow_id: workflowId,
  task_id: taskId,
  agent_name: "<from task ownerAgent>"
})

# Proceed with task execution per masday-workflow-run
```

### 5. Start Next Pending Task

```
# Find the next task whose dependencies are met
For each pending task (in plan order):
  Check if all dependency tasks are "completed"
  If yes: that's the next task
  If no: skip, check next

Call: workflow.startTask({
  workflow_id: workflowId,
  task_id: nextTaskId,
  agent_name: "<from task ownerAgent>"
})

# Execute task following masday-workflow-run pattern
```

### 6. All Tasks Completed — Verify

```
# If all tasks are "completed", check if review passed
Call: review.get_latest({ workflow_id: workflowId, task_id: lastTaskId })

If review is APPROVED:
  Call: workflow.completeTask({ workflow_id: workflowId, task_id: lastTaskId })
  Report: "Workflow complete! All tasks done."

If review is REWORK_REQUIRED:
  Restart the task with reviewer feedback as context

If no review:
  Report: "All tasks completed but review pending. Run /masday-workflow-verify."
```

### 7. Plan Exists But Never Started

```
Call: workflow.execute({ id: workflowId })

This transitions INIT -> ANALYZE -> PLAN -> EXECUTE automatically.
Then follow the task execution loop.
```

### 8. Save Progress After Each Action

```
After every significant action, save progress:

Call: workflow.saveProgress({
  workflow_id: workflowId,
  task_id: taskId,
  agent_name: "masday-continue",
  progress_note: "<what was just done>",
  evidence: ["<file paths changed>", "<test results>"]
})
```

## Output Format

```
══════════════════════════════════════════
   Workflow Resumed: {workflow name}
══════════════════════════════════════════

Workflow ID: {id}
Status: {status} -> {new_status}
Resume point: Task "{title}" ({status})

Tasks:
  ✅ {completed task 1}
  ✅ {completed task 2}
  🔄 {resumed task} <-- RESUMING HERE
  ⏳ {pending task 4}
  ⏳ {pending task 5}

Context loaded: {count} memories, context pack rebuilt
Previous progress: "{last progress note}"

══════════════════════════════════════════
   NEXT ACTION
══════════════════════════════════════════

{specific next step to take}
```

## Error Handling

| Error | Recovery |
|-------|----------|
| No active workflow | List all workflows, prompt user to pick one or start new |
| Workflow in unexpected state | Log state, suggest `masday-workflow-fix` |
| Task has no ownerAgent | Assign to `masday-executor` as default |
| Context fingerprint mismatch | Rebuild full context pack from scratch |
| Policy validation fails | Check policy output for specific violation, address it |
| Memory recall empty | Proceed without prior context, note "cold start" in progress |

## What You NEVER Do

- NEVER create a new workflow when one already exists for this task
- NEVER re-execute completed tasks
- NEVER skip the fingerprint check — stale context causes drift
- NEVER assume task order — always check dependencies
- NEVER mark a task complete without review approval
- NEVER discard prior progress notes — they contain recovery context
- NEVER start a task without calling `policy.validate_execution` first
