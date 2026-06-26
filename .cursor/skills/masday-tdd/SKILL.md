---
name: masday-tdd
description: Test-Driven Development for masday — strict RED-GREEN-REFACTOR cycle with Vitest, coverage enforcement (80%+), and masday review pipeline integration
disable-model-invocation: false
allowed-tools: Read Write Edit Bash Grep Glob TodoWrite mcp__masday__workflow_saveProgress mcp__masday__review_submit mcp__masday__policy_validate_completion mcp__masday__workflow_completeTask mcp__masday__memory_store mcp__masday__local_sync mcp__masday__tests_run mcp__masday__semantic-search_code_search
context: inline
---

# Masday TDD — Test-Driven Development

Strict RED-GREEN-REFACTOR cycle using Vitest. Writes tests first, enforces 80%+ coverage, integrates with masday review pipeline.

## When to Use

- Before writing new feature code
- Before fixing a bug (write test that reproduces the bug first)
- Before refactoring (ensure existing tests cover the code)
- When explicitly invoked via `/masday-tdd`

## Prerequisites

Before starting, gather context:

```
1. Read vitest.config.ts or vitest.config.js for test configuration
2. Glob for existing test patterns: **/*.test.ts, **/*.spec.ts
3. Check project conventions in CLAUDE.md
4. If in workflow: mcp__masday__workflow_getActive + getCurrentTask for task context
```

## The RED-GREEN-REFACTOR Cycle

This skill enforces **mandatory step completion**. Each step must be completed before proceeding. Do not skip steps.

### Phase 1: RED — Write Failing Tests

**Goal:** Write tests that define expected behavior. ALL new tests must FAIL.

**Step 1: Identify what to test**

```
- Read the source file or interface definition
- List all public functions/methods
- Identify: happy path, edge cases, error cases, boundary values
- Check for integration points (DB, API, external deps)
```

**Step 2: Discover existing test patterns**

```
mcp__masday__semantic-search_code_search({ query: "unit test example describe it expect" })
Glob({ pattern: "packages/*/src/**/*.test.ts" })
```

**Step 3: Write test file**

Test file naming: `<module>.test.ts` (unit) or `<module>.integration.test.ts` (integration)

Test structure:

```typescript
import { describe, it, expect, beforeEach, vi } from 'vitest';

describe('<ModuleName>', () => {
  beforeEach(() => {
    // Reset state between tests
  });

  describe('<functionName>', () => {
    it('should <expected behavior> when <condition>', () => {
      // Arrange
      const input = ...;
      // Act
      const result = functionName(input);
      // Assert
      expect(result).toEqual(expected);
    });

    it('should throw <error> when <invalid input>', () => {
      expect(() => functionName(invalidInput)).toThrow();
    });

    it('should handle <edge case>', () => {
      const result = functionName(edgeCaseInput);
      expect(result).toBeDefined();
    });
  });
});
```

**Step 4: Verify RED — tests must FAIL**

```
mcp__masday__tests_run({ pattern: "<test-file>" })
```

All new tests must fail. If any pass unexpectedly, check if the function already exists or the test is not properly isolated.

**Step 5: Save progress**

```
mcp__masday__workflow_saveProgress({
  workflow_id: "<id>",
  task_id: "<id>",
  agent_name: "<agent>",
  progress_note: "RED phase complete: N test cases written, all failing",
  evidence: ["path/to/test-file.test.ts"]
})
```

### Phase 2: GREEN — Implement Minimum Code

**Goal:** Write the minimum code to make all tests pass. Do NOT over-engineer.

**Step 1: Implement**

Read the source file (or create it). Write the minimum implementation:
- Make tests pass one at a time
- Run tests after each function implementation
- Keep code simple — no premature abstractions
- Follow project conventions (TypeScript strict, ESM .js imports, no any)

**Step 2: Verify GREEN — all tests must PASS**

```
mcp__masday__tests_run({ pattern: "<test-file>" })
```

If tests fail, fix the implementation, NOT the tests.

**Step 3: Save progress**

```
mcp__masday__workflow_saveProgress({
  workflow_id: "<id>",
  task_id: "<id>",
  agent_name: "<agent>",
  progress_note: "GREEN phase complete: all N tests passing",
  evidence: ["path/to/source-file.ts", "path/to/test-file.test.ts"]
})
```

### Phase 3: REFACTOR — Clean Up

**Goal:** Improve code quality while keeping tests green.

Check for:
- Duplicate code — extract shared functions
- Long functions (>50 lines) — split into smaller
- Magic numbers — extract to constants
- Unclear names — rename for clarity
- Missing error handling — add explicit handling
- Dead code — remove

Refactor incrementally — ONE change at a time, run tests after each.

### Phase 4: Coverage Check

Run with coverage:

```bash
npx vitest run --coverage packages/<module>
```

Enforce 80% minimum across statements, branches, functions, and lines.

If below threshold: identify uncovered lines, write additional tests, re-run.

**GATE**: Pre-completion checkpoint. Verify all prior steps are fully complete.

### Phase 5: Regression Check

Run the full test suite to ensure no regressions:

```bash
pnpm test
```

## Test Quality Checklist

Before marking TDD complete:

- [ ] Tests follow naming: `it should <behavior> when <condition>`
- [ ] Each test runs independently (no shared mutable state)
- [ ] Tests are deterministic (no flaky tests)
- [ ] External dependencies mocked, not the unit under test
- [ ] Each test has at least one explicit assertion
- [ ] Happy path, edge cases, and error cases covered
- [ ] Coverage 80%+ for the module
- [ ] Full test suite passes (no regressions)

## Test File Templates

### Unit Test

```typescript
import { describe, it, expect, beforeEach, vi } from 'vitest';
import { functionName } from './module.js';

describe('ModuleName', () => {
  describe('functionName', () => {
    it('should return expected value for valid input', () => {
      const result = functionName({ key: 'value' });
      expect(result).toEqual({ expected: 'output' });
    });

    it('should throw on invalid input', () => {
      expect(() => functionName(null as any)).toThrow();
    });

    it('should handle empty input', () => {
      const result = functionName({});
      expect(result).toBeDefined();
    });
  });
});
```

### Bug Reproduction Test

```typescript
describe('Bug #XXX: <description>', () => {
  it('should reproduce the bug', () => {
    const input = { /* bug-specific input */ };
    const result = buggyFunction(input);
    expect(result).toEqual(expectedCorrectOutput);
  });
});
```

## Error Recovery

| Situation | Recovery |
|-----------|----------|
| Tests fail in RED phase | Expected — verify they fail for the right reasons |
| Tests fail in GREEN phase | Fix implementation, NOT tests |
| Tests fail in REFACTOR phase | Revert the refactor, try different approach |
| Coverage below 80% | Write more tests for uncovered paths |
| Flaky test detected | Isolate test, remove timing deps, mock external state |
| Full suite has regressions | Identify which test broke, fix implementation |

## Mandatory Review Pipeline

When this skill completes work on a workflow task:

`
STEP 1: Save progress
  mcp__masday__workflow_saveProgress({
    workflow_id, task_id, agent_name, progress_note,
    evidence: ["<test files>", "<source files>"],
    test_evidence: { testFiles: ["<paths>"], testsPassed: true, coveragePercent: <num> }
  })

STEP 2: Submit for review
  mcp__masday__review_submit({ workflow_id, task_id, reviewer_agent: "masday-reviewer", decision, notes, gaps })

STEP 3: If REWORK_REQUIRED — fix and loop (max 2 attempts)

STEP 4: If APPROVED — validate completion
  mcp__masday__policy_validate_completion({ workflow_id, task_id })

STEP 5: Complete task
  mcp__masday__workflow_completeTask({ workflow_id, task_id })

STEP 6: Sync local state
  mcp__masday__local_sync({ cwd, workflow_id })

STEP 7: Persist findings
  mcp__masday__memory_store({ memory_type: "artifact", summary, content, created_by_agent, tags: ["tdd", "tests"] })
`

### Never
- Never call workflow_completeTask without review_submit (APPROVED)
- Never skip policy_validate_completion before completion
- Never skip local_sync after completing a task
- Never claim done without saving progress to PostgreSQL
- Never modify tests to make them pass in GREEN phase
- Never write implementation before tests (RED must come first)
- Never accept coverage below 80%
- Never skip the full regression suite check
- Never skip any step — complete each step before proceeding
- Never bypass a GATE marker without validating prior steps
- Never claim completion without executing all steps in order
