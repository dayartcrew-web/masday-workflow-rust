---
name: masday-workflow-status
description: Check workflow status from MCP runtime state and local .masday/ context artifacts
allowed-tools: workflow.list workflow.get filesystem.read filesystem.write
---

# Workflow Status

Show status from the MCP runtime store and any local `.masday/` context artifacts.

## Steps

1. **Query MCP** using `workflow.list` and `workflow.get`
2. **Read local context** from `.masday/context/current-workflow.json`
3. **Prefer MCP/SQLite as the source of truth** if local artifacts differ from runtime state
4. **Merge and display**:

   ```
   📊 Masday Workflows — <project-name>

   🟢 abc123  Add auth module    DONE      4/4 ✅  (2h ago)
   🔵 def456  Refactor API       EXECUTE   2/5     (active)
   ⚪ ghi789  User tests         PLAN      0/3     (draft)

    📁 Local artifacts: .masday/
   - Research: 3 files (last: 30min ago)
   - Plans: 2 files
   - Notes: 5 files

   💡 → /masday-workflow-run def456
   ```

5. **Update local context summary** → `.masday/context/current-workflow.json`
