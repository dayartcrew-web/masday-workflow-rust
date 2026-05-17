---
name: masday-workflow-fix
description: Fix a failed or incomplete workflow — re-run failed tasks or add missing ones
argument-hint: [workflow-id]
disable-model-invocation: true
allowed-tools: workflow.get workflow.getStatus workflow.addTask filesystem.read filesystem.list
context: fork
---

Diagnose and fix issues in a failed workflow.

## Input
$ARGUMENTS (workflow ID)

## Steps

1. **Load workflow** using `workflow.get`
2. **Identify failures** — find tasks with error status
3. **Analyze root cause**:
   - Read relevant files (`filesystem.read`)
   - Check dependencies between failed tasks
   - Identify missing inputs or wrong paths
4. **Create fix tasks** using `workflow.addTask`:
   - Fix tasks should reference the original failed task
   - Include corrected inputs
5. **Present fix plan**:
   ```
   🔧 Fix Plan: <name>
   
   Issues found:
   - #4 [qa] Run tests — FAILED: missing import
   
   Fix tasks:
   #5 [backend] Add missing export to index.ts
       → skill: filesystem.write
       → input: { path: "src/index.ts", content: "..." }
   
   #6 [qa] Re-run tests
       → skill: tests.run
       → deps: [#5]
   
   Execute fixes with: /masday-workflow-run <id>
   ```
6. **Ask confirmation** before adding fix tasks
