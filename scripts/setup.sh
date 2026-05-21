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

# 2. Generate Drizzle client (skip if client exists and MCP server may be running)
echo "[2/9] Generating Drizzle client..."
if [ -f "node_modules/drizzle-orm/index.js" ] || [ -f "packages/db/node_modules/drizzle-orm/index.js" ]; then
  echo "  Drizzle client already exists, skipping (run 'pnpm db:generate' manually to update)"
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


# 9. Gemini MCP config (portable npx tsx)
echo "[9/11] Setting up Gemini MCP config..."
[ -f "scripts/.gemini/settings.json" ] && cp scripts/.gemini/settings.json .gemini/settings.json
echo "  .gemini/settings.json installed"

# 10. GitHub Copilot + VS Code MCP config
echo "[10/11] Setting up Copilot + VS Code MCP..."
mkdir -p .github/agents .vscode
echo "  Copilot + VS Code MCP ready"

# 11. Git hooks (cross-platform enforcement)
echo "[11/11] Installing git hooks..."
if [ -d ".git/hooks" ]; then
  for hook in pre-commit pre-push; do
    [ -f "scripts/git-hooks/${hook}" ] && cp "scripts/git-hooks/${hook}" ".git/hooks/${hook}" && chmod +x ".git/hooks/${hook}"
  done
  echo "  Git hooks installed (pre-commit + pre-push)"
fi
mkdir -p .masday/cache/tasks .masday/reports

echo ""
echo "=== Setup complete ==="
echo "  Claude Code:  .claude/settings.json (hooks + MCP)"
echo "  Gemini CLI:   .gemini/settings.json (MCP via npx tsx)"
echo "  VS Code:      .vscode/mcp.json (Copilot MCP)"
echo "  GitHub:       .github/agents/masday.md (coding agent)"
echo "  OpenCode:     .opencode/agent/ (converted agents)"
echo "  Git hooks:    .git/hooks/pre-commit + pre-push (ALL platforms)"
echo "  Skills:       ${copied} masday-* skills installed"
echo ""
echo "Start: node ${ROOT_DIR}/apps/agent-runner/dist/runtime/mcp.js"
