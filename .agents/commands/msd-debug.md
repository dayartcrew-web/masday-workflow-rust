Debug the current task by investigating errors, tracing root causes, and proposing fixes.

## Purpose

Systematic debugging command that investigates failures, test errors, build errors, or unexpected behavior in the current task. Follows a scientific method approach: observe, hypothesize, test, fix.

## When to Use

- Build fails and error messages are unclear
- Tests fail and root cause is not obvious
- Implementation produces unexpected behavior
- MCP tool calls return errors
- Database queries produce wrong results

## Steps

### 1. Identify the Problem
```
Load current context:
1. workflow.getActive → workflowId
2. workflow.getCurrentTask → taskId, title
3. memory.recall_recent → recent progress and errors

Ask: What exactly is failing?

Categorize the issue:
| Category | Symptoms |
|----------|----------|
| Build error | TypeScript compilation fails, type errors |
| Test failure | Assert fails, timeout, wrong output |
| Runtime error | MCP tool returns error, crash, exception |
| Logic error | Code runs but produces wrong result |
| Integration error | Components don't work together |
| Database error | Prisma query fails, connection issue |

Record the specific error message and context.
```

### 2. Reproduce the Issue
```
Run the failing command to see the exact error:

Build errors:
  pnpm build 2>&1 | head -50

Test failures:
  pnpm test 2>&1 | head -50

Lint errors:
  pnpm lint 2>&1 | head -50

Capture:
- Exact error message
- File and line number
- Stack trace (if available)
- Error code (if available)
```

### 3. Trace Root Cause
```
Starting from the error, work backward:

For build/type errors:
1. Read the file at the error location
2. Check the type definition vs usage
3. Check imports — are they correct?
4. Check if shared-types were updated after code change

For test failures:
1. Read the failing test
2. Identify what assertion failed
3. Run just that test file: pnpm test {file}
4. Read the implementation being tested
5. Check if recent changes broke the assumption

For runtime/MCP errors:
1. Read the tool handler code
2. Check input validation (Zod schema)
3. Check database connection and queries
4. Check environment variables (.env)
5. Check if Prisma client is generated: pnpm db:generate

For database errors:
1. Check Docker is running: docker ps
2. Check DATABASE_URL in .env
3. Check if schema is pushed: pnpm db:push
4. Check if migration is needed
```

### 4. Form Hypothesis
```
Based on root cause tracing, state your hypothesis:

"The error occurs because {specific reason}.
 The fix should {specific action}."

Example:
"The build fails because WorkflowStatus was renamed to
WorkflowState in shared-types but the workflow engine
still imports WorkflowStatus. Fix: update the import."

Be specific. Don't say "something is wrong with types."
```

### 5. Apply Targeted Fix
```
Apply the minimum fix needed:

Rules:
- Fix the ROOT CAUSE, not the symptom
- Make the SMALLEST change that fixes it
- Do NOT refactor surrounding code
- Do NOT add features while fixing
- Do NOT modify tests to pass (fix the code)

After fix:
1. Run the failing command → must pass now
2. Run pnpm build → must pass
3. Run pnpm test → all tests must pass
4. Run pnpm lint → must pass
```

### 6. Save Debug Progress
```
Call workflow.saveProgress with:
{
  workflowId,
  taskId,
  agentName: "msd-debugger",
  progressNote: "Debugged {issue}. Root cause: {cause}. Fix: {what was changed}.",
  evidence: [
    "Error was: {error message}",
    "Root cause: {explanation}",
    "Fix: {file}:{line} — {change}",
    "Build: PASS",
    "Tests: PASS",
    "Lint: PASS"
  ]
}
```

### 7. Report Results
```
Debug Summary:
  Issue: {what was failing}
  Root Cause: {why it was failing}
  Fix: {what was changed, where}
  Verification: build/tests/lint all PASS

  Next Step: /msd-review to validate the fix
```

## Common Debug Patterns

### TypeScript Build Errors
```
"Cannot find module X" → Check import path, run pnpm build for dependency
"Type X is not assignable to Y" → Read both types, find mismatch
"Property X does not exist on Y" → Check if type needs updating
"Module resolution error" → Check tsconfig paths, package.json exports
```

### Test Failures
```
"Expected X, received Y" → Read implementation, check logic
"Timeout exceeded" → Check for async/await issues, infinite loops
"Cannot read property of undefined" → Add null check, check data flow
"Mock not called" → Check mock setup, check import paths
```

### MCP Tool Errors
```
"Tool not found" → Check tool registration in server setup
"Invalid input" → Check Zod schema, validate input shape
"Database error" → Check Prisma client, schema, connection
"Internal error" → Check server logs, add error logging
```

### Prisma/Database Errors
```
"Prisma Client not generated" → Run pnpm db:generate
"Table does not exist" → Run pnpm db:push
"Connection refused" → Check Docker, DATABASE_URL
"Unique constraint failed" → Check seed data, check for duplicates
```

## Anti-Patterns

- Never fix symptoms instead of root cause
- Never modify tests to make them pass
- Never refactor code while debugging
- Never add new features while fixing bugs
- Never skip running build/tests/lint after fix
