---
name: masday-workflow-status
description: >
  Display a dashboard of all workflows and their current status. Shows task progress,
  memory statistics, and highlights blocked or failed workflows. Use when the user says
  "show status", "workflow dashboard", "what's running", "list workflows", or "check progress".
allowed-tools:
  - workflow.list
  - workflow.get
  - workflow.getStatus
  - workflow.listTasks
  - workflow.getCurrentTask
  - memory.stats
  - memory.recall_recent
---

# Masday Workflow Status

Show a dashboard of workflow progress and system state.

## Steps

1. **List all workflows**
   - Call `workflow.list` to get all workflows with their statuses
   - Group by status: executing, planning, ready, blocked, completed

2. **Get details for active workflows**
   - For each non-completed workflow, call `workflow.getStatus` for detailed state
   - Call `workflow.listTasks` to see task breakdown and completion percentages
   - Call `workflow.getCurrentTask` to identify the active task

3. **Get memory statistics**
   - Call `memory.stats` to show total memories, breakdown by type, and average importance

4. **Check recent activity**
   - Call `memory.recall_recent` to show the latest context entries

5. **Format the dashboard**
   ```
   === Workflow Dashboard ===

   Active:
   - [wf-001] "Add auth module" (EXECUTING) 3/5 tasks done
     Current: "Write auth middleware" (task-004)
   - [wf-002] "Fix memory leak" (PLANNING)

   Completed (recent):
   - [wf-003] "Refactor store" (DONE) 4/4 tasks

   Blocked:
   - (none)

   Memory: 47 entries | avg importance: 0.72
   ```

6. **Highlight issues**
   - Flag any workflows stuck in EXECUTE for over 30 minutes
   - Flag any tasks with FAILED status
   - Suggest next actions for blocked workflows

## Never

- Never modify workflow state -- this is a read-only dashboard
- Never show raw JSON -- always format as a human-readable summary
- Never omit blocked or failed workflows from the report
