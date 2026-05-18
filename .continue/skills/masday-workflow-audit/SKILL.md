---
name: masday-workflow-audit
description: >
  Audit active workflows for stuck tasks, missing reviews, scope drift, and stale sessions.
  Provides a health report with actionable recommendations. Use when the user says "audit
  workflows", "check health", "find stuck tasks", "workflow audit", or "what needs attention".
allowed-tools:
  - workflow.list
  - workflow.get
  - workflow.getStatus
  - workflow.listTasks
  - capability.workflow_audit
  - memory.stats
  - memory.search
  - memory.recall_recent
---

# Masday Workflow Audit

Audit workflows for issues and provide a health report.

## Steps

1. **Run system audit**
   - Call `capability.workflow_audit` with `maxAgeMinutes: 30` (configurable)
   - This detects: stuck tasks, missing reviews, scope drift, stale sessions

2. **List all workflows**
   - Call `workflow.list` to get a complete inventory
   - Filter for non-completed workflows as the audit focus

3. **Inspect each active workflow**
   - For each non-completed workflow:
     - Call `workflow.get` for full details
     - Call `workflow.getStatus` for current state
     - Call `workflow.listTasks` to check individual task statuses
   - Identify: tasks stuck in EXECUTING for too long, tasks in FAILED state, missing reviews

4. **Check memory health**
   - Call `memory.stats` for total entries, type distribution, and average importance
   - Call `memory.search` for entries tagged with blockers or issues
   - Call `memory.recall_recent` to find recent warnings or failures

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

6. **Provide actionable recommendations**
   - For stuck tasks: suggest reset or retry via `masday-workflow-fix`
   - For missing reviews: suggest running verification
   - For scope drift: suggest re-planning affected tasks

## Never

- Never modify workflow state during an audit -- read-only analysis
- Never ignore stale sessions in the report
- Never skip the memory health check
- Never auto-fix issues without user confirmation
