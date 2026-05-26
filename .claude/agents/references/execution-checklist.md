# Execution Checklist Reference

Pre-implementation checklist for masday-executor and masday-tdd-guide agents.

## Before Writing Code

- [ ] Read task acceptance criteria from workflow context
- [ ] Read existing code in target files (never guess)
- [ ] Check project conventions in CLAUDE.md
- [ ] Discover existing patterns via `semantic-search_code_search`
- [ ] Identify files to create or modify
- [ ] Create TodoWrite checklist from acceptance criteria

## During Implementation

- [ ] Follow TypeScript strict mode (no `any`)
- [ ] Use ESM `.js` import extensions
- [ ] Functions < 50 lines, files < 400 lines
- [ ] Immutable patterns (spread, no mutation)
- [ ] Zod for runtime validation at system boundaries
- [ ] No `console.log` in production code
- [ ] No hardcoded secrets or credentials

## After Implementation

- [ ] Run `tsc --noEmit` — type check must pass
- [ ] Run relevant tests — all must pass
- [ ] Verify no regressions in full test suite
- [ ] Report: files created/modified, validation results

## TDD GREEN Phase Specific

- [ ] Read failing test file first (written by masday-tdd-guide)
- [ ] Implement minimum code to pass (no over-engineering)
- [ ] Run `npx vitest run <test-file>` after each function
- [ ] NEVER modify test files — fix implementation only
- [ ] Report which tests pass/fail

## Code Quality Gate

| Check | Tool | Threshold |
|-------|------|-----------|
| Type safety | `tsc --noEmit` | 0 errors |
| Tests pass | `npx vitest run` | 100% pass |
| Coverage | `npx vitest run --coverage` | >= 80% |
| No regressions | `pnpm test` | Full suite pass |

## Report Template

```
## Execution Report

### Task: <task-title>
### Agent: masday-executor

### Files Created
- <path>

### Files Modified
- <path>

### Validation
- tsc --noEmit: PASS/FAIL
- Tests: X/Y passing
- Coverage: Z%

### Issues
- <none or list>

### Next Steps
- <recommendations>
```
