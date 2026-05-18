#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT_DIR"

HOME_CLAUDE="${HOME}/.claude"

echo "=== masday-workflow-rebuild Setup ==="

# 1. Install dependencies
echo "[1/6] Installing dependencies..."
pnpm install --frozen-lockfile 2>/dev/null || pnpm install

# 2. Generate Prisma client
echo "[2/6] Generating Prisma client..."
pnpm db:generate

# 3. Build all packages
echo "[3/6] Building all packages..."
pnpm build

# 4. Build agent-runner MCP server (compiled JS for MCP config)
echo "[4/6] Building agent-runner MCP server..."
pnpm --filter @mcp-rebuild/agent-runner build

# 5. Sync masday-* skills to local platform directories
echo "[5/6] Syncing masday-* skills to local platform directories..."

# Codex (.agents/)
mkdir -p .agents/agents .agents/skills
cp -r .claude/agents/* .agents/agents/ 2>/dev/null || true
for skill_dir in .claude/skills/masday-*/; do
  [ -d "$skill_dir" ] || continue
  cp -r "$skill_dir" ".agents/skills/$(basename "$skill_dir")" 2>/dev/null || true
done

# Gemini (.gemini/)
mkdir -p .gemini/agents .gemini/skills
cp -r .claude/agents/* .gemini/agents/ 2>/dev/null || true
for skill_dir in .claude/skills/masday-*/; do
  [ -d "$skill_dir" ] || continue
  cp -r "$skill_dir" ".gemini/skills/$(basename "$skill_dir")" 2>/dev/null || true
done

# Continue (.continue/)
mkdir -p .continue/agents .continue/skills
cp -r .claude/agents/* .continue/agents/ 2>/dev/null || true
for skill_dir in .claude/skills/masday-*/; do
  [ -d "$skill_dir" ] || continue
  cp -r "$skill_dir" ".continue/skills/$(basename "$skill_dir")" 2>/dev/null || true
done

# 6. Install masday-* skills to global ~/.claude/skills/
echo "[6/6] Installing masday-* skills to global ${HOME_CLAUDE}/skills/..."
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
