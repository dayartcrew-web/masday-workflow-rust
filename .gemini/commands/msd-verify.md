Run pre-completion verification checks on the current task.

## Purpose

Final validation gate before task completion. Runs scope drift detection, checks review status, verifies evidence completeness, and confirms policy compliance. Only passes if the task is truly done.

## Pre-conditions

- [ ] Active workflow exists
- [ ] Current task has implementation progress
- [ ] Review has been submitted and APPROVED

If review is missing or not approved → STOP and suggest /msd-review first.

## Steps

### 1. Load Verification Context
```
Gather everything needed for verification:

1. workflow.getActive → workflowId
2. workflow.getCurrentTask → taskId, title, acceptance criteria
3. workflow.getPlan → task dependencies and context
4. memory.recall_recent → progress notes and evidence
5. review.get_latest → review decision and notes

If ANY pre-condition fails:
  → Report: "Cannot verify: {what's missing}"
  → STOP with specific instructions to fix
```

### 2. Scope Drift Check
```
Call policy.detect_scope_drift with:
{
  taskTitle: "{current task title}",
  acceptanceCriteria: ["{criterion 1}", "{criterion 2}", ...],
  requiredContext: ["{context 1}", "{context 2}", ...],
  outputText: "{summary of what was actually produced}",
  progressNote: "{what the executor claims was done}"
}

Evaluate result:
- If drift detected → REJECT: "Scope drift found: {details}"
  → List what drifted and how to bring it back
- If aligned → proceed to next check
```

### 3. Review Status Check
```
Call review.get_latest with { workflowId, taskId }

Check:
- If no review exists → REJECT: "No review submitted"
  → Suggest: "Run /msd-review first"
- If review.decision != "APPROVED" → REJECT: "Review not approved: {decision}"
  → Show review notes and required fixes
- If review approved → proceed to next check
```

### 4. Evidence Completeness Check
```
Verify all required evidence is present in progress notes:

Required evidence:
- [ ] Files modified are listed with paths
- [ ] Build output recorded (pass/fail with details)
- [ ] Test results recorded (pass/fail, count of tests)
- [ ] Lint results recorded (pass/fail, any warnings)

For each missing item:
  → REJECT: "Missing evidence: {what's missing}"

If all evidence present → proceed to next check
```

### 5. Policy Validation
```
Call policy.validate_completion with:
{
  workflowId: "{workflowId}",
  taskId: "{taskId}",
  completionSummary: "{summary of what was accomplished}"
}

If result.ok === false:
  → REJECT: "Policy blocked: {reason}"
  → Show specific policy violation and how to resolve

If result.ok === true:
  → ALL CHECKS PASS
```

### 6. Return Verdict

#### PASS — All checks passed
```
═══════════════════════════════════════════
  Verification Result: PASS
═══════════════════════════════════════════

Task: {title}
Checks performed: 4

| Check           | Status | Detail                    |
|-----------------|--------|---------------------------|
| Scope drift     | PASS   | No drift detected         |
| Review status   | PASS   | Approved by msd-reviewer  |
| Evidence        | PASS   | All evidence present      |
| Policy          | PASS   | No policy violations      |

✓ Task is ready for completion via workflow.completeTask
═══════════════════════════════════════════
```

#### REJECT — One or more checks failed
```
═══════════════════════════════════════════
  Verification Result: REJECT
═══════════════════════════════════════════

Task: {title}
Checks performed: 4

| Check           | Status  | Detail                    |
|-----------------|---------|---------------------------|
| Scope drift     | PASS    | No drift detected         |
| Review status   | FAIL    | Review not approved       |
| Evidence        | FAIL    | Missing: build output     |
| Policy          | —       | Skipped (prior check failed) |

Fix the following before re-verifying:
1. {specific fix instruction}
2. {specific fix instruction}

Then re-run: /msd-verify
═══════════════════════════════════════════
```

### 7. Complete Task (if PASS)
```
If all verification checks pass:

Call workflow.completeTask with:
{
  workflowId: "{workflowId}",
  taskId: "{taskId}",
  completionEvidence: {
    filesModified: [...],
    testsPassed: true,
    buildPassed: true,
    reviewApproved: true
  }
}

Report: "Task '{title}' marked complete."
Check if more tasks remain in the plan.
If yes → suggest /msd-continue for next task.
If no → suggest /msd-status to review workflow completion.
```

## Anti-Patterns

- Never approve if scope drift is detected
- Never approve if review is missing or not approved
- Never skip any of the 4 verification checks
- Never mark task complete yourself on REJECT — only on PASS
- Never proceed with later checks if an earlier check fails (fail fast)
