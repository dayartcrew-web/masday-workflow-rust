#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT_DIR"

HOME_CLAUDE="${HOME}/.claude"
HOME_OPENCODE="${HOME}/.config/opencode"

echo "=== masday-workflow-rebuild Setup ==="

# 1. Install dependencies
echo "[1/8] Installing dependencies..."
pnpm install --frozen-lockfile 2>/dev/null || pnpm install

# 2. Generate Prisma client (skip if client exists and MCP server may be running)
echo "[2/8] Generating Prisma client..."
if [ -f "node_modules/.pnpm/@prisma+client@*/node_modules/.prisma/client/index.js" ]; then
  echo "  Prisma client already exists, skipping (run 'pnpm db:generate' manually to update)"
else
  pnpm db:generate
fi

# 3. Build all packages
echo "[3/8] Building all packages..."
pnpm build

# 4. Build agent-runner MCP server
echo "[4/8] Building agent-runner MCP server..."
pnpm --filter @mcp-rebuild/agent-runner build

# 5. Sync masday-* skills to local platform directories
echo "[5/8] Syncing masday-* skills to local platform directories..."

# Clean and recreate platform directories (prevents stale copies)
for dir in .agents .gemini .continue; do
  rm -rf "$dir"
  mkdir -p "$dir/agents" "$dir/skills"
done

# Codex (.agents/) - single nesting, agents/ and skills/ dirs
cp -r .claude/agents/* .agents/agents/ 2>/dev/null || true
for skill_dir in .claude/skills/masday-*/; do
  [ -d "$skill_dir" ] || continue
  cp -r "$skill_dir" ".agents/skills/$(basename "$skill_dir")" 2>/dev/null || true
done

# Gemini (.gemini/)
cp -r .claude/agents/* .gemini/agents/ 2>/dev/null || true
for skill_dir in .claude/skills/masday-*/; do
  [ -d "$skill_dir" ] || continue
  cp -r "$skill_dir" ".gemini/skills/$(basename "$skill_dir")" 2>/dev/null || true
done

# Continue (.continue/)
cp -r .claude/agents/* .continue/agents/ 2>/dev/null || true
for skill_dir in .claude/skills/masday-*/; do
  [ -d "$skill_dir" ] || continue
  cp -r "$skill_dir" ".continue/skills/$(basename "$skill_dir")" 2>/dev/null || true
done

# 6. Install masday-* skills to global ~/.claude/skills/
echo "[6/8] Installing masday-* skills to global ${HOME_CLAUDE}/skills/..."
mkdir -p "${HOME_CLAUDE}/skills"

copied=0
for skill_dir in .claude/skills/masday-*/; do
  [ -d "$skill_dir" ] || continue
  skill_name="$(basename "$skill_dir")"
  rm -rf "${HOME_CLAUDE}/skills/${skill_name}"
  cp -r "$skill_dir" "${HOME_CLAUDE}/skills/${skill_name}"
  copied=$((copied + 1))
done

# 7. Convert and install agents to ~/.config/opencode/agent/
echo "[7/8] Converting agents to opencode format and installing to ${HOME_OPENCODE}/agent/..."
mkdir -p "${HOME_OPENCODE}/agent"

node "${ROOT_DIR}/scripts/convert-agents.mjs" convert-to-dir \
  "${ROOT_DIR}/.claude/agents" \
  "${HOME_OPENCODE}/agent"

# 8. Install masday-* skills to ~/.config/opencode/skills/
echo "[8/8] Installing masday-* skills to ${HOME_OPENCODE}/skills/..."
mkdir -p "${HOME_OPENCODE}/skills"

for skill_dir in .claude/skills/masday-*/; do
  [ -d "$skill_dir" ] || continue
  skill_name="$(basename "$skill_dir")"
  rm -rf "${HOME_OPENCODE}/skills/${skill_name}"
  cp -r "$skill_dir" "${HOME_OPENCODE}/skills/${skill_name}"
done

# Summary
echo ""
echo "=== Setup complete ==="
echo "MCP server: masday (87 tools, 16 namespaces)"
echo "  Agents:  $(ls .claude/agents/*.md 2>/dev/null | wc -l) registered"
echo "  Hooks:   $(ls .claude/hooks/*.js .claude/hooks/*.mjs 2>/dev/null | wc -l) executable + $(ls .claude/hooks/*.md 2>/dev/null | wc -l) advisory"
echo "  Skills:  ${copied} masday-* skills -> ${HOME_CLAUDE}/skills/"
echo "  Opencode: $(ls "${HOME_OPENCODE}/agent/masday-"*.md 2>/dev/null | wc -l) agents + $(ls -d "${HOME_OPENCODE}/skills/masday-"* 2>/dev/null | wc -l) skills"
echo ""
echo "Start: node ${ROOT_DIR}/apps/agent-runner/dist/runtime/mcp.js"