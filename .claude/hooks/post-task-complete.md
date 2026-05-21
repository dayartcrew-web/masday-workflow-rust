# Post-Task Hook: Auto-verify and Store Progress
# Triggers after each task completion during workflow execution

After each task completes during `workflow_execute`:

1. **Validate completion** — call `policy_validate_completion` to verify the task meets acceptance criteria
2. **Detect scope drift** — call `policy_detect_scope_drift` to check if output has drifted from the original task scope
3. **Store progress** — call `memory_store` to save the task outcome (type: "artifact", summary of what was done)
4. **Recall context** — call `memory_recall_by_task` to load any relevant prior context for the next task
5. If task used `filesystem_write`:
   - Verify file exists using `filesystem_stat`
   - Check for TypeScript syntax if .ts file
6. If task used `tests_run`:
   - Verify test results are reported
   - Flag any failures immediately
   - Verify test results are from real `pnpm test` execution (tests_run runs real test suite via execSync)
7. If task failed:
   - Log the error context via `memory_store` (type: "blocker")
   - Do NOT auto-retry (let user decide)
   - Continue remaining tasks if dependencies allow

Purpose: Catch issues early, persist progress, and prevent scope drift.
