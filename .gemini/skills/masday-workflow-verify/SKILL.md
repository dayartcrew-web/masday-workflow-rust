---
name: masday-workflow-verify
description: >
  Verify a completed workflow by running policy checks, tests, and code review. Validates
  that all acceptance criteria are met, tests pass, and no regressions were introduced.
  Use when the user says "verify workflow", "check completed work", "validate results",
  or "run verification".
allowed-tools:
  - workflow.get
  - workflow.getStatus
  - workflow.listTasks
  - policy.validate_completion
  - policy.validate_parallel_completion
  - policy.detect_scope_drift
  - tests.run
  - git.diff
  - git.status
  - filesystem.read
  - filesystem.list
  - filesystem.stat
  - memory.store
  - memory.recall_by_task
---

# Masday Workflow Verify

Verify that a completed workflow meets all acceptance criteria and quality standards.

## Steps

1. **Get workflow state**
   - Call `workflow.get` with the workflow ID
   - Call `workflow.getStatus` to confirm it is in VERIFY or DONE state
   - Call `workflow.listTasks` to see all tasks and their statuses

2. **Check for failed or incomplete tasks**
   - Identify any tasks with status FAILED, BLOCKED, or PENDING
   - If incomplete tasks exist, report them and recommend using `masday-workflow-fix`

3. **Validate each task against acceptance criteria**
   - For each completed task:
     - Call `policy.validate_completion` with workflow ID and task ID
     - Call `memory.recall_by_task` to load task artifacts and evidence
     - Cross-reference evidence against the acceptance criteria

4. **Run tests**
   - Call `tests.run` to execute the test suite
   - If any tests fail, report the failures with file names and error messages
   - Verify test coverage meets the 80% minimum threshold

5. **Check for regressions**
   - Call `git.status` to see all modified files
   - Call `git.diff` to review the full change set
   - Use `filesystem.read` to inspect changed files for quality issues
   - Check for: hardcoded values, missing error handling, deep nesting, large functions

6. **Detect scope drift**
   - Call `policy.detect_scope_drift` with the workflow output and original task scope
   - Report any deviations from the planned scope

7. **Validate parallel completions** (if applicable)
   - Call `policy.validate_parallel_completion` for tasks with parallel branches

8. **Store verification results**
   - Call `memory.store` with `memory_type: "artifact"` containing the verification report
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

- Never skip running the test suite during verification
- Never mark verification as passed if tests are failing
- Never ignore scope drift warnings
- Never skip the policy validation checks
