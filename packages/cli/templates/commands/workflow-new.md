---
name: masday-workflow-new
description: Create AND execute a new workflow in one shot — uses cached .masday/ context for efficiency while runtime state lives in SQLite
argument-hint: [prompt — describe what you want to build]
disable-model-invocation: true
allowed-tools: workflow.create workflow.addTask workflow.execute workflow.getStatus workflow.get filesystem.read filesystem.list filesystem.write
context: fork
---

End-to-end workflow: create, plan, and execute with SQLite-backed runtime persistence plus local artifact export.

## Input

$ARGUMENTS

## Steps

1. **Ensure `.masday/` exists**:

   ```bash
    mkdir -p .masday/{research,context,plans,notes}
   ```

2. **Load cached context**:
   - `.masday/research/codebase-analysis.md` — skip full scan if fresh
   - `.masday/context/project-context.md` — baseline knowledge

3. **Parse prompt** — understand intent, scope, affected areas

4. **Quick targeted scan** — only read files relevant to the prompt (not entire codebase)

5. **Create workflow** using `workflow.create`

6. **Auto-generate tasks**:
   - Break into atomic steps
   - Agent routing: backend (API/DB), frontend (UI/code), qa (tests), general-purpose (docs)
   - Map skills: `filesystem.*`, `git.*`, `tests.*`, `npm.*`, `code.*`
   - Set dependencies, parallelize where possible

7. **Save plan** → `.masday/plans/<date>-<name>.md`

8. **Execute** using `workflow.execute`

9. **Persist runtime state** in SQLite via the workflow/task stores

10. **Write notes** → `.masday/notes/<date>-<name>.md`

11. **Report**:

    ```
    ✅ Workflow Complete
    🆔 <id>
    📊 Tasks: 4/4
    ⏱️ DONE

    Artifacts:
    - .masday/plans/<date>-<name>.md
    - .masday/notes/<date>-<name>.md

    → /masday-workflow-verify <id>
    ```

## Rules

- Always include verification task
- Max 15 tasks
- Use cache aggressively — don't re-read files already in .masday/
- Treat SQLite as the source of truth for runtime workflow state
