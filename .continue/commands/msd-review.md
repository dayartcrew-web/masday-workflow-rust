Review the active task against acceptance criteria, evidence, and policy.

## Purpose

Quality gate that validates implementation output against the task's acceptance criteria. Returns a structured verdict that determines whether the task can proceed to completion.

## Pre-conditions

- [ ] Active workflow exists
- [ ] Current task has implementation progress
- [ ] Progress evidence exists (files modified, build/test results)

## Steps

### 1. Load Review Context
```
Gather all context needed for review:

1. workflow.getCurrentTask → title, acceptance criteria, required context
2. workflow.getPlan → understand task's role in the bigger picture
3. memory.recall_recent → what the executor claims to have done
4. workflow.saveProgress → evidence: files, tests, build results

If ANY of these are missing:
  → Report: "Cannot review: {what's missing}"
  → STOP
```

### 2. Validate Acceptance Criteria
```
For EACH acceptance criterion:

1. Identify what observable behavior it requires
2. Check if the implementation satisfies it
   - Read the relevant source files
   - Trace the code path
   - Verify edge cases are handled
3. Run verification if applicable (tests, build, lint)
4. Record: criterion → status (PASS/FAIL) → evidence

Example evaluation:
  Criterion: "API endpoint returns 200 for valid input"
  Action: Read the endpoint handler, check response format
  Result: PASS — handler returns { success: true, data: {...} }

  Criterion: "Returns 400 for invalid input"
  Action: Check for Zod validation, test for error response
  Result: FAIL — No input validation middleware found
```

### 3. Code Quality Check
```
Review changed files for:

Quality:
- [ ] TypeScript strict compliance (no any, explicit types)
- [ ] Error handling present at every level
- [ ] No hardcoded secrets or credentials
- [ ] No console.log in production code
- [ ] Files under 400 lines, functions under 50 lines
- [ ] Immutable patterns (spread, not mutation)

Testing:
- [ ] Tests exist for new functionality
- [ ] Tests cover error paths, not just happy path
- [ ] Existing tests still pass

Scope:
- [ ] No changes to files outside task scope
- [ ] No new features beyond acceptance criteria
- [ ] No unnecessary refactoring
```

### 4. Run Validation Commands
```
Execute in sequence:
1. pnpm build → must pass
2. pnpm test → must pass (all existing + new tests)
3. pnpm lint → must pass

Record exact output for evidence.
If any command fails → include in verdict as REWORK_REQUIRED.
```

### 5. Submit Review Verdict

#### APPROVED — All criteria met, quality good
```
Call review.submit with:
{
  workflowId: string,
  taskId: string,
  reviewerAgent: "msd-reviewer",
  decision: "APPROVED",
  notes: "All {N} acceptance criteria satisfied. Build and tests pass.",
  criteriaResults: [
    { criterion: "...", status: "PASS" },
    ...
  ]
}
```

#### REWORK_REQUIRED — Some criteria failed or quality issues
```
Call review.submit with:
{
  workflowId: string,
  taskId: string,
  reviewerAgent: "msd-reviewer",
  decision: "REWORK_REQUIRED",
  notes: "{X} of {Y} criteria pass. {summary of issues}",
  criteriaResults: [
    { criterion: "...", status: "PASS" },
    { criterion: "...", status: "FAIL", detail: "Specific issue" },
    ...
  ],
  requiredFixes: [
    "Fix 1: specific instruction",
    "Fix 2: specific instruction"
  ]
}
```

#### BLOCKED — Fundamental issue prevents progress
```
Call review.submit with:
{
  workflowId: string,
  taskId: string,
  reviewerAgent: "msd-reviewer",
  decision: "BLOCKED",
  notes: "Implementation contradicts existing architecture.",
  blocker: "Specific blocker description",
  recommendation: "Suggested resolution"
}
```

### 6. Report and Hand Off
```
After submitting review:

If APPROVED:
  → Report: "Review PASSED. Ready for verification via /msd-verify"

If REWORK_REQUIRED:
  → Report: "Review REWORK. {count} issues found:"
  → List each issue with specific fix instructions
  → Suggest: "Fix issues then re-run /msd-implement"

If BLOCKED:
  → Report: "Review BLOCKED. {blocker description}"
  → Suggest: "Resolve blocker before continuing"
```

## Anti-Patterns to Avoid

- Never APPROVE without checking every acceptance criterion
- Never modify code during review — only report findings
- Never skip running build/tests before verdict
- Never give vague feedback — always specify exact issues and fixes
- Never approve scope-expanded work without flagging it
