---
name: masday-workflow-verify
description: Verify completed workflow — check outputs, run tests, validate against .masday/ context
allowed-tools: workflow.get workflow.getStatus filesystem.read filesystem.list filesystem.write
context: fork
---

# Workflow Verify

Post-execution validation with local state tracking.

## Steps

1. **Load workflow** using `workflow.get`
2. **Read baseline** from `.masday/context/project-context.md` — what existed before
3. **Validate each task**:
   - Files exist and non-empty (`filesystem.read`)
   - TypeScript compiles (no syntax errors)
   - Exports are correct
4. **Compare before/after** — what changed vs baseline
5. **Write verification** → `.masday/notes/<date>-verify-<id>.md`:
   ```markdown
   # Verification: <workflow-name>
   Date: <date>
   Workflow: <id>
   
   ## Results
   ✅ #1 files created — OK
   ❌ #3 missing export — FAIL
   
   ## Artifacts
   - Created: src/auth/login.ts (142 lines)
   - Modified: src/index.ts (+1 export)
   
   ## Score: 3/4 clean
   ```
6. **Update project context** if all green

Report findings clearly.
