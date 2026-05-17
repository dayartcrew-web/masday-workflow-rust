---
name: masday-project-sync
description: Refresh .masday/ data — re-analyze codebase, update context, sync workflow state from MCP SQLite
disable-model-invocation: true
allowed-tools: Bash filesystem.read filesystem.list filesystem.write workflow.list workflow.get
---

Sync Masday project data — refresh analysis and state.

## Steps

1. **Check `.masday/` exists** — if not, run `/masday-project-init` first

2. **Re-analyze codebase** → update `.masday/research/codebase-analysis.md`:
   - Diff what changed since last analysis
   - Update structure, new files, changed dependencies
   - Keep changelog at bottom of file

3. **Sync workflow summaries** from MCP:
   - `workflow.list` → get all workflows
   - For each: `workflow.get` → summarize into `.masday/context/` or `.masday/notes/` if local export is useful
   - Summary → `.masday/context/current-workflow.json`

4. **Update project context** → `.masday/context/project-context.md`:
   - Refresh development status
   - Update completed features / pending work
   - Add any new architecture decisions

5. **Report changes**:

   ```
   🔄 Masday Project Synced

    Updated:
    - research/codebase-analysis.md (+15 lines — new auth module)
    - context/project-context.md (refreshed)
    - context/current-workflow.json (workflow summary refreshed, 1 active workflow)
   ```

## When to use

- After significant code changes
- Before starting a new workflow (fresh context)
- After resuming work on a project
