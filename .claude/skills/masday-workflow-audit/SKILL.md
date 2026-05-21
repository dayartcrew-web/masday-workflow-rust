---
name: masday-workflow-audit
description: >
  Audit active workflows for stuck tasks, missing reviews, scope drift, and stale sessions.
  Provides a health report with actionable recommendations. If invoked without a prompt,
  auto-continues through all steps and asks user to pick next step.
  Use when the user says "audit workflows", "check health", "find stuck tasks",
  "workflow audit", or "what needs attention".
allowed-tools:
  - workflow_list
  - workflow_get
  - workflow_getStatus
  - workflow_listTasks
  - capability_workflow_audit
  - memory_stats
  - memory_search
  - memory_recall_recent
  - memory_recall_documents
---

# Masday Workflow Audit

Audit workflows for issues and provide a health report.

## Steps

1. **Run system audit**
   - Call `capability_workflow_audit` with `maxAgeMinutes: 30` (configurable)
   - This detects: stuck tasks, missing reviews, scope drift, stale sessions

2. **List all workflows**
   - Call `workflow_list` to get a complete inventory
   - Filter for non-completed workflows as the audit focus

3. **Inspect each active workflow**
   - For each non-completed workflow:
     - Call `workflow_get` for full details
     - Call `workflow_getStatus` for current state
     - Call `workflow_listTasks` to check individual task statuses
   - Identify: tasks stuck in EXECUTING for too long, tasks in FAILED state, missing reviews

4. **Check memory health**
   - Call `memory_stats` for total entries, type distribution, and average importance
   - Call `memory_search` for entries tagged with blockers or issues
   - Call `memory_recall_recent` to find recent warnings or failures
   - Call `memory_recall_documents` to find stored decisions that may be stale

5. **Compile the audit report**
   ```
   === Workflow Audit Report ===

   Active workflows: 3
   Stuck tasks (>30min): 1
   Missing reviews: 0
   Scope drift warnings: 0
   Stale sessions: 0

   Issues:
   1. [wf-001] Task "build-api" stuck in EXECUTING for 47min
      Recommendation: Check if agent is responsive, consider reset

   Memory health: 47 entries, avg importance 0.72
   Recent blockers: none
   ```

6. **Report and ask next step**
   Use AskUserQuestion to present the audit report and let the user pick:
   ```
   Audit complete: [3 active workflows, 1 stuck task found]
   ```

   Ask user:
   - "/masday-workflow-fix — fix the stuck task"
   - "/masday-workflow-run — resume a workflow"
   - "Continue with another task"

## Never

- Never modify workflow state during an audit -- read-only analysis
- Never ignore stale sessions in the report
- Never skip the memory health check
- Never auto-fix issues without user confirmation

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
