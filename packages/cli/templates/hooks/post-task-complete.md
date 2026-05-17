# Post-Task Hook: Auto-verify
# Triggers after each task completion during workflow execution

After each task completes during `workflow.execute`:

1. Check task result status
2. If task used `filesystem.write`:
   - Verify file exists and is non-empty
   - Check for TypeScript syntax if .ts file
3. If task used `tests.run`:
   - Verify test results are reported
   - Flag any failures immediately
4. If task failed:
   - Log the error context
   - Do NOT auto-retry (let user decide)
   - Continue remaining tasks if dependencies allow

Purpose: Catch issues early instead of discovering them at the end.
