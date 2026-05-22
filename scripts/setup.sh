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
HOME_GEMINI="${HOME}/.gemini"
HOME_OPENCODE="${HOME}/.config/opencode"

echo "=== masday-workflow-rebuild Setup ==="

# 1. Install dependencies
echo "[1/10] Installing dependencies..."
pnpm install --frozen-lockfile 2>/dev/null || pnpm install

# 2. Generate Drizzle client
echo "[2/10] Generating Drizzle client..."
if [ -f "node_modules/drizzle-orm/index.js" ] || [ -f "packages/db/node_modules/drizzle-orm/index.js" ]; then
  echo "  Drizzle ORM already installed, skipping (run 'pnpm db:generate' manually to update)"
else
  pnpm db:generate
fi

# 3. Build all packages
echo "[3/10] Building all packages..."
pnpm build

# 4. Build agent-runner MCP server
echo "[4/10] Building agent-runner MCP server..."
pnpm --filter @mcp-rebuild/agent-runner build

# 4b. pgvector setup (PostgreSQL only)
DB_URL="${DATABASE_URL:-}"
if [ -n "$DB_URL" ] && [ "$DB_URL" != "sqlite://local" ]; then
  echo "  Running pgvector column setup (EMBEDDING_DIMENSIONS=${EMBEDDING_DIMENSIONS:-768})..."
  node "${ROOT_DIR}/scripts/setup-pgvector.mjs" && echo "  pgvector ready." || echo "  pgvector setup failed — run 'pnpm db:pgvector' manually."
else
  echo "  DATABASE_URL is sqlite://local or unset — skipping pgvector setup."
  echo "  Set DATABASE_URL to a PostgreSQL URL and run 'pnpm db:pgvector' to enable vector search."
fi

# 5. Create .env if missing
echo "[5/10] Checking .env file..."
if [ ! -f ".env" ] && [ -f ".env.example" ]; then
  cp .env.example .env
  echo "  Created .env from .env.example — fill in your values before starting."
elif [ ! -f ".env" ]; then
  echo "  No .env or .env.example found — skipping."
else
  echo "  .env already exists."
fi

# 6. Sync masday-* skills to local platform directories
echo "[6/10] Syncing masday-* skills to local platform directories..."

# Preserve .gemini/settings.json before cleaning
GEMINI_SETTINGS_BAK=""
if [ -f ".gemini/settings.json" ]; then
  GEMINI_SETTINGS_BAK="$(cat .gemini/settings.json)"
fi

for dir in .agents .gemini .continue; do
  rm -rf "$dir"
  mkdir -p "$dir/agents" "$dir/skills"
done

# Restore .gemini/settings.json
if [ -n "$GEMINI_SETTINGS_BAK" ]; then
  echo "$GEMINI_SETTINGS_BAK" > .gemini/settings.json
elif [ -f "scripts/.gemini/settings.json" ]; then
  cp scripts/.gemini/settings.json .gemini/settings.json
fi

# Copy agents and skills to each platform
for plat_dir in .agents .gemini .continue; do
  cp -r .claude/agents/* "${plat_dir}/agents/" 2>/dev/null || true
  for skill_dir in .claude/skills/masday-*/; do
    [ -d "$skill_dir" ] || continue
    cp -r "$skill_dir" "${plat_dir}/skills/$(basename "$skill_dir")" 2>/dev/null || true
  done
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

# 7. Install masday-* skills to global directories
echo "[7/10] Installing masday-* skills to global directories..."

# Claude Code: ~/.claude/skills/
mkdir -p "${HOME_CLAUDE}/skills"
copied_claude=0
for skill_dir in .claude/skills/masday-*/; do
  [ -d "$skill_dir" ] || continue
  skill_name="$(basename "$skill_dir")"
  rm -rf "${HOME_CLAUDE}/skills/${skill_name}"
  cp -r "$skill_dir" "${HOME_CLAUDE}/skills/${skill_name}"
  copied_claude=$((copied_claude + 1))
done
echo "  Claude Code: ${copied_claude} skills → ${HOME_CLAUDE}/skills/"

# Gemini: ~/.gemini/config/skills/
mkdir -p "${HOME_GEMINI}/config/skills"
copied_gemini=0
for skill_dir in .claude/skills/masday-*/; do
  [ -d "$skill_dir" ] || continue
  skill_name="$(basename "$skill_dir")"
  rm -rf "${HOME_GEMINI}/config/skills/${skill_name}"
  cp -r "$skill_dir" "${HOME_GEMINI}/config/skills/${skill_name}"
  copied_gemini=$((copied_gemini + 1))
done
echo "  Gemini CLI:  ${copied_gemini} skills → ${HOME_GEMINI}/config/skills/"

# OpenCode: ~/.config/opencode/skills/
mkdir -p "${HOME_OPENCODE}/skills"
copied_opencode=0
for skill_dir in .claude/skills/masday-*/; do
  [ -d "$skill_dir" ] || continue
  skill_name="$(basename "$skill_dir")"
  rm -rf "${HOME_OPENCODE}/skills/${skill_name}"
  cp -r "$skill_dir" "${HOME_OPENCODE}/skills/${skill_name}"
  copied_opencode=$((copied_opencode + 1))
done
echo "  OpenCode:    ${copied_opencode} skills → ${HOME_OPENCODE}/skills/"

# 8. Convert and install agents to OpenCode
echo "[8/10] Converting agents to opencode format (global + project)..."
mkdir -p "${HOME_OPENCODE}/agent"
mkdir -p "${ROOT_DIR}/.opencode/agent"

if [ -f "${ROOT_DIR}/scripts/convert-agents.mjs" ]; then
  node "${ROOT_DIR}/scripts/convert-agents.mjs" convert \
    "${ROOT_DIR}/.claude/agents"
  echo "  OpenCode agents converted."
else
  echo "  scripts/convert-agents.mjs not found — skipping agent conversion."
fi

# 9. MCP config for each platform
echo "[9/10] Setting up MCP configs..."

# Project .mcp.json (Claude Code reads this)
cat > .mcp.json << 'MCPEOF'
{
  "mcpServers": {
    "masday": {
      "type": "stdio",
      "command": "node",
      "args": [
        "apps/agent-runner/dist/runtime/mcp.js"
      ]
    }
  }
}
MCPEOF
echo "  .mcp.json (Claude Code) — uses relative path to built JS"

# .gemini/settings.json (already restored or copied in step 6)
if [ ! -f ".gemini/settings.json" ]; then
  cat > .gemini/settings.json << 'GEMINI_EOF'
{
  "context": "This project uses masday-workflow-rebuild MCP server for workflow management.",
  "mcpServers": {
    "masday": {
      "type": "stdio",
      "command": "node",
      "args": [
        "--no-warnings",
        "apps/agent-runner/dist/runtime/mcp.js"
      ]
    }
  }
}
GEMINI_EOF
fi
echo "  .gemini/settings.json (Gemini CLI) — uses built JS with suppressed logging"

# .vscode/mcp.json (Copilot)
mkdir -p .vscode
cat > .vscode/mcp.json << 'VSCODEEOF'
{
  "servers": {
    "masday": {
      "type": "stdio",
      "command": "npx",
      "args": ["tsx", "apps/agent-runner/src/runtime/mcp.ts"]
    }
  }
}
VSCODEEOF
echo "  .vscode/mcp.json (VS Code Copilot)"

# .github/agents/
mkdir -p .github/agents
echo "  .github/agents/ (GitHub Copilot) ready"

# 10. Git hooks (cross-platform enforcement)
echo "[10/10] Installing git hooks..."
if [ -d ".git/hooks" ]; then
  for hook in pre-commit pre-push; do
    [ -f "scripts/git-hooks/${hook}" ] && cp "scripts/git-hooks/${hook}" ".git/hooks/${hook}" && chmod +x ".git/hooks/${hook}"
  done
  echo "  Git hooks installed (pre-commit + pre-push)"
fi
mkdir -p .masday/cache/tasks .masday/reports

echo ""
echo "=== Setup complete ==="
echo "  Claude Code:  .claude/settings.json (hooks) + .mcp.json (MCP)"
echo "  Gemini CLI:   .gemini/settings.json (MCP via npx tsx)"
echo "                ${HOME_GEMINI}/config/skills/ (${copied_gemini} skills)"
echo "  VS Code:      .vscode/mcp.json (Copilot MCP)"
echo "  GitHub:       .github/agents/ (coding agent)"
echo "  OpenCode:     .opencode/agent/ (${copied_opencode} agents converted)"
echo "  Git hooks:    .git/hooks/pre-commit + pre-push (ALL platforms)"
echo "  Skills:       ${copied_claude} masday-* skills installed"
echo ""
echo "Start: node apps/agent-runner/dist/runtime/mcp.js"
