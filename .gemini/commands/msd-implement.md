Implement the current task strictly within scope.

## Purpose

Execute implementation for the active task. This command enforces scope discipline: implement ONLY what the task requires, nothing more.

## Pre-conditions (MANDATORY — execute these tool calls in order)

```
Step 0a — Init .msd/:
  mcp__masday__local_init({ cwd: process.cwd() })

Step 0b — Get active workflow:
  const activeWf = await mcp__masday__workflow_get_active({ cwd: process.cwd() })
  If none: STOP — "No active workflow. Run /msd-start-work first."
  const workflowId = activeWf.id;

Step 0c — Get plan:
  const plan = await mcp__masday__workflow_get_plan({ workflow_id: workflowId })
  If no plan: STOP — "No plan found. Run /msd-plan first."
  const planId = plan.plan.id;

Step 0d — Get current task:
  const currentTask = await mcp__masday__workflow_get_current_task({ workflow_id: workflowId })
  If none: STOP — "No current task. Run /msd-continue first."
  const taskId = currentTask.id;
  const title = currentTask.title;
  const acceptanceCriteria = currentTask.acceptanceCriteria;
  const requiredContext = currentTask.requiredContext;

Step 0e — Load context pack:
  const ctxPack = await mcp__semantic-search__search_hybrid_context_pack({
    workflow_id: workflowId,
    plan_id: planId,
    task_id: taskId,
    cwd: process.cwd()
  })

Step 0f — Start task:
  await mcp__workflow-orchestrator__workflow_start_task({
    workflow_id: workflowId,
    task_id: taskId
  })
```

If any step fails → STOP and report what's missing.

## Steps

### 1. Lock Scope
```
Read the current task's:
- Title (what to build)
- Acceptance criteria (how to verify)
- Required context (what to know)

Write down in your thinking:
"I will implement ONLY {title}. I will NOT add {things not in scope}."
This prevents scope creep during implementation.
```

### 2. Check Existing Code
```
For each file the task expects to modify:

1. Glob for the file → does it exist?
2. If exists → Read it fully before changing anything
3. If not → check if a similar pattern exists elsewhere
4. Grep for related patterns to understand conventions

Record:
- Files that exist and will be modified
- Files that need to be created
- Patterns to follow from existing code
```

### 3. TDD Cycle (MANDATORY)

This step enforces Test-Driven Development. Do NOT skip to implementation without tests.

#### 3a. RED — Write Tests First
```
Before writing ANY implementation code:

1. Identify test locations based on project structure:
   - Python → tests/test_{feature}.py or src/tests/
   - TypeScript/JS → src/__tests__/{feature}.test.ts
   - Go → {feature}_test.go (same package)
   - Rust → #[cfg(test)] mod tests in source or tests/ dir
   - Java/Kotlin → src/test/java/.../{Feature}Test.java
   - PHP → tests/Feature/{Feature}Test.php
   - Follow whatever convention already exists in the project

2. Write tests that cover:
   - Happy path for each acceptance criterion
   - Error/edge cases from acceptance criteria
   - Input validation (if applicable)

3. Run tests — they MUST FAIL:
   Use the project's test runner (pnpm test, pytest, go test, cargo test, php unit test, etc.)

4. Confirm: tests fail because implementation doesn't exist yet.
   If tests pass without implementation → tests are wrong, rewrite them.

DO NOT proceed to 3b until tests are written and failing.
```

#### 3b. GREEN — Minimal Implementation
```
Write the MINIMUM code needed to make all tests pass:

Follow msd-mcp code conventions:
- TypeScript strict mode — no `any`, explicit return types on exports
- Immutable patterns — spread operators, never mutate
- Files under 400 lines, functions under 50 lines
- Zod validation for all MCP tool inputs
- Error handling at every level, never silently swallow

For NEW MCP tools:
1. Add business logic function in packages/workflow-engine/src/
2. Add Zod input schema in shared-types
3. Wire up MCP tool handler in the relevant app

For EXISTING code changes:
1. Read the current implementation fully
2. Make minimal, targeted changes
3. Preserve existing behavior unless explicitly changing it

After implementation:
1. Run tests — they MUST ALL PASS:
   pnpm test -- --run
2. If any test fails → fix CODE (not tests), re-run
3. Do NOT add extra features beyond what tests require
```

#### 3c. REFACTOR — Clean Up
```
While keeping tests green:

1. Remove duplication
2. Improve naming if unclear
3. Extract helpers for repeated patterns
4. Ensure files stay under 400 lines

After each refactor:
  pnpm test -- --run → must still pass

DO NOT change behavior during refactor — only structure.
```

### 4. Validate After Implementation
```
Run in sequence:
1. pnpm build → must pass with zero errors
2. pnpm test → all tests pass (including pre-existing)
3. pnpm lint → no type errors or warnings

If any fails:
  → Fix the issue immediately
  → Re-run the failed step
  → Do NOT proceed until all pass
```

### 5. Save Progress
```
mcp__workflow-orchestrator__workflow_save_progress({
  workflow_id: workflowId,
  task_id: taskId,
  agent_name: "msd-executor",
  progress_note: "TDD: Implemented {what} in {files}. RED→GREEN→REFACTOR complete. Build/tests/lint all pass.",
  evidence: [
    "Tests written: {test files}",
    "Modified: {file list}",
    "Created: {file list}",
    "Build: PASS",
    "Tests: PASS ({count} tests)",
    "Lint: PASS"
  ]
})

Then save implementation report:
Write to: .msd/reports/implement-{task-slug}.md
Content:
  ## Implementation Report
  Task: {title}
  TDD: RED → GREEN → REFACTOR
  Tests: {test files}
  Files: {modified + created}
  Notes: {key decisions}
```

### 6. Hand Off to Review
```
DO NOT mark the task complete.
Instead, report:
"Implementation complete (TDD enforced). Ready for review via /msd-review"

The reviewer will validate against acceptance criteria.
```

## Scope Guard Rails

These are explicitly OUT OF SCOPE during implementation:
- Adding features not in the acceptance criteria
- Refactoring code not touched by this task
- Adding documentation not requested
- Changing configuration unrelated to the task
- Adding tests for code not modified by this task

If you find something that SHOULD be fixed but is out of scope:
→ Note it in progress as "Observed but out of scope: {issue}"
→ Do NOT fix it during this task

## Error Recovery

| Error | Action |
|-------|--------|
| Build fails | Read error, fix TypeScript issues, re-build |
| Tests fail | Read error, fix CODE (not tests), re-run |
| Lint fails | Fix type issues, remove console.log |
| Pre-conditions missing | Stop and report, don't improvise |
| File not found | Re-check path, use Glob to locate |

## Output

```
Task: {title}
Files Modified: {list}
Files Created: {list}
Build: PASS/FAIL
Tests: {count} passing
Lint: PASS/FAIL
Progress Saved: Yes
Next Step: /msd-review
```
