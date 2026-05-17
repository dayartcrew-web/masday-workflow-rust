---
name: masday-workflow-run
description: Run a complete Masday workflow. Uses SQLite for runtime state and may export human-readable artifacts to .masday/ for continuity.
allowed-tools: Bash workflow.create workflow.execute workflow.getStatus workflow.addTask workflow.list workflow.get filesystem.read filesystem.write
context: fork
---

# Workflow Run

Execute a full Masday workflow with SQLite-backed runtime persistence and local artifact export.

## Pre-flight

1. **Check `.masday/`** exists — if not, init first:

   ```bash
   mkdir -p .masday/{research,context,plans,notes}
   ```

2. **Load cached context** — read `.masday/context/project-context.md` for baseline
3. **Load any existing research** from `.masday/research/` to avoid re-scanning

## Execution

4. **Create workflow** using `workflow.create`
5. **Add tasks** using `workflow.addTask` — use cached context for accurate planning
6. **Execute** using `workflow.execute`
7. **Monitor** using `workflow.getStatus`

## Post-execution

8. **Persist runtime state** in SQLite via the workflow/task stores exposed by the MCP runtime
9. **Update current-workflow.json**:
   ```json
   {
     "lastWorkflowId": "<id>",
     "lastWorkflowState": "<DONE|FAILED>",
     "lastRunAt": "<timestamp>",
     "activeWorkflows": [],
     "completedWorkflows": <n+1>
   }
   ```
10. **Write execution notes** → `.masday/notes/<date>-<workflow-name>.md`:
    - What was done
    - Files changed
    - Issues encountered
    - Next steps

## Agent Routing

- `backend` → filesystem, database, API
- `frontend` → UI, code generation
- `qa` → tests, validation
- `general-purpose` → docs, config

Report final state clearly, distinguishing SQLite runtime state from `.masday/` artifacts.
