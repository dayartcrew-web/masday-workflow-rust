---
name: masday-workflow-run
description: Execute a planned workflow by ID — runs all tasks through the state machine
argument-hint: [workflow-id]
disable-model-invocation: true
allowed-tools: workflow.get workflow.execute workflow.getStatus workflow.addTask
context: fork
---

Execute a Masday workflow that has been planned and is ready to run.

## Input
$ARGUMENTS (workflow ID)

## Steps

1. **Validate** — use `workflow.get` to confirm workflow exists and has tasks
2. **Pre-flight check**:
   - All tasks have valid agent types
   - All skills are registered
   - Dependencies are resolvable (no circular deps)
3. **Execute** using `workflow.execute`
4. **Monitor** using `workflow.getStatus` — track progress
5. **Report**:
   ```
   🚀 Executing: <name>
   🆔 <id>
   
   ✅ #1 [backend] Create user model — DONE (0.8s)
   ✅ #2 [backend] Create API routes — DONE (1.2s)
   ✅ #3 [frontend] Create component — DONE (0.9s) [parallel with #2]
   ❌ #4 [qa] Run tests — FAILED
      Error: Missing import in user.ts
   
   ⚠️ 1 task failed. Use /masday-workflow-fix <id> to retry.
   ```

If all tasks succeed:
```
✅ Workflow Complete
📊 4/4 tasks — DONE
→ /masday-workflow-verify <id> for post-check
→ /masday-workflow-status for overview
```
