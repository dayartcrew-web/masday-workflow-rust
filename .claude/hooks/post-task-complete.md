# Post-Task Hook: Auto-verify and Store Progress
# Triggers after each task completion during workflow execution

After each task completes during `workflow.execute`:

1. **Validate completion** — call `policy.validate_completion` to verify the task meets acceptance criteria
2. **Detect scope drift** — call `policy.detect_scope_drift` to check if output has drifted from the original task scope
3. **Store progress** — call `memory.store` to save the task outcome (type: "artifact", summary of what was done)
4. **Recall context** — call `memory.recall_by_task` to load any relevant prior context for the next task
5. If task used `filesystem.write`:
   - Verify file exists using `filesystem.stat`
   - Check for TypeScript syntax if .ts file
6. If task used `tests.run`:
   - Verify test results are reported
   - Flag any failures immediately
7. If task failed:
   - Log the error context via `memory.store` (type: "blocker")
   - Do NOT auto-retry (let user decide)
   - Continue remaining tasks if dependencies allow

Purpose: Catch issues early, persist progress, and prevent scope drift.
