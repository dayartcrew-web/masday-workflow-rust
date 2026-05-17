---
name: masday-workflow-status
description: Show all workflows and their current state — active, completed, and pending
disable-model-invocation: true
allowed-tools: workflow.list workflow.get workflow.getStatus
---

Overview of all Masday workflows.

## Steps

1. **List all** using `workflow.list`
2. **Get details** for each using `workflow.get`
3. **Display summary**:
   ```
   📊 Masday Workflows
   
   🟢 abc123  Add auth module          DONE      4/4 tasks ✅
   🔵 def456  Refactor API layer       EXECUTE   2/5 tasks
   🟡 ghi789  Add user tests           PLAN      0/3 tasks
   ⚪ jkl012  Database migration       INIT      no tasks
   
   💡 1 active | 1 complete | 2 pending
   
   Commands:
   → /masday-workflow-run def456        — Resume active
   → /masday-workflow-run ghi789        — Start planned
   → /masday-workflow-verify abc123     — Verify complete
   ```
