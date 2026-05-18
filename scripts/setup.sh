#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT_DIR"

HOME_CLAUDE="${HOME}/.claude"

echo "=== masday-workflow-rebuild Setup ==="

# 1. Install dependencies
echo "[1/7] Installing dependencies..."
pnpm install --frozen-lockfile 2>/dev/null || pnpm install

# 2. Generate Prisma client
echo "[2/7] Generating Prisma client..."
pnpm db:generate

# 3. Build all packages
echo "[3/7] Building all packages..."
pnpm build

# 4. Build agent-runner MCP server (compiled JS for MCP config)
echo "[4/7] Building agent-runner MCP server..."
pnpm --filter @mcp-rebuild/agent-runner build

# 5. Sync commands into .claude/commands/ from .agents/commands/
echo "[5/7] Syncing commands to .claude/commands/..."
mkdir -p .claude/commands
cp -r .agents/commands/* .claude/commands/ 2>/dev/null || true

# 6. Sync to local platform directories
echo "[6/7] Syncing to local platform directories..."

# Codex (.agents/)
mkdir -p .agents/agents .agents/skills .agents/commands
cp -r .claude/agents/* .agents/agents/ 2>/dev/null || true
cp -r .claude/skills/* .agents/skills/ 2>/dev/null || true
cp -r .claude/commands/* .agents/commands/ 2>/dev/null || true

# Gemini (.gemini/)
mkdir -p .gemini/agents .gemini/skills .gemini/commands
cp -r .claude/agents/* .gemini/agents/ 2>/dev/null || true
cp -r .claude/skills/* .gemini/skills/ 2>/dev/null || true
cp -r .claude/commands/* .gemini/commands/ 2>/dev/null || true

# Continue (.continue/)
mkdir -p .continue/agents .continue/skills .continue/commands
cp -r .claude/agents/* .continue/agents/ 2>/dev/null || true
cp -r .claude/skills/* .continue/skills/ 2>/dev/null || true
cp -r .claude/commands/* .continue/commands/ 2>/dev/null || true

# 7. Install masday-* skills to global ~/.claude/skills/
echo "[7/7] Installing masday-* skills to global ${HOME_CLAUDE}/skills/..."
mkdir -p "${HOME_CLAUDE}/skills"

copied=0
for skill_dir in .claude/skills/masday-*/; do
  [ -d "$skill_dir" ] || continue
  skill_name="$(basename "$skill_dir")"
  rm -rf "${HOME_CLAUDE}/skills/${skill_name}"
  cp -r "$skill_dir" "${HOME_CLAUDE}/skills/${skill_name}"
  copied=$((copied + 1))
done

# Summary
echo ""
echo "=== Setup complete ==="
echo "MCP servers: workflow-orchestrator(26), memory(9), semantic-search(2), policy(6), capability(10), unified(70)"
echo "  Agents:  $(ls .claude/agents/*.md 2>/dev/null | wc -l) registered"
echo "  Hooks:   $(ls .claude/hooks/*.js .claude/hooks/*.mjs 2>/dev/null | wc -l) executable + $(ls .claude/hooks/*.md 2>/dev/null | wc -l) advisory"
echo "  Skills:  ${copied} masday-* skills -> ${HOME_CLAUDE}/skills/"
echo ""
echo "Start: node ${ROOT_DIR}/apps/agent-runner/dist/runtime/mcp.js"
