# Getting Started - masday-workflow-rebuild

## Quick Start

```bash
# 1. Install & build
pnpm install
pnpm db:generate
pnpm build

# 2. Setup all platforms
bash scripts/setup.sh         # Linux/Mac
powershell scripts/setup.ps1  # Windows

# 3. Start MCP server
npx tsx apps/unified-mcp/src/index.ts
```

## Platform Support

| Platform | Agents Dir | Skills Dir | Config File |
|----------|-----------|-----------|-------------|
| Claude Code | `.claude/agents/` | `.claude/skills/` | `.claude/settings.json` |
| Codex CLI | `.agents/agents/` | `.agents/skills/` | `.codex/config.toml` |
| Gemini CLI | `.gemini/agents/` | `.gemini/skills/` | `.gemini/settings.json` |
| Continue | `.continue/agents/` | -- | `.continue/config.json` |
| GitHub Copilot | -- | -- | `.github/copilot.yml` |

## MCP Servers

| Server | Tools | Port |
|--------|-------|------|
| workflow-orchestrator | 26 | stdio |
| memory | 9 | stdio |
| semantic-search | 2 | stdio |
| policy | 6 | stdio |
| capability | 10 | stdio |
| unified | 70 | stdio |

## Architecture

See [docs/architecture.md](../docs/architecture.md) for full details.
