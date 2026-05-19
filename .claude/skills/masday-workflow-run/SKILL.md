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
  - workflow.getCurrentTask
  - workflow.startTask
  - workflow.completeTask
  - workflow.saveProgress
  - workflow.listTasks
  - workflow.createParallelBranches
  - workflow.completeParallelBranch
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
   - Call `workflow.listTasks` to see all planned tasks and their statuses
   - Confirm the task order and dependencies make sense
   - Report the task summary to the user before starting

3. **Execute the workflow**
   - Call `workflow.execute` with the workflow ID
   - This transitions the workflow through INIT -> ANALYZE -> PLAN -> EXECUTE

4. **Monitor task execution**
   - Loop through tasks as they execute:
     - Call `workflow.getCurrentTask` to identify the active task
     - Call `policy.validate_execution` before each task starts
     - Call `memory.recall_by_task` to load context for the current task
     - Perform the task work using appropriate tools
     - Call `policy.detect_scope_drift` to check for deviations from the plan
     - Call `workflow.saveProgress` with a progress note and evidence
     - Call `policy.validate_completion` after each task completes
     - Call `workflow.completeTask` to mark it done

5. **Handle parallel branches** (if applicable)
   - Call `workflow.createParallelBranches` for independent tasks
   - Monitor each branch independently
   - Call `policy.validate_parallel_completion` when all branches finish
   - Call `workflow.completeParallelBranch` for each completed branch

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

## Mandatory Review Pipeline

When this skill completes work on a workflow task, it MUST follow this pipeline:

`
STEP 1: Save progress to PostgreSQL
  workflow.saveProgress({
    workflow_id: "<workflowId>",
    task_id: "<taskId>",
    agent_name: "<current-agent>",
    progress_note: "<summary of work done>",
    evidence: ["<files modified>", "<tests run>"]
  })

STEP 2: Submit for review
  review.submit({
    workflow_id: "<workflowId>",
    task_id: "<taskId>",
    reviewer_agent: "masday-reviewer",
    decision: "<APPROVED | REWORK_REQUIRED | BLOCKED>",
    notes: "<what was done, key decisions>",
    gaps: ["<any gaps found>"]
  })

STEP 3: If REWORK_REQUIRED — fix and loop
  - Fix the gaps identified in the review
  - Re-save progress (workflow.saveProgress)
  - Re-submit review (review.submit)
  - Max 2 rework attempts, then STOP

STEP 4: If APPROVED — validate completion
  policy.validate_completion({
    workflow_id: "<workflowId>",
    task_id: "<taskId>"
  })

STEP 5: Complete task
  workflow.completeTask({ workflow_id: "<workflowId>", task_id: "<taskId>" })

STEP 6: Sync local state
  local.sync({ cwd: process.cwd(), workflow_id: "<workflowId>" })
`

### Never
- Never call workflow.completeTask without review.submit (APPROVED)
- Never skip policy.validate_completion before completion
- Never skip local.sync after completing a task
- Never claim done without saving progress to PostgreSQL
