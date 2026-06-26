---
name: masday-workflow-verify
description: >
  Verify a completed workflow by running policy checks, tests, and code review. Validates
  that all acceptance criteria are met, tests pass, and no regressions were introduced.
  Use when the user says "verify workflow", "check completed work", "validate results",
  or "run verification".
allowed-tools:
  - workflow_get
  - workflow_getStatus
  - workflow_listTasks
  - policy_validate_completion
  - policy_validate_parallel_completion
  - policy_detect_scope_drift
  - tests_run
  - git_diff
  - git_status
  - filesystem_read
  - filesystem_list
  - filesystem_stat
  - memory_store
  - memory_recall_by_task
---

# Masday Workflow Verify

Verify that a completed workflow meets all acceptance criteria and quality standards.

## Steps

This skill enforces **mandatory step completion**. Each step must be completed before proceeding. Do not skip steps.


1. **Get workflow state**
   - Call `workflow_get` with the workflow ID
   - Call `workflow_getStatus` to confirm it is in VERIFY or DONE state
   - Call `workflow_listTasks` to see all tasks and their statuses

2. **Check for failed or incomplete tasks**
   - Identify any tasks with status FAILED, BLOCKED, or PENDING
   - If incomplete tasks exist, report them and recommend using `masday-workflow-fix`

3. **Validate each task against acceptance criteria**
   - For each completed task:
     - Call `policy_validate_completion` with workflow ID and task ID
     - Call `memory_recall_by_task` to load task artifacts and evidence
     - Cross-reference evidence against the acceptance criteria

4. **Run tests**
   - Call `tests_run` to execute the test suite
   - If any tests fail, report the failures with file names and error messages
   - Verify test coverage meets the 80% minimum threshold

5. **Check for regressions**
   - Call `git_status` to see all modified files
   - Call `git_diff` to review the full change set
   - Use `filesystem_read` to inspect changed files for quality issues
   - Check for: hardcoded values, missing error handling, deep nesting, large functions


**GATE**: Verify steps 1-5 are complete before proceeding.

6. **Detect scope drift**
   - Call `policy_detect_scope_drift` with the workflow output and original task scope
   - Report any deviations from the planned scope

7. **Validate parallel completions** (if applicable)
   - Call `policy_validate_parallel_completion` for tasks with parallel branches

8. **Store verification results**
   - Call `memory_store` with `memory_type: "artifact"` containing the verification report
   - Include: pass/fail status, test results, issues found, recommendations

9. **Report verification summary**
   ```
   === Verification Report ===
   Workflow: [wf-001] "Add auth module"
   Status: PASS / FAIL / PARTIAL

   Tasks: 5/5 completed
   Tests: 47/47 passing (coverage: 84%)
   Scope drift: None detected
   Issues: 0 critical, 1 medium (see below)

   Medium: auth.ts line 42 - consider extracting token validation
   ```

## Never
- Never skip any step — complete each step before proceeding
- Never bypass a GATE marker without validating prior steps
- Never claim completion without executing all steps in order

- Never skip running the test suite during verification
- Never mark verification as passed if tests are failing
- Never ignore scope drift warnings
- Never skip the policy validation checks

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
