Resume the active workflow from where it was left off.

## Purpose

Re-enter an in-progress workflow, restore full context, and continue the active task. Used when starting a new Claude session or after a break.

## Steps

### 0. Sync Local State (MANDATORY — do this first)
```
CRITICAL: You MUST ensure .msd/ exists and is synced before proceeding.

Step 0a — Init .msd/ (use ONE of these):
  Option A: mcp__workflow-orchestrator__local_init({ cwd: process.cwd() })
  Option B (fallback): Bash: pnpm manage init

Step 0b — After loading workflow (step 1), sync DB → local:
  Option A: mcp__workflow-orchestrator__local_sync({ cwd: process.cwd(), workflow_id: workflowId })
  Option B (fallback): Bash: pnpm manage sync

DO NOT skip this step.
```

### 1. Load Workflow State
```
Call workflow.getActive

If no active workflow:
  → Report: "No active workflow found. Use /msd-start-work to begin."
  → STOP

If active workflow exists:
  → Record: workflowId, status, current task reference
  → Report workflow title and overall status
```

### 2. Load Plan
```
Call workflow.getPlan with { workflowId }

Review all tasks:
- List completed tasks (brief)
- Highlight current task (detailed)
- List remaining tasks (brief)

If no plan:
  → Report: "No plan found. Run /msd-plan first."
  → STOP
```

### 3. Load Current Task
```
Call workflow.getCurrentTask with { workflowId }

If no current task:
  → Find next pending task via workflow.listTasks
  → Call workflow.startTask to activate it
  → Report: "Starting next task: {title}"

Record: taskId, title, acceptance criteria, required context
```

### 4. Load Context Pack
```
Call semantic-search.search_hybrid_context_pack with:
{
  workflowId,
  planId,
  taskId
}

Also check .msd/ for existing local artifacts:
  → Read .msd/context/ files for previously assembled context
  → Read .msd/plans/ files for previous planning output
  → Read .msd/reports/ files for previous review/progress notes

Merge DB context pack with local artifact context.
Verify .msd/context/codebase/context-pack.md was written (auto-saved by the MCP tool).
If missing, save manually via local.save_artifact.

If context is stale or insufficient:
  → Re-run with expanded query
  → Check if new files were added since last session
```

### 5. Inspect Recent Progress
```
Call memory.recall_recent with { workflowId, limit: 5 }

Also check .msd/reports/ for local progress notes.

Review:
- What was done last session
- Any saved progress notes
- Any research stored
- Any review verdicts

This tells you WHERE to resume, not start over.
```

### 6. Restore Session State
```
Call session.get_state with { workflowId }

Check flags:
- contextLoaded: true/false
- reviewCompleted: true/false
- evidenceCollected: true/false

Resume from the appropriate point:
- Context not loaded → re-load context pack
- Context loaded, no implementation → delegate to executor
- Implementation done, no review → delegate to reviewer
- Review approved → delegate to verifier
```

### 7. Continue Active Task
```
Based on session state, continue with the appropriate action:

| Last State | Next Action |
|------------|-------------|
| Planning done | Start implementation (msd-executor) |
| Implementation in progress | Continue implementation |
| Implementation done | Run review (msd-reviewer) |
| Review returned REWORK | Fix issues (msd-executor) |
| Review APPROVED | Run verification (msd-verifier) |
| Verification PASS | Complete task (workflow.completeTask) |

When delegating to agents, remind them to write artifacts to .msd/:
  - Plans → local.save_artifact({ category: "plans", ... })
  - Research → local.save_artifact({ category: "context/research", ... })
  - Reports → local.save_artifact({ category: "reports", ... })
  - Analysis → local.save_artifact({ category: "context/codebase", ... })
```

### 8. Save Progress and Sync (after each action)
```
Step 8a — Save progress to DB:
  workflow.saveProgress({ workflowId, taskId, agentName, progressNote, evidence })

Step 8b — Save progress summary as local artifact:
  mcp__workflow-orchestrator__local_save_artifact({
    cwd: process.cwd(),
    category: "reports",
    filename: "progress-<task-slug>.md",
    content: "<progress note and key decisions>"
  })

Step 8c — Push local changes to DB (if any local state divergence):
  mcp__workflow-orchestrator__local_push({ cwd: process.cwd(), workflow_id: workflowId })

Step 8d — Sync DB state to local:
  Option A: mcp__workflow-orchestrator__local_sync({ cwd: process.cwd(), workflow_id: workflowId })
  Option B: pnpm manage sync

Step 8e — Update session state:
  session.patch_state({ contextLoaded: true, taskLoaded: true })
```

## Error Handling

| Error | Action |
|-------|--------|
| Stale session state | Re-load all state from MCP, ignore cached values |
| Task stuck in progress | Check progress notes, determine if restart needed |
| Review not found | Re-run review with msd-reviewer |
| Context missing | Re-run hybrid_context_pack with fresh query |
| .msd/ artifacts missing | Re-assemble context and save via local.save_artifact |

## Output

```
Resumed: {workflow title}
Task: {task title} — {status}
Last Action: {what was done last}
Local Artifacts: {files in .msd/}
Resuming At: {step name}
Agent: {which agent to use next}
```
