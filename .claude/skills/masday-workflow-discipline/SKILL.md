---
name: masday-workflow-discipline
description: >
  Enforce workflow discipline by validating execution policies, detecting scope drift,
  checking context freshness, and ensuring review gates are respected. Acts as a guardrail
  to prevent unauthorized state transitions and policy violations. Use when the user says
  "check discipline", "enforce policy", "validate workflow", "drift check", or "policy check".
allowed-tools:
  - workflow.get
  - workflow.getStatus
  - workflow.listTasks
  - workflow.getCurrentTask
  - workflow.startTask
  - workflow.completeTask
  - workflow.saveProgress
  - policy.validate_execution
  - policy.validate_completion
  - policy.validate_parallel_completion
  - policy.detect_scope_drift
  - policy.check_session_readiness
  - policy.require_context_refresh
  - memory.store
  - memory.search
  - memory.recall_documents
  - semantic-search.search_context_fingerprint
  - semantic-search.code_search
---

# Masday Workflow Discipline

Enforce policy, detect drift, and maintain workflow discipline.

## Steps

1. **Get current workflow state**
   - Call `workflow.get` to retrieve the workflow details
   - Call `workflow.getStatus` to see the current state machine position
   - Call `workflow.listTasks` to see all tasks and their statuses

2. **Check session readiness**
   - Call `policy.check_session_readiness` with the session key
   - Verify the session has all required context loaded
   - If readiness fails, identify what context is missing

3. **Validate execution permission**
   - Call `policy.validate_execution` with workflow ID, task ID, and session key
   - This checks: is the task in a valid state to start? does the agent have permission?
   - If validation fails, report the specific policy violation

4. **Check context freshness**
   - Call `semantic-search.search_context_fingerprint` with workflow, plan, and task IDs
   - Call `policy.require_context_refresh` to check if context has changed since last load
   - If refresh is needed, flag it and recommend reloading context before proceeding

5. **Detect scope drift**
   - Call `policy.detect_scope_drift` with:
     - `workflow_id` and `task_id`
     - `output_text`: the current task output or proposed changes
   - If drift is detected, report:
     - What was planned (original scope)
     - What was delivered (actual output)
     - The specific deviation

6. **Validate task completion**
   - Call `policy.validate_completion` with workflow ID and task ID
   - This checks: is the review approved? are acceptance criteria met?
   - If validation fails, list the specific gaps

7. **Validate parallel completion** (if applicable)
   - Call `policy.validate_parallel_completion` for tasks with parallel branches
   - Verify all branches completed before synthesis

8. **Search for policy violations in memory**
   - Call `memory.search` for "blocker" or "policy" tagged entries
   - Call `memory.recall_documents` for stored policy decisions

9. **Store discipline report**
   - Call `memory.store` with `memory_type: "artifact"` containing the discipline check results

10. **Report**
    ```
    === Workflow Discipline Report ===
    Workflow: [wf-001]

    Session: READY / NEEDS_CONTEXT
    Execution: ALLOWED / BLOCKED (reason)
    Completion: VALIDATED / GAPS (list gaps)
    Scope drift: NONE / DETECTED (describe)
    Context: FRESH / STALE (needs refresh)
    Review gates: ALL PASSED / PENDING (which)

    Overall: COMPLIANT / VIOLATIONS FOUND

    Actions required:
    1. <specific action to resolve violation>
    ```

## Never

- Never skip policy validation -- even if the task looks complete
- Never auto-approve a task that fails policy validation
- Never ignore scope drift -- always report it to the user
- Never bypass review gates for convenience
- Never mark a task complete without storing the discipline check results

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
