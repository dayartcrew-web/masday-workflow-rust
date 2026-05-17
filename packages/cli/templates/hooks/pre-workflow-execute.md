# Pre-Tool Hook: Workflow Validation
# Triggers before workflow.execute to validate task setup

Before executing any workflow via `workflow.execute`:

1. Verify the workflow ID exists using `workflow.get`
2. Check all tasks have valid:
   - Agent types (system, backend, frontend, qa, general-purpose)
   - Skill names matching registered skills
   - Dependencies point to existing task IDs
3. If validation fails:
   - Report which tasks have issues
   - Suggest fixes
   - Do NOT proceed with execution

This ensures workflows are well-formed before burning compute.
