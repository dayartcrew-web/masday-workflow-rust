---
name: masday-workflow-fix
description: >
  Diagnose and fix a failed or incomplete workflow. Analyzes failure causes, searches for
  related code solutions, re-runs failed tasks, and stores the fix for future reference.
  Use when the user says "fix workflow", "workflow failed", "retry task",
  "repair workflow", or "workflow is stuck".
allowed-tools:
  - workflow.get
  - workflow.getStatus
  - workflow.listTasks
  - workflow.startTask
  - workflow.completeTask
  - workflow.execute
  - workflow.addTask
  - workflow.saveProgress
  - policy.validate_execution
  - policy.validate_completion
  - policy.detect_scope_drift
  - memory.store
  - memory.recall_by_task
  - memory.search
  - semantic-search.code_search
  - semantic-search.search_hybrid_context_pack
  - tests.run
  - npm.install
  - npm.run
---

# Masday Workflow Fix

Diagnose and fix issues in a failed or incomplete workflow.

## Steps

1. **Get workflow state**
   - Call `workflow.get` with the workflow ID
   - Call `workflow.getStatus` to see current state (BLOCKED, EXECUTE with failures)
   - Call `workflow.listTasks` to identify all failed or incomplete tasks

2. **Diagnose each failure**
   - For each task with status FAILED or BLOCKED:
     - Read the error details from the task result
     - Call `memory.recall_by_task` to load the task's execution context
     - Categorize the failure: configuration error, code bug, dependency issue, or agent timeout

3. **Search for solutions**
   - Call `semantic-search.code_search` with queries related to the error
   - Call `semantic-search.search_hybrid_context_pack` to get broader codebase context
   - Call `memory.search` for similar failures encountered in past workflows

4. **Determine fix approach**
   - **Configuration error**: Fix the task parameters and call `workflow.addTask` with corrected config
   - **Code bug**: Analyze the code, fix the issue, then call `workflow.startTask` to retry
   - **Dependency issue**: Adjust task dependency ordering, re-add tasks with corrected deps
   - **Agent timeout**: Consider splitting the task into smaller subtasks

5. **Validate before re-execution**
   - Call `policy.validate_execution` to ensure the fix is sound
   - Call `policy.detect_scope_drift` to confirm the fix stays within scope

6. **Re-execute the workflow**
   - Call `workflow.execute` with the workflow ID to resume
   - Monitor the re-execution:
     - Call `workflow.listTasks` to track progress
     - Call `workflow.saveProgress` at each milestone
     - Call `policy.validate_completion` after each task completes

7. **Store the fix**
   - Call `memory.store` with `memory_type: "learning"` containing:
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
