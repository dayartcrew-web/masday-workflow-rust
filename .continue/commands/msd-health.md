Quick read-only health check for MSD infrastructure. No side effects.

## Checks

Run these in parallel where possible:

### 1. .msd/ Directory
```
Bash: ls .msd/ 2>/dev/null || echo "MISSING"
```
Report: exists / missing

### 2. MCP Config (Project-Local)
```
Read .mcp.json from project root
Check for mcpServers key with msd-* entries
```
Report: "project .mcp.json" / "missing" / "no msd-* servers"

### 3. MCP Config (Global)
```
Read ~/.claude/settings.json
Check for mcpServers key
Count msd-* prefixed servers
```
Report: "global settings.json (N msd servers)" / "no mcpServers key"

### 4. MCP Servers
```
Call mcp__capability__capability_ping
```
Report: N/N servers up, or list which are down

### 5. Database + pgvector
```
Call mcp__capability__capability_system_readiness
```
Report: connected/disconnected, pgvector enabled/missing, schema sync status

### 6. Active Workflow
```
Call mcp__workflow-orchestrator__workflow_get_active
```
Report: workflow title + status, or "No active workflow"

### 7. Local State
```
Read .msd/state.json
Extract: currentTaskId, workflow status, last sync timestamp
```
Report: synced / stale / missing

### 8. Artifacts
```
Bash: find .msd/ -type f 2>/dev/null | wc -l
Bash: ls .msd/context/codebase/ 2>/dev/null || echo "empty"
Bash: ls .msd/plans/ 2>/dev/null || echo "empty"
Bash: ls .msd/reports/ 2>/dev/null || echo "empty"
```
Report: N files across categories

### 9. Sync Drift (if workflow active)
```
If active workflow:
  Call mcp__workflow-orchestrator__workflow_list_tasks { workflowId }
  Compare task statuses from DB vs .msd/state.json local tasks
  If mismatch → report drift details
  If match → "none"
If no workflow:
  Report: "N/A (no workflow)"
```

### 10. Global Sync
```
Bash: ls .claude/agents/msd-*.md 2>/dev/null | wc -l
Bash: ls ~/.claude/agents/msd-*.md 2>/dev/null | wc -l
Bash: ls .claude/commands/msd-*.md 2>/dev/null | wc -l
Bash: ls ~/.claude/commands/msd-*.md 2>/dev/null | wc -l
```
Report: N agents (local/global), N commands (local/global)

## Output Format

```
╭─────────────────────────────────────╮
│  MSD Health Check                   │
├─────────────────────────────────────┤
│  MCP Config:    {status}            │
│  MCP Servers:   {N/N up}            │
│  Database:      {status}            │
│  pgvector:      {status}            │
│  .msd/:         {exists/missing}    │
│  Workflow:      {title} ({status})  │
│  Local State:   {synced/stale}      │
│  Artifacts:     {N} files           │
│  Sync Drift:    {none/drift}        │
│  Global Sync:   {N} agents, {N} cmds│
│                                    │
│  Status: {HEALTHY / DEGRADED / DOWN}│
╰─────────────────────────────────────╯
```

### Status Determination

- **HEALTHY**: All checks pass
- **DEGRADED**: MCP servers partially up, or minor sync drift, or stale state
- **DOWN**: DB disconnected, or 0 MCP servers up, or critical config missing
