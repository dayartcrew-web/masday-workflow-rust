---
name: masday-workflow-fix
description: >
  Diagnose and fix a failed or incomplete workflow. Analyzes failure causes, searches for
  related code solutions, re-runs failed tasks, and stores the fix for future reference.
  Use when the user says "fix workflow", "workflow failed", "retry task",
  "repair workflow", or "workflow is stuck".
allowed-tools:
  - workflow_get
  - workflow_getStatus
  - workflow_listTasks
  - workflow_startTask
  - workflow_completeTask
  - workflow_execute
  - workflow_addTask
  - workflow_saveProgress
  - policy_validate_execution
  - policy_validate_completion
  - policy_detect_scope_drift
  - memory_store
  - memory_recall_by_task
  - memory_search
  - semantic-search_code_search
  - semantic-search_search_hybrid_context_pack
  - tests_run
  - npm_install
  - npm_run
---

# Masday Workflow Fix

Diagnose and fix issues in a failed or incomplete workflow.

## Steps

1. **Get workflow state**
   - Call `workflow_get` with the workflow ID
   - Call `workflow_getStatus` to see current state (BLOCKED, EXECUTE with failures)
   - Call `workflow_listTasks` to identify all failed or incomplete tasks

2. **Diagnose each failure**
   - For each task with status FAILED or BLOCKED:
     - Read the error details from the task result
     - Call `memory_recall_by_task` to load the task's execution context
     - Categorize the failure: configuration error, code bug, dependency issue, or agent timeout

3. **Search for solutions**
   - Call `semantic-search_code_search` with queries related to the error
   - Call `semantic-search_search_hybrid_context_pack` to get broader codebase context
   - Call `memory_search` for similar failures encountered in past workflows

4. **Determine fix approach**
   - **Configuration error**: Fix the task parameters and call `workflow_addTask` with corrected config
   - **Code bug**: Analyze the code, fix the issue, then call `workflow_startTask` to retry
   - **Dependency issue**: Adjust task dependency ordering, re-add tasks with corrected deps
   - **Agent timeout**: Consider splitting the task into smaller subtasks

5. **Validate before re-execution**
   - Call `policy_validate_execution` to ensure the fix is sound
   - Call `policy_detect_scope_drift` to confirm the fix stays within scope

6. **Re-execute the workflow**
   - Call `workflow_execute` with the workflow ID to resume
   - Monitor the re-execution:
     - Call `workflow_listTasks` to track progress
     - Call `workflow_saveProgress` at each milestone
     - Call `policy_validate_completion` after each task completes

7. **Store the fix**
   - Call `memory_store` with `memory_type: "learning"` containing:
     - Root cause of the failure
     - The fix applied
     - Steps to prevent similar failures
   - Tag with the error category for future recall

8. **Report**
   ```
   === Workflow Fix Report ===
   Workflow: [wf-001]
   Failed tasks: 2
   Root causes:
   1. Missing dependency in task-003 -> added task-002b as prerequisite
   2. Invalid config in task-004 -> corrected input parameters

   Fix applied: Re-executed with corrected tasks
   Result: 5/5 tasks now completed
   ```

## Never

- Never blindly retry without diagnosing the root cause first
- Never modify completed tasks -- only fix failed or pending ones
- Never skip the policy validation after applying fixes
- Never discard failure context -- always store it as a learning

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
