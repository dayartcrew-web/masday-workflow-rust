# CLI Commands Reference

This page consolidates the main commands and workflows referenced in the contributor-facing docs.

## masday CLI (Rust Binary)

The `masday` binary is a self-contained installer and workflow CLI built in Rust.

### Distribution Model

```
User receives ONLY:
  masday (binary, 7.6MB)
    └── embedded templates (compile-time):
        ├── 28 agent .md files
        ├── 30+ skill directories
        ├── 6 global hooks (.js)
        └── 9 project hooks (.cjs/.js/.sh)

User does NOT receive:
  ✗ Root project source code
  ✗ Cargo workspace / Rust source
  ✗ pnpm monorepo / TypeScript source
  ✗ PostgreSQL schema (remote mode connects to server)
  ✗ Dashboard frontend
```

### Commands

```bash
# Install (full setup)
masday install                          # Local: build + install everything
masday install --remote <url> --api-key <key>  # Remote: connect to remote API
masday install --platform claude-code   # Platform-specific only
masday install --skip-build             # Use existing binaries
masday install --local-only             # Skip global dirs
masday install --force                  # Overwrite existing configs

# Uninstall (cleanup)
masday uninstall                        # Remove from project dirs
masday uninstall --global               # Remove from global dirs too

# Update
masday update                           # Re-install with force (preserves .env)

# Other
masday db-migrate                       # Run database migrations
masday serve                            # Start API server
masday status [id]                      # Show workflow status
masday --version                        # Print version
```

### Install Flow (Local Mode)

```
1. Check prerequisites (cargo, node, pnpm)
2. Build Rust crates (masday-api, masday-mcp)
3. Ensure .env exists (create from .env.example, never overwrite)
4. Extract embedded agents → .claude/agents/, .agents/, .gemini/agents/
5. Extract embedded skills → .claude/skills/, global skill dirs
6. Install global hooks → ~/.claude/hooks/
7. Install project hooks → .claude/hooks/
8. Generate MCP configs for each platform (.mcp.json, .vscode/mcp.json, etc.)
9. Update global settings.json (statusline, autoCompact)
```

### Install Flow (Remote Mode)

```
1. Check prerequisites (node only, no cargo needed)
2. Resolve masday-mcp binary (PATH or download)
3. Verify remote API connectivity (GET /api/health)
4. Steps 4-9 same as local, but MCP config points to remote URL
```

### Platform Support

| Platform | Project Agents | Project Skills | Global Skills | MCP Config |
|----------|---------------|----------------|---------------|------------|
| Claude Code | `.claude/agents/` | `.claude/skills/` | `~/.claude/skills/` | `.mcp.json` |
| Gemini CLI | `.gemini/agents/` | `.gemini/skills/` | `~/.gemini/config/skills/` | `.gemini/settings.json` |
| VS Code Copilot | `.agents/` | `.continue/skills/` | — | `.vscode/mcp.json` |
| OpenCode | `.opencode/agent/` | `.opencode/skills/` | `~/.config/opencode/agent/` (singular) | `.opencode/mcp.json` |

## Local project commands (Development)

```bash
cargo build --workspace                 # Build all Rust crates
cargo build --release -p masday-cli     # Build release CLI binary
cargo test -p masday-cli                # Run CLI tests (53 tests)
cargo run -p masday-cli -- install --help  # Test CLI help output

# Dashboard
cd apps/dashboard
pnpm install
pnpm dev                                # Dev server on port 3002
pnpm build                              # Production build

# MCP server
cargo run -p masday-mcp                 # Start MCP stdio server
```

### Demo commands

```bash
cd apps/agent-runner
pnpm demo:basic          # Basic workflow engine demo
pnpm demo:enhanced       # Enhanced engine with planner demo
pnpm demo:orchestrated   # Multi-agent orchestration demo
pnpm demo:intelligence   # Repository intelligence demo
pnpm demo:production     # Production features demo
```

## Claude / workflow commands (Skills)

### Workflow management

- `/masday-workflow-init` — Initialize `.masday/` data directory
- `/masday-workflow-new [prompt]` — Create + execute workflow in one shot
- `/masday-workflow-plan [id|prompt]` — Plan tasks for a workflow
- `/masday-workflow-run [id]` — Execute workflow
- `/masday-workflow-status` — Show all workflows
- `/masday-workflow-verify [id]` — Validate workflow results
- `/masday-workflow-fix [id]` — Fix workflow failures (retry logic)
- `/masday-workflow-add-task [id] [agent] [skill] [desc]` — Add task to workflow

### Research

- `/masday-research [topic]` — Research codebase with cached analysis

### Scaffolding

- `/create-agent [name]` — Create a new agent definition
- `/create-skill [name]` — Create a new skill
- `/create-command [name]` — Create a new slash command
- `/create-mcp-skill [name]` — Create a new MCP skill

All scaffolding commands auto-detect the project root via `git rev-parse`.

## Usage guidance

- Local mode requires Rust toolchain + cargo build
- Remote mode only requires the `masday` binary — no Rust needed
- All platforms auto-detected from existing config files
- `masday install` is idempotent — safe to run multiple times

## Related docs

- [Getting started](../getting-started.md)
- [MCP tools](./mcp-tools.md)
- [Workflow lifecycle](../workflows/lifecycle.md)
- [CLI distribution plan](../masday-cli-distribution-plan.md)
