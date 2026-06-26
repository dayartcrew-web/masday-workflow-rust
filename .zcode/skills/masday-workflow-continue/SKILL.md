---
name: masday-workflow-continue
description: |
  Resume an interrupted or paused workflow from where it left off.
  Use when a workflow session was interrupted (context limit, user disconnect, error)
  and you need to pick up exactly where the last task stopped.
  Handles: paused workflows, failed tasks with retry budget, partially completed plans.
allowed-tools:
  - workflow_getActive
  - workflow_getCurrentTask
  - workflow_getPlan
  - workflow_listTasks
  - workflow_startTask
  - workflow_execute
  - workflow_saveProgress
  - workflow_completeTask
  - review_get_latest
  - memory_recall_recent
  - memory_recall_by_task
  - memory_recall_documents
  - semantic-search_search_hybrid_context_pack
  - semantic-search_search_context_fingerprint
  - policy_validate_execution
  - tests_run
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

This skill enforces **mandatory step completion**. Each step must be completed before proceeding. Do not skip steps.

### 1. Detect Active Workflow

```
Call: workflow_getActive({ cwd: process.cwd() })

If no active workflow:
  Call: workflow_list({ status: "EXECUTE" })

If still none:
  Call: workflow_list({ status: "reviewing" })

If no workflows found at all:
  STOP: "No active or paused workflows found. Use /masday-workflow-new to start one."

Record: workflowId, workflow status
```

### 2. Load Full State

```
Call: workflow_getPlan({ workflow_id: workflowId })
Call: workflow_listTasks({ workflow_id: workflowId })
Call: workflow_getCurrentTask({ workflow_id: workflowId })

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
| All tasks `pending` (plan never started) | Call `workflow_execute` (Step 7) |

### 4. Resume Interrupted Task

For a task that was `in_progress` or `failed`:

```
# Load prior context
Call: memory_recall_by_task({ task_id: taskId, limit: 10 })
Call: review_get_latest({ workflow_id: workflowId, task_id: taskId })

# Check fingerprint for context freshness
Call: semantic-search_search_context_fingerprint({
  workflow_id: workflowId,
  plan_id: planId,
  task_id: taskId
})

# Build fresh context pack
Call: semantic-search_search_hybrid_context_pack({
  workflow_id: workflowId,
  plan_id: planId,
  task_id: taskId
})

# Validate execution is allowed
Call: policy_validate_execution({
  workflow_id: workflowId,
  task_id: taskId,
  session_key: "resume-" + Date.now()
})

# Re-start the task
Call: workflow_startTask({
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
  Check if all dependency tasks are "DONE"
  If yes: that's the next task
  If no: skip, check next

Call: workflow_startTask({
  workflow_id: workflowId,
  task_id: nextTaskId,
  agent_name: "<from task ownerAgent>"
})

# Execute task following masday-workflow-run pattern
```

### 6. All Tasks Completed — Verify

```
# If all tasks are "DONE", check if review passed
Call: review_get_latest({ workflow_id: workflowId, task_id: lastTaskId })

If review is APPROVED:
  Call: workflow_completeTask({ workflow_id: workflowId, task_id: lastTaskId })
  Report: "Workflow complete! All tasks done."

If review is REWORK_REQUIRED:
  Restart the task with reviewer feedback as context

If no review:
  Report: "All tasks completed but review pending. Run /masday-workflow-verify."
```

### 7. Plan Exists But Never Started

```
Call: workflow_execute({ id: workflowId })

This transitions INIT -> ANALYZE -> PLAN -> EXECUTE automatically.
Then follow the task execution loop.
```

**GATE**: Pre-completion checkpoint. Verify all prior steps are fully complete.

### 8. Save Progress After Each Action

```
After every significant action, save progress:

Call: workflow_saveProgress({
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
- NEVER start a task without calling `policy_validate_execution` first

## Mandatory Review Pipeline

When this skill completes work on a workflow task, it MUST follow this pipeline:

`
STEP 1: Save progress to PostgreSQL
  workflow_saveProgress({
    workflow_id: "<workflowId>",
    task_id: "<taskId>",
    agent_name: "<current-agent>",
    progress_note: "<summary of work done>",
    evidence: ["<files modified>", "<tests run>"]
  })

STEP 2: Submit for review
  review_submit({
    workflow_id: "<workflowId>",
    task_id: "<taskId>",
    reviewer_agent: "masday-reviewer",
    decision: "<APPROVED | REWORK_REQUIRED | BLOCKED>",
    notes: "<what was done, key decisions>",
    gaps: ["<any gaps found>"]
  })

STEP 3: If REWORK_REQUIRED — fix and loop
  - Fix the gaps identified in the review
  - Re-save progress (workflow_saveProgress)
  - Re-submit review (review_submit)
  - Max 2 rework attempts, then STOP

STEP 4: If APPROVED — validate completion
  policy_validate_completion({
    workflow_id: "<workflowId>",
    task_id: "<taskId>"
  })

STEP 5: Complete task
  workflow_completeTask({ workflow_id: "<workflowId>", task_id: "<taskId>" })

STEP 6: Sync local state
  local_sync({ cwd: process.cwd(), workflow_id: "<workflowId>" })
`

### Never
- Never call workflow_completeTask without review_submit (APPROVED)
- Never skip policy_validate_completion before completion
- Never skip local_sync after completing a task
- Never claim done without saving progress to PostgreSQL
- Never skip any step — complete each step before proceeding
- Never bypass a GATE marker without validating prior steps
- Never claim completion without executing all steps in order
