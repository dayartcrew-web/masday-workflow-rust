---
name: masday-doctor
description: >
  Diagnose and fix Masday installation and configuration issues. Checks SQLite database,
  PostgreSQL connectivity, Redis, API server, MCP binary, directory structure, stale workflows,
  and MCP config. Provides auto-fix capabilities for common issues. Use when the user says
  "doctor", "diagnose", "check health", "fix issues", or "masday not working".
allowed-tools:
  - filesystem_stat
  - filesystem_read_file
  - filesystem_write_file
  - filesystem_list_directory
  - local_sync
  - workflow_list
  - workflow_get
  - capability_check
  - memory_store
---

# Masday Doctor

Diagnose and automatically fix common Masday installation and configuration issues.

## When to Use

Use this skill when:
- User runs `/masday-doctor` command
- User asks to "diagnose masday issues" or "check masday health"
- User reports "masday not working" or "something is broken"
- User asks to "fix masday" or "repair masday installation"
- User encounters MCP connection errors
- User encounters database errors
- User encounters workflow state issues

## Steps

This skill enforces **mandatory step completion**. Each step must be completed before proceeding. Do not skip steps.


1. **Check SQLite database**
   - Verify `~/.masday/data.db` exists
   - If missing, create directory structure and initialize database
   - Check database has all required tables (workflows, tasks, plans, memories, etc.)
   - Verify database is readable (try a simple query)

2. **Check PostgreSQL connectivity (if configured)**
   - Check if PostgreSQL is configured (port 54341)
   - Test connectivity to PostgreSQL
   - Verify database exists and has required tables
   - Report connection status

3. **Check Redis connectivity (if configured)**
   - Check if Redis is configured (port 63791)
   - Test connectivity to Redis
   - Report connection status

4. **Check API server health**
   - Try to connect to `http://localhost:30101/api/health`
   - Report server status (healthy, down, or port not open)
   - If down, offer to start the server

5. **Check MCP server binary**
   - Verify `masday-mcp` binary exists in `~/.masday/bin/`
   - Check binary is executable
   - If missing, offer to build/install the binary

6. **Validate .masday/ directory structure**
   - Check `~/.masday/` exists and has proper structure:
     - `bin/` directory for binaries
     - `data.db` for SQLite database
     - `context/` for context packs
     - `state/` for state backups
   - Create missing directories if needed

7. **Check for stale workflows**
   - Query for workflows stuck in RUNNING status for >30 minutes
   - Query for workflows stuck in EXECUTE status for >1 hour
   - Offer to reset stuck workflows to PAUSED status

8. **Check .mcp.json config validity**
   - Locate `.mcp.json` (project level or global `~/.claude/.mcp.json`)
   - Verify binary path points to valid `masday-mcp` binary
   - Check config has required fields
   - Fix binary path if incorrect

9. **Generate health report**
   ```
   === Masday Doctor Report ===

   SQLite Database: ✓ OK (16 tables)
   PostgreSQL: ✓ OK (port 54341, 16 tables)
   Redis: ✓ OK (port 63791)
   API Server: ✓ OK (http://localhost:30101)
   MCP Binary: ✓ OK (~/.masday/bin/masday-mcp)
   Directory Structure: ✓ OK
   Stale Workflows: ⚠ 2 stuck workflows found
   MCP Config: ✓ OK

   Overall: DEGRADED (2 stale workflows)
   ```

10. **Apply auto-fixes (if --fix mode)**
    - Create missing `~/.masday/` directory structure
    - Create missing `~/.masday/data.db` with proper schema
    - Fix `.mcp.json` binary path
    - Kill stale MCP processes
    - Reset stuck workflows to PAUSED

**GATE**: Verify steps 1-10 are complete before proceeding.

## Auto-Fix Capabilities

### SQLite Database Issues
- Missing database: Create `~/.masday/data.db` with full schema
- Corrupted database: Warn user, recommend manual intervention
- Missing tables: Run schema migration to add missing tables

### PostgreSQL Issues
- Container not running: Offer to start with `docker start masday-postgres`
- Connection failed: Check credentials in config, offer to reconfigure
- Missing tables: Offer to run migrations

### Directory Structure Issues
- Missing `~/.masday/`: Create full directory structure
- Missing subdirectories: Create `bin/`, `context/`, `state/`, `research/`, etc.

### MCP Binary Issues
- Binary not found: Offer to build from source or download release
- Binary not executable: Fix permissions with `chmod +x`
- Wrong binary path: Update `.mcp.json` config

### Stale Workflow Issues
- Stuck in RUNNING: Reset to PAUSED
- Stuck in EXECUTE: Reset to PAUSED
- Orphaned tasks: Mark as FAILED

### MCP Config Issues
- Invalid JSON: Fix syntax errors
- Wrong binary path: Update to correct path
- Missing required fields: Add defaults

## Never

- Never skip any step — complete each step before proceeding
- Never bypass a GATE marker without validating prior steps
- Never claim completion without executing all steps in order

- Never run destructive operations without user confirmation
- Never delete workflows without confirming project ownership
- Never modify database without proper backup
- Never kill processes without identifying them first
- Never skip reporting issues to the user

## Fix Safety Rules

When applying auto-fixes:

1. **Always ask first** — Describe the fix and get user consent
2. **Backup before modifying** — Copy files before changing them
3. **Graceful degradation** — If a fix fails, report the error and continue
4. **No data loss** — Never delete data without explicit confirmation
5. **Idempotent operations** — Fixes should be safe to run multiple times

## Output Format

After completing all steps, output a structured report:

```
=== Masday Doctor Report ===

Component Status:
  SQLite Database: ✓ OK (16 tables, 47 memories)
  PostgreSQL: — SKIP (not configured)
  Redis: — SKIP (not configured)
  API Server: ✗ FAIL (connection refused)
  MCP Binary: ✓ OK (~/.masday/bin/masday-mcp)
  Directory Structure: ✓ OK
  Stale Workflows: ✓ OK (no stuck workflows)
  MCP Config: ✓ OK

Issues Found: 1
  [FAIL] API Server: Connection refused at http://localhost:30101

Fixes Applied: 0
  (no fixes needed or user declined)

Overall Status: UNHEALTHY
Next Steps:
  1. Start API server: masday serve
  2. Check server logs for startup errors
```

## Memory Storage

After completing doctor checks and fixes:

```javascript
memory_store({
  memory_type: "fact",
  summary: "Masday system health check completed",
  content: JSON.stringify({
    timestamp: new Date().toISOString(),
    overall_status: "HEALTHY|DEGRADED|UNHEALTHY",
    issues_found: ["issue1", "issue2"],
    fixes_applied: ["fix1", "fix2"],
    component_status: {
      sqlite: "OK|FAIL|SKIP",
      postgresql: "OK|FAIL|SKIP",
      redis: "OK|FAIL|SKIP",
      api: "OK|FAIL|SKIP",
      mcp_binary: "OK|FAIL|SKIP",
      directory_structure: "OK|FAIL|SKIP",
      stale_workflows: "OK|WARN|SKIP",
      mcp_config: "OK|FAIL|SKIP"
    }
  }),
  importance: 0.5,
  tags: ["masday", "doctor", "health-check", "system-status"]
})
```

### Never

- Never skip memory storage after doctor checks
- Never store incomplete doctor reports
- Never store sensitive credentials in memory
