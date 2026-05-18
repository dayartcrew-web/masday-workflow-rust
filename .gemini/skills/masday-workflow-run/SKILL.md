---
name: masday-workflow-run
description: >
  Execute a planned workflow by running all tasks through the state machine (INIT -> ANALYZE ->
  PLAN -> EXECUTE -> VERIFY -> DONE). Validates execution policy at each step, monitors for
  scope drift, and stores progress artifacts. Use when the user says "run workflow",
  "execute plan", "start execution", or "run the workflow now".
allowed-tools:
  - workflow.get
  - workflow.execute
  - workflow.getStatus
  - workflow.get_current_task
  - workflow.start_task
  - workflow.complete_task
  - workflow.save_progress
  - workflow.list_tasks
  - workflow.create_parallel_branches
  - workflow.complete_parallel_branch
  - policy.validate_execution
  - policy.validate_completion
  - policy.validate_parallel_completion
  - policy.detect_scope_drift
  - memory.store
  - memory.recall_by_task
  - memory.recall_documents
  - tests.run
  - npm.run
  - git.status
---

# Masday Workflow Run

Execute a Masday workflow that has been planned and is ready to run.

## Steps

1. **Verify workflow exists and is ready**
   - Call `workflow.get` with the workflow ID to confirm it exists
   - Call `workflow.getStatus` to check it is in a runnable state (not already completed or blocked)
   - If the workflow is not ready, report the current state and stop

2. **List and review tasks**
   - Call `workflow.list_tasks` to see all planned tasks and their statuses
   - Confirm the task order and dependencies make sense
   - Report the task summary to the user before starting

3. **Execute the workflow**
   - Call `workflow.execute` with the workflow ID
   - This transitions the workflow through INIT -> ANALYZE -> PLAN -> EXECUTE

4. **Monitor task execution**
   - Loop through tasks as they execute:
     - Call `workflow.get_current_task` to identify the active task
     - Call `policy.validate_execution` before each task starts
     - Call `memory.recall_by_task` to load context for the current task
     - Perform the task work using appropriate tools
     - Call `policy.detect_scope_drift` to check for deviations from the plan
     - Call `workflow.save_progress` with a progress note and evidence
     - Call `policy.validate_completion` after each task completes
     - Call `workflow.complete_task` to mark it done

5. **Handle parallel branches** (if applicable)
   - Call `workflow.create_parallel_branches` for independent tasks
   - Monitor each branch independently
   - Call `policy.validate_parallel_completion` when all branches finish
   - Call `workflow.complete_parallel_branch` for each completed branch

6. **Store execution artifacts**
   - Call `memory.store` with `memory_type: "artifact"` for key outputs
   - Call `memory.store` with `memory_type: "learning"` for insights gained

7. **Report final status**
   - Call `workflow.getStatus` for the final state
   - Summarize: completed tasks, failures, warnings, and recommended next steps

## Never

- Never execute a workflow that is already in DONE or EXECUTE state without user confirmation
- Never skip policy validation between tasks
- Never ignore scope drift warnings -- report them to the user
- Never mark a task complete without calling `policy.validate_completion`
