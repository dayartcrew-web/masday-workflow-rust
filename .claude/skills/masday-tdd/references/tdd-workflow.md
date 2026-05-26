# TDD Workflow Reference

## Overview

Test-Driven Development in the masday ecosystem follows a strict RED-GREEN-REFACTOR cycle with full pipeline integration.

## Agent Sequence

```
1. masday-tdd-guide  (RED)       Write failing tests
2. masday-executor   (GREEN)     Implement minimum code to pass
3. masday-tdd-guide  (REFACTOR)  Clean up, coverage check, regression
4. masday-reviewer   (REVIEW)    Code review of tests + implementation
5. masday-verifier   (VERIFY)    Final validation
```

## RED Phase Checklist

- [ ] Read source file or interface definition
- [ ] Identify all public functions/methods
- [ ] Write tests for: happy path, edge cases, error cases
- [ ] Verify ALL new tests FAIL
- [ ] Save progress via `workflow_saveProgress`
- [ ] Store artifact via `memory_store`

## GREEN Phase Checklist

- [ ] Read failing test file first (written by masday-tdd-guide)
- [ ] Implement minimum code to pass tests
- [ ] Run tests after each function implementation
- [ ] NEVER modify test files — fix implementation only
- [ ] Verify ALL tests pass
- [ ] Report: files modified, tests passing, remaining failures

## REFACTOR Phase Checklist

- [ ] Review test quality (no duplicate setup, clear descriptions)
- [ ] Review implementation quality (functions <50 lines, no magic numbers)
- [ ] Refactor incrementally (one change at a time)
- [ ] Run tests after each refactor
- [ ] Verify coverage >= 80% (statements, branches, functions, lines)
- [ ] Run full suite for regressions
- [ ] Save progress with `test_evidence`

## Coverage Thresholds

| Metric | Minimum |
|--------|---------|
| Statements | 80% |
| Branches | 80% |
| Functions | 80% |
| Lines | 80% |

## Test File Naming

| Type | Pattern | Example |
|------|---------|---------|
| Unit test | `<module>.test.ts` | `auth.test.ts` |
| Integration test | `<module>.integration.test.ts` | `auth.integration.test.ts` |
| Bug reproduction | `<module>.bug-<id>.test.ts` | `auth.bug-42.test.ts` |

## Vitest Commands

```bash
# Run single test file
npx vitest run packages/auth/src/auth.test.ts

# Run with coverage
npx vitest run --coverage packages/auth

# Run full suite
pnpm test

# Watch mode (interactive)
npx vitest packages/auth
```

## Pipeline Integration

Every TDD cycle must complete the masday review pipeline:

```
workflow_saveProgress  ->  review_submit  ->  policy_validate_completion  ->  workflow_completeTask  ->  local_sync  ->  memory_store
```

## Available Skills & Agents

| Component | Location | Invocation |
|-----------|----------|------------|
| TDD Skill | `.claude/skills/masday-tdd/SKILL.md` | `/masday-tdd` |
| TDD Agent | `.claude/agents/masday-tdd-guide.md` | `capability_match_agent` dispatch |
| Executor | `.claude/agents/masday-executor.md` | GREEN phase implementation |
| QA Agent | `.claude/agents/masday-qa.md` | Coverage + CI/CD |
| E2E Skill | `.claude/skills/masday-e2e/SKILL.md` | `/masday-e2e` |
