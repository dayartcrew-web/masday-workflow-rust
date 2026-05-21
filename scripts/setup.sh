#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT_DIR"

OS_NAME="$(uname -s 2>/dev/null || echo unknown)"
case "$OS_NAME" in
  Linux|Darwin)
    ;;
  MINGW*|MSYS*|CYGWIN*)
    echo "Detected Windows via Git Bash/MSYS — continuing with setup.sh"
    ;;
  *)
    if [ -n "${WINDIR:-}" ] || [ -n "${ComSpec:-}" ]; then
      echo "Detected Windows shell environment without Git Bash/MSYS."
      echo "Please run scripts/setup.ps1 from PowerShell instead."
      exit 1
    fi
    echo "Warning: unrecognized OS '$OS_NAME' — attempting to continue."
    ;;
esac

HOME_CLAUDE="${HOME}/.claude"
HOME_OPENCODE="${HOME}/.config/opencode"

echo "=== masday-workflow-rebuild Setup ==="

# 1. Install dependencies
echo "[1/9] Installing dependencies..."
pnpm install --frozen-lockfile 2>/dev/null || pnpm install

# 2. Generate Prisma client (skip if client exists and MCP server may be running)
echo "[2/9] Generating Prisma client..."
PRISMA_CLIENT=$(ls node_modules/.pnpm/@prisma+client@*/node_modules/.prisma/client/index.js 2>/dev/null | head -1)
if [ -n "$PRISMA_CLIENT" ]; then
  echo "  Prisma client already exists, skipping (run 'pnpm db:generate' manually to update)"
else
  pnpm db:generate
fi

# 3. Build all packages
echo "[3/9] Building all packages..."
pnpm build

# 4. Build agent-runner MCP server
echo "[4/9] Building agent-runner MCP server..."
pnpm --filter @mcp-rebuild/agent-runner build

# 4b. pgvector setup (PostgreSQL only — skipped for sqlite://local)
DB_URL="${DATABASE_URL:-}"
if [ -n "$DB_URL" ] && [ "$DB_URL" != "sqlite://local" ]; then
  echo "  Running pgvector column setup (EMBEDDING_DIMENSIONS=${EMBEDDING_DIMENSIONS:-768})..."
  node "${ROOT_DIR}/scripts/setup-pgvector.mjs" && echo "  pgvector ready." || echo "  pgvector setup failed — run 'pnpm db:pgvector' manually."
else
  echo "  DATABASE_URL is sqlite://local or unset — skipping pgvector setup."
  echo "  Set DATABASE_URL to a PostgreSQL URL and run 'pnpm db:pgvector' to enable vector search."
fi

# 5. Sync masday-* skills to local platform directories
echo "[5/9] Syncing masday-* skills to local platform directories..."

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

# Sync rules to all platforms
echo "  Syncing .claude/rules/ to all platform directories..."
for plat_dir in .agents .gemini .continue .opencode .codex; do
  mkdir -p "$plat_dir/rules"
  rm -rf "$plat_dir/rules"/*
  if [ -d ".claude/rules" ]; then
    cp -r .claude/rules/* "$plat_dir/rules/" 2>/dev/null || true
  fi
done

# 6. Install masday-* skills to global ~/.claude/skills/
echo "[6/9] Installing masday-* skills to global ${HOME_CLAUDE}/skills/..."
mkdir -p "${HOME_CLAUDE}/skills"

copied=0
for skill_dir in .claude/skills/masday-*/; do
  [ -d "$skill_dir" ] || continue
  skill_name="$(basename "$skill_dir")"
  rm -rf "${HOME_CLAUDE}/skills/${skill_name}"
  cp -r "$skill_dir" "${HOME_CLAUDE}/skills/${skill_name}"
  copied=$((copied + 1))
done

# 7. Convert and install agents to ~/.config/opencode/agent/ AND project .opencode/agent/
echo "[7/9] Converting agents to opencode format (global + project)..."
mkdir -p "${HOME_OPENCODE}/agent"
mkdir -p "${ROOT_DIR}/.opencode/agent"

node "${ROOT_DIR}/scripts/convert-agents.mjs" convert \
  "${ROOT_DIR}/.claude/agents"

# 8. Install masday-* skills to ~/.config/opencode/skills/
echo "[8/9] Installing masday-* skills to ${HOME_OPENCODE}/skills/..."
mkdir -p "${HOME_OPENCODE}/skills"

for skill_dir in .claude/skills/masday-*/; do
  [ -d "$skill_dir" ] || continue
  skill_name="$(basename "$skill_dir")"
  rm -rf "${HOME_OPENCODE}/skills/${skill_name}"
  cp -r "$skill_dir" "${HOME_OPENCODE}/skills/${skill_name}"
done

# 9. Ensure .masday/ state directories exist (used by tdd-guard hook)
mkdir -p "${ROOT_DIR}/.masday/cache/tasks"
mkdir -p "${ROOT_DIR}/.masday/reports"

# 10. Summary
echo ""
echo "=== Setup complete ==="
echo "MCP server: masday (87 tools, 16 namespaces)"
echo "  Agents:  $(ls .claude/agents/*.md 2>/dev/null | wc -l) registered"
echo "  Hooks:   $(ls .claude/hooks/*.js .claude/hooks/*.mjs 2>/dev/null | wc -l) executable + $(ls .claude/hooks/*.md 2>/dev/null | wc -l) advisory"
echo "  TDD guard: workflow-aware (requiresTdd tasks blocked without tests)"
echo "  Skills:  ${copied} masday-* skills -> ${HOME_CLAUDE}/skills/"
echo "  Opencode: $(ls "${HOME_OPENCODE}/agent/masday-"*.md 2>/dev/null | wc -l) global agents + $(ls "${ROOT_DIR}/.opencode/agent/masday-"*.md 2>/dev/null | wc -l) project agents"
echo "  Embedding: EMBEDDING_PROVIDER=${EMBEDDING_PROVIDER:-fastembed} (fastembed|ollama|openai)"
echo "  Vector search: pnpm db:pgvector (PostgreSQL only; skipped for sqlite://local)"
echo ""
echo "Start: node ${ROOT_DIR}/apps/agent-runner/dist/runtime/mcp.js"