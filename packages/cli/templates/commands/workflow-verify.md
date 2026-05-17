---
name: masday-workflow-verify
description: Verify a completed workflow — check outputs, run tests, validate results
argument-hint: [workflow-id]
disable-model-invocation: true
allowed-tools: workflow.get workflow.getStatus filesystem.read filesystem.list tests.run
context: fork
---

Post-execution verification for a Masday workflow.

## Input
$ARGUMENTS (workflow ID)

## Steps

1. **Load workflow** using `workflow.get`
2. **Check each task result**:
   - Status: completed / failed
   - Output data present
3. **Validate artifacts**:
   - Files written exist and are non-empty (`filesystem.read`)
   - No TypeScript syntax errors
   - Test results are green (`tests.run`)
4. **Report**:
   ```
   🔍 Verification: <name>
   
   ✅ #1 Create user model — file exists, 142 lines
   ✅ #2 Create API routes — file exists, exports correct
   ✅ #3 Tests — 12/12 passing
   ⚠️ #4 Missing index.ts export for user model
   
   Score: 3/4 clean
   
   Fix: /masday-workflow-fix <id>
   ```
