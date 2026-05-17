#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT_DIR"

echo "=== masday-workflow-rebuild Setup ==="

# 1. Install dependencies
echo "[1/5] Installing dependencies..."
pnpm install --frozen-lockfile 2>/dev/null || pnpm install

# 2. Generate Prisma client
echo "[2/5] Generating Prisma client..."
pnpm db:generate

# 3. Build all packages
echo "[3/5] Building all packages..."
pnpm build

# 4. Sync to platform directories
echo "[4/5] Syncing to platform directories..."

# Codex (.agents/)
mkdir -p .agents/agents .agents/skills
cp -r .claude/agents/* .agents/agents/ 2>/dev/null || true
cp -r .claude/skills/* .agents/skills/ 2>/dev/null || true

# Gemini (.gemini/)
mkdir -p .gemini/agents .gemini/skills
cp -r .claude/agents/* .gemini/agents/ 2>/dev/null || true
cp -r .claude/skills/* .gemini/skills/ 2>/dev/null || true

# Continue (.continue/)
mkdir -p .continue/agents
cp -r .claude/agents/* .continue/agents/ 2>/dev/null || true

# 5. Summary
echo "[5/5] Registration summary:"
echo "  Agents: $(ls .claude/agents/*.md 2>/dev/null | wc -l) registered"
echo "  Skills: $(ls -d .claude/skills/*/ 2>/dev/null | wc -l) registered"
echo "  Hooks:  $(ls .claude/hooks/*.js .claude/hooks/*.mjs 2>/dev/null | wc -l) executable + $(ls .claude/hooks/*.md 2>/dev/null | wc -l) advisory"
echo ""
echo "=== Setup complete ==="
echo "MCP servers: workflow-orchestrator(26), memory(9), semantic-search(2), policy(6), capability(10), unified(70)"
echo "Start: npx tsx apps/unified-mcp/src/index.ts"
