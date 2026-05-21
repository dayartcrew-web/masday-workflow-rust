# Pre-Tool Hook: Workflow Validation
# Triggers before workflow_execute to validate task setup

Before executing any workflow via `workflow_execute`:

1. **Session readiness** — call `policy_check_session_readiness` to verify all required context is loaded
2. **System health** — call `capability_system_readiness` to check database, schema, and dependencies
3. **Context fingerprint** — call `semantic-search_search_context_fingerprint` to validate context sufficiency for the workflow
4. **Validate execution** — call `policy_validate_execution` to confirm the task is allowed to run
5. **Review gate** — call `review_get_latest` to verify the current task has an APPROVED review. Do NOT execute without APPROVED review.
6. **Context freshness** — call `policy_require_context_refresh` to ensure context isn't stale
6. **Verify workflow** — use `workflow_get` to confirm the workflow ID exists
7. Check all tasks have valid:
   - Agent types (system, backend, frontend, qa, general-purpose)
   - Skill names matching registered skills
   - Dependencies point to existing task IDs
8. If any validation fails:
   - Report which checks failed
   - Suggest fixes
   - Do NOT proceed with execution

This ensures workflows are well-formed and the system is ready before burning compute.
