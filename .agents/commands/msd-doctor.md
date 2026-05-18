Deep diagnosis and auto-fix for MSD infrastructure issues.

## Phase 1: Health Check

Run all checks from `/msd-health` first. Collect PASS / WARN / FAIL for each.

```
Run the same 10 checks as msd-health:
1. .msd/ directory
2. MCP config (project-local .mcp.json)
3. MCP config (global ~/.claude/settings.json)
4. MCP servers (capability.ping)
5. Database + pgvector (capability.system_readiness)
6. Active workflow (workflow.getActive)
7. Local state (.msd/state.json)
8. Artifacts (file counts)
9. Sync drift (local vs DB)
10. Global sync (agents + commands)
```

## Phase 2: Deep Diagnosis

For each WARN or FAIL from Phase 1, run additional diagnostics:

### .msd/ Missing
```
Bash: ls -la .msd/ 2>/dev/null
Auto-fix: Call mcp__workflow-orchestrator__local_init({ cwd: process.cwd() })
```

### MCP Config Missing (Project)
```
Read .mcp.json — does it exist? Does it have mcpServers?
If missing: Show minimal .mcp.json template with msd-* servers
Auto-fix: Offer to create .mcp.json from template
```

### MCP Config Missing (Global)
```
Read ~/.claude/settings.json — does it have mcpServers?
If missing: Show how to add mcpServers key
Auto-fix: Cannot auto-fix global settings (requires manual edit)
```

### MCP Server Down
```
Call mcp__capability__capability_ping — which servers respond?
For each down server: identify package name and suggest restart command
Auto-fix: Cannot auto-fix (suggest restart: pnpm dev or restart Claude Code)
```

### DB Disconnected
```
Call mcp__capability__capability_system_readiness
If DB fails:
  Bash: docker ps | grep postgres || echo "Docker not running or no postgres container"
  Read .env — check DATABASE_URL exists
Auto-fix: Suggest docker compose up -d or verify DATABASE_URL
```

### pgvector Missing
```
If system_readiness shows pgvector disabled:
  Suggest: pnpm db:push or run SQL: CREATE EXTENSION IF NOT EXISTS vector;
Auto-fix: Cannot auto-fix (requires DB admin access)
```

### No Active Workflow
```
Call mcp__workflow-orchestrator__workflow_get_active
If none: Suggest /msd-start-work to create one
Auto-fix: Cannot auto-fix (requires user intent)
```

### No Plan Found
```
Call mcp__workflow-orchestrator__workflow_get_plan (if workflow exists)
If no plan: Suggest /msd-plan to create one
Auto-fix: Cannot auto-fix (requires planning)
```

### Stuck Tasks
```
Call mcp__capability__capability_workflow_audit({ maxAgeMinutes: 30 })
Call mcp__workflow-orchestrator__workflow_list_tasks
Flag any task with status "in_progress" that has no recent progress note
Auto-fix: Offer to reset stuck task to "todo" status
```

### Missing Review
```
For each completed task without APPROVED review:
  Call mcp__workflow-orchestrator__review_get_latest
  If no review or last review is not APPROVED:
    Flag: "Task {title} needs review — run /msd-review"
Auto-fix: Cannot auto-fix (requires review agent)
```

### Stale Session
```
Call mcp__workflow-orchestrator__session_get_state
If session flags are stale (contextLoaded=true but .msd/ context missing):
  Auto-fix: Call session.patch_state to reset flags to false
```

### Sync Drift
```
Compare .msd/state.json task statuses vs DB task statuses
If drift detected:
  Auto-fix:
    1. Call mcp__workflow-orchestrator__local_push({ cwd, workflow_id })
    2. Call mcp__workflow-orchestrator__local_sync({ cwd, workflow_id })
    3. Re-read .msd/state.json to verify
```

### Empty Context
```
Bash: ls .msd/context/codebase/ 2>/dev/null
If empty or missing context-pack.md:
  Suggest: Run /msd-continue to re-assemble context pack
Auto-fix: Cannot auto-fix (requires workflow context)
```

### Missing .env
```
Read .env — check DATABASE_URL is set
Read .env.example — compare required vars vs actual
Report which vars are missing
Auto-fix: Cannot auto-fix (contains secrets)
```

### Build Errors
```
Bash: pnpm build 2>&1 | tail -50
If errors: show first 10 lines of error output
Auto-fix: Cannot auto-fix (code issue)
```

### Schema Drift
```
Bash: pnpm db:push --dry-run 2>&1 | tail -20
If pending changes: report what would change
Auto-fix: Offer to run pnpm db:push
```

### Orphaned Branches
```
Call mcp__workflow-orchestrator__workflow_list_parallel_branches (if workflow active)
Flag branches with status != "completed" that have no recent activity
Auto-fix: Offer to mark orphaned branches as completed
```

## Phase 3: Auto-Fix (with confirmation)

For each fixable issue found in Phase 2:

```
1. Show issue summary:
   "{ISSUE}: {description}"
   "Proposed fix: {action}"

2. Ask user: "Fix this? (Y/n)"

3. If yes, apply fix and verify:
   - Run the fix action
   - Re-run the diagnostic check
   - Report: FIXED / FAILED TO FIX

4. If no, mark as MANUAL in report
```

## Phase 4: Report

```
╭──────────────────────────────────────╮
│  MSD Doctor Report                   │
├──────────────────────────────────────┤
│  Issues Found:    {N}                │
│  Auto-fixed:      {N}                │
│  Needs Manual:    {N}                │
│                                      │
│  ✓ FIXED: {issue} → {fix}           │
│  ✓ FIXED: {issue} → {fix}           │
│  ⚠ MANUAL: {issue}                  │
│    {suggestion}                      │
╰──────────────────────────────────────╯
```

## Error Handling

| Error | Action |
|-------|--------|
| capability.ping times out | Report all servers as DOWN, proceed with other checks |
| DB completely unreachable | Skip all DB-dependent checks, report DB as DOWN |
| .msd/state.json corrupt | Report as corrupt, offer local.init to regenerate |
| Permission denied on files | Report path, suggest chmod or admin access |
