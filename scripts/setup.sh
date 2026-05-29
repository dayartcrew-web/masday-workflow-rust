#!/usr/bin/env bash
set -euo pipefail

_SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
if command -v realpath &>/dev/null; then
  ROOT_DIR="$(realpath "$_SCRIPT_DIR/..")"
else
  ROOT_DIR="$_SCRIPT_DIR/.."
fi
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

CP_FLAGS='-r'
if [ "$OS_NAME" = 'Linux' ]; then
  CP_FLAGS='-a'
fi

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
  mkdir -p "$dir/agents" "$dir/skills"
  # Only remove masday-* prefixed items to preserve other agents/skills
  rm -rf "$dir/agents/masday-"* 2>/dev/null || true
  rm -rf "$dir/skills/masday-"* 2>/dev/null || true
done

# Restore .gemini/settings.json
if [ -n "$GEMINI_SETTINGS_BAK" ]; then
  echo "$GEMINI_SETTINGS_BAK" > .gemini/settings.json
elif [ -f "scripts/.gemini/settings.json" ]; then
  cp scripts/.gemini/settings.json .gemini/settings.json
fi

# Copy agents and skills to each platform
for plat_dir in .agents .gemini .continue; do
  cp $CP_FLAGS .claude/agents/* "${plat_dir}/agents/" 2>/dev/null || true
  for skill_dir in .claude/skills/masday-*/; do
    [ -d "$skill_dir" ] || continue
    cp $CP_FLAGS "$skill_dir" "${plat_dir}/skills/$(basename "$skill_dir")" 2>/dev/null || true
  done
done

# Sync rules to all platforms
echo "  Syncing .claude/rules/ to all platform directories..."
for plat_dir in .agents .gemini .continue .opencode .codex; do
  mkdir -p "$plat_dir/rules"
  rm -rf "$plat_dir/rules/masday-"* 2>/dev/null || true
  if [ -d ".claude/rules" ]; then
    cp $CP_FLAGS .claude/rules/* "$plat_dir/rules/" 2>/dev/null || true
  fi
done

# 7. Install masday-* skills to global directories
echo "[7/10] Installing masday-* skills to global directories..."

# Helper: copy skill safely — skip if target dir not writable (e.g. owned by root)
_safe_cp_skill() {
  local src="$1" dest="$2"
  local dest_parent
  dest_parent="$(dirname "$dest")"
  [ -d "$dest_parent" ] && [ -w "$dest_parent" ] || {
    echo "  Skip: $dest_parent not writable"
    return 1
  }
  rm -rf "$dest" 2>/dev/null || true
  cp $CP_FLAGS "$src" "$dest"
}

# Claude Code: ~/.claude/skills/
mkdir -p "${HOME_CLAUDE}/skills" 2>/dev/null || true
copied_claude=0
for skill_dir in .claude/skills/masday-*/; do
  [ -d "$skill_dir" ] || continue
  skill_name="$(basename "$skill_dir")"
  _safe_cp_skill "$skill_dir" "${HOME_CLAUDE}/skills/${skill_name}" && copied_claude=$((copied_claude + 1))
done
echo "  Claude Code: ${copied_claude} skills → ${HOME_CLAUDE}/skills/"

# Gemini: ~/.gemini/config/skills/
mkdir -p "${HOME_GEMINI}/config/skills" 2>/dev/null || true
copied_gemini=0
for skill_dir in .claude/skills/masday-*/; do
  [ -d "$skill_dir" ] || continue
  skill_name="$(basename "$skill_dir")"
  _safe_cp_skill "$skill_dir" "${HOME_GEMINI}/config/skills/${skill_name}" && copied_gemini=$((copied_gemini + 1))
done
echo "  Gemini CLI:  ${copied_gemini} skills → ${HOME_GEMINI}/config/skills/"

# OpenCode: ~/.config/opencode/skills/
mkdir -p "${HOME_OPENCODE}/skills" 2>/dev/null || true
copied_opencode=0
for skill_dir in .claude/skills/masday-*/; do
  [ -d "$skill_dir" ] || continue
  skill_name="$(basename "$skill_dir")"
  _safe_cp_skill "$skill_dir" "${HOME_OPENCODE}/skills/${skill_name}" && copied_opencode=$((copied_opencode + 1))
done
echo "  OpenCode:    ${copied_opencode} skills → ${HOME_OPENCODE}/skills/"

# 8. Convert and install agents to OpenCode
echo "[8/10] Converting agents to opencode format (global + project)..."

# Project-local dir (always writable)
mkdir -p "${ROOT_DIR}/.opencode/agent"

# Global dir (may be owned by root — skip if not writable)
if mkdir -p "${HOME_OPENCODE}/agent" 2>/dev/null; then
  echo "  Global OpenCode dir: ${HOME_OPENCODE}/agent"
else
  echo "  Skip: cannot create ${HOME_OPENCODE}/agent (permission denied) — project-local only."
fi

if [ -f "${ROOT_DIR}/scripts/convert-agents.mjs" ]; then
  node "${ROOT_DIR}/scripts/convert-agents.mjs" convert \
    "${ROOT_DIR}/.claude/agents" 2>/dev/null && echo "  OpenCode agents converted." || echo "  Agent conversion had errors (some dirs may not be writable)."
else
  echo "  scripts/convert-agents.mjs not found — skipping agent conversion."
fi

# 9. MCP config + Copilot agents/hooks for each platform
echo "[9/10] Setting up MCP configs + Copilot customization..."

MCP_JS="apps/agent-runner/dist/runtime/mcp.js"

# --- Claude Code: .mcp.json ---
# IMPORTANT: cwd and env are required so dotenv resolves .env correctly
# when Claude Code launches the MCP server from an arbitrary directory.
# The MCP server also resolves .env from its own script location as fallback.
cat > .mcp.json << MCPEOF
{
  "mcpServers": {
    "masday": {
      "type": "stdio",
      "command": "node",
      "args": [
        "apps/agent-runner/dist/runtime/mcp.js"
      ],
      "cwd": "${ROOT_DIR}",
      "env": {
        "DATABASE_URL": "${DATABASE_URL:-}",
        "NODE_ENV": "development"
      }
    }
  }
}
MCPEOF
echo "  .mcp.json (Claude Code) — with cwd + env for reliable .env resolution"

# --- Gemini CLI: .gemini/settings.json (already restored or copied in step 6) ---
if [ ! -f ".gemini/settings.json" ]; then
  cat > .gemini/settings.json << GEMINI_EOF
{
  "context": "This project uses masday-workflow-rebuild MCP server for workflow management.",
  "mcpServers": {
    "masday": {
      "type": "stdio",
      "command": "node",
      "args": [
        "--no-warnings",
        "apps/agent-runner/dist/runtime/mcp.js"
      ],
      "cwd": "${ROOT_DIR}",
      "env": {
        "DATABASE_URL": "${DATABASE_URL:-}",
        "NODE_ENV": "development"
      }
    }
  }
}
GEMINI_EOF
fi
echo "  .gemini/settings.json (Gemini CLI)"

# --- VS Code Copilot: .vscode/mcp.json ---
# Docs: https://code.visualstudio.com/docs/copilot/customization/mcp-servers
# Format: { "servers": { "name": { "command": "...", "args": [...] } } }
# No "type" field needed — stdio is inferred when "command" is present.
mkdir -p .vscode
cat > .vscode/mcp.json << VSCODEEOF
{
  "servers": {
    "masday": {
      "command": "node",
      "args": [
        "--no-warnings",
        "apps/agent-runner/dist/runtime/mcp.js"
      ],
      "cwd": "${ROOT_DIR}",
      "env": {
        "DATABASE_URL": "${DATABASE_URL:-}",
        "NODE_ENV": "development"
      }
    }
  }
}
VSCODEEOF
echo "  .vscode/mcp.json (VS Code Copilot) — node + built JS + cwd + env"

# --- VS Code Copilot: .github/agents/masday.agent.md ---
# Docs: https://code.visualstudio.com/docs/copilot/customization/custom-agents
# VS Code auto-discovers .agent.md files in .github/agents/
# Also reads .claude/agents/*.md (Claude format) automatically.
mkdir -p .github/agents
cat > .github/agents/masday.agent.md << 'AGENTEOF'
---
name: masday
description: masday-workflow-rebuild workflow orchestration agent with 87 MCP tools
tools: ['*']
model: ['Claude Sonnet 4.6', 'GPT-5.2']
handoffs:
  - label: Implement Plan
    agent: agent
    prompt: Implement the plan outlined above.
    send: false
---

# masday Agent

You are the masday-workflow-rebuild orchestration agent. You have access to 87 MCP tools across 16 namespaces.

## Mandatory Protocol

1. **Check masday MCP tools first** — use MCP tools before falling back to shell commands.
2. **Follow the workflow lifecycle** — INIT > ANALYZE > PLAN > EXECUTE > VERIFY > DONE
3. **Enforce review pipeline** — after completing work, run review_submit > policy_validate_completion > workflow_completeTask
4. **Use underscore tool names** — all MCP tools use underscore format (e.g., `workflow_create`, `memory_store`)

## Priority Order

1. masday MCP tools (workflow, memory, search, policy, capability)
2. Agent orchestrator for task routing
3. Code skills for implementation

## Pre-Commit Checks

Before marking any task complete:
- Run `pnpm typecheck` — must pass with zero errors
- Run `pnpm test` — all tests must pass
- No hardcoded secrets or credentials
- No console.log statements in production code
AGENTEOF
echo "  .github/agents/masday.agent.md (VS Code Copilot custom agent)"

# --- VS Code Copilot: .github/hooks/ ---
# Docs: https://code.visualstudio.com/docs/copilot/customization/hooks
# VS Code reads .github/hooks/*.json AND .claude/settings.json hooks.
# Convert key hooks to Copilot-native format for best compatibility.
# VS Code tool names differ from Claude: editFiles/createFile (not Edit/Write).
mkdir -p .github/hooks

cat > .github/hooks/masday-hooks.json << 'HOOKSEOF'
{
  "hooks": {
    "SessionStart": [
      {
        "type": "command",
        "command": "node .claude/hooks/run-hook.mjs masday-mem-context",
        "timeout": 15
      }
    ],
    "PreToolUse": [
      {
        "type": "command",
        "command": "node .claude/hooks/run-hook.mjs pre-tool-use",
        "timeout": 30
      },
      {
        "type": "command",
        "command": "node .claude/hooks/run-hook.mjs workflow-lock",
        "timeout": 30
      },
      {
        "type": "command",
        "command": "node .claude/hooks/run-hook.mjs tdd-guard",
        "timeout": 30
      },
      {
        "type": "command",
        "command": "node .claude/hooks/skill-step-guard.cjs",
        "timeout": 30
      }
    ],
    "PostToolUse": [
      {
        "type": "command",
        "command": "node .claude/hooks/run-hook.mjs post-tool-use",
        "timeout": 30
      }
    ],
    "Stop": [
      {
        "type": "command",
        "command": "node .claude/hooks/run-hook.mjs on-stop",
        "timeout": 60
      }
    ]
  }
}
HOOKSEOF
echo "  .github/hooks/masday-hooks.json (VS Code Copilot hooks + skill-step-guard)"

# --- Copilot user-level MCP registration (optional, uses `code` CLI) ---
# This registers masday at the user profile level so it works across all workspaces.
CODE_CLI=""
for cli in code code-insiders codium; do
  if command -v "$cli" &>/dev/null; then
    CODE_CLI="$cli"
    break
  fi
done
if [ -n "$CODE_CLI" ]; then
  ABS_MCP_JS="$(cd "$ROOT_DIR" && pwd)/${MCP_JS}"
  "$CODE_CLI" --add-mcp "{\"name\":\"masday\",\"command\":\"node\",\"args\":[\"--no-warnings\",\"${ABS_MCP_JS}\"]}" 2>/dev/null && \
    echo "  User-level MCP registered via '$CODE_CLI --add-mcp'" || \
    echo "  '$CODE_CLI --add-mcp' skipped (VS Code not running or not available)"
else
  echo "  No VS Code CLI found (tried code, code-insiders, codium) — skipping user-level MCP"
fi

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
echo "  Gemini CLI:   .gemini/settings.json (MCP via node)"
echo "                ${HOME_GEMINI}/config/skills/ (${copied_gemini} skills)"
echo "  VS Code:      .vscode/mcp.json (Copilot MCP)"
echo "                .github/agents/masday.agent.md (custom agent)"
echo "                .github/hooks/masday-hooks.json (Copilot hooks)"
echo "  GitHub:       .github/agents/ (coding agent)"
echo "  OpenCode:     .opencode/agent/ (${copied_opencode} agents converted)"
echo "  Git hooks:    .git/hooks/pre-commit + pre-push (ALL platforms)"
echo "  Skills:       ${copied_claude} masday-* skills installed"
echo ""
echo "Start: node apps/agent-runner/dist/runtime/mcp.js"
