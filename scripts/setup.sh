#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT_DIR"

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

# 5. Sync to platform directories
echo "[5/6] Syncing to platform directories..."

# Codex (.agents/)
mkdir -p .agents/agents
cp -r .claude/agents/* .agents/agents/ 2>/dev/null || true

# Gemini (.gemini/)
mkdir -p .gemini/agents
cp -r .claude/agents/* .gemini/agents/ 2>/dev/null || true

# Continue (.continue/)
mkdir -p .continue/agents
cp -r .claude/agents/* .continue/agents/ 2>/dev/null || true

# 6. Summary
echo "[6/6] Registration summary:"
echo "  Agents: $(ls .claude/agents/*.md 2>/dev/null | wc -l) registered"
echo "  Hooks:  $(ls .claude/hooks/*.js .claude/hooks/*.mjs 2>/dev/null | wc -l) executable + $(ls .claude/hooks/*.md 2>/dev/null | wc -l) advisory"
echo ""
echo "=== Setup complete ==="
echo "MCP servers: workflow-orchestrator(26), memory(9), semantic-search(2), policy(6), capability(10), unified(70)"
echo "Start: node C:/Users/AQR STD/Documents/GitHub/vibe-masday-workflow/masday-workflow-rebuild/apps/agent-runner/dist/runtime/mcp.js"
