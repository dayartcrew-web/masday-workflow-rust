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

echo "=== masday-workflow-rust Setup ==="
echo ""
echo "This project has been migrated to Rust."
echo "Backend: Rust crates (masday-core, masday-db, masday-service, masday-api, masday-mcp)"
echo "Frontend: Next.js dashboard (apps/dashboard)"
echo ""

# 1. Check Rust installation
echo "[1/8] Checking Rust installation..."
if ! command -v cargo &>/dev/null; then
  echo "  ERROR: Rust not found. Please install Rust first:"
  echo "  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
  exit 1
fi
echo "  Rust found: $(cargo --version | head -1)"

# 2. Check pnpm installation (for dashboard)
echo "[2/8] Checking pnpm installation..."
if ! command -v pnpm &>/dev/null; then
  echo "  Installing pnpm..."
  npm install -g pnpm
fi
echo "  pnpm found: $(pnpm --version)"

# 3. Install dashboard dependencies
echo "[3/8] Installing dashboard dependencies..."
cd apps/dashboard
pnpm install 2>/dev/null || pnpm install
cd "$ROOT_DIR"

# 4. Build Rust crates
echo "[4/8] Building Rust crates..."
cargo build --workspace 2>&1 | head -20

# 5. Create .env if missing
echo "[5/8] Checking .env file..."
if [ ! -f ".env" ] && [ -f ".env.example" ]; then
  cp .env.example .env
  echo "  Created .env from .env.example — fill in your values before starting."
elif [ ! -f ".env" ]; then
  echo "  No .env or .env.example found — skipping."
else
  echo "  .env already exists."
fi

# 6. Sync masday-* skills to local platform directories
echo "[6/8] Syncing masday-* skills to local platform directories..."

# Preserve .gemini/settings.json before cleaning
GEMINI_SETTINGS_BAK=""
if [ -f ".gemini/settings.json" ]; then
  GEMINI_SETTINGS_BAK="$(cat .gemini/settings.json)"
fi

for dir in .agents .gemini .continue; do
  mkdir -p "$dir/agents" "$dir/skills"
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
echo "[7/8] Installing masday-* skills to global directories..."

# Helper: copy skill safely — skip if target dir not writable
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

# 8. MCP config for each platform (Rust binary)
echo "[8/8] Setting up MCP configs..."

# Find the built Rust MCP binary
MCP_BIN=""
if [ -f "target/release/masday-mcp" ]; then
  MCP_BIN="target/release/masday-mcp"
elif [ -f "target/debug/masday-mcp" ]; then
  MCP_BIN="target/debug/masday-mcp"
else
  echo "  WARNING: Rust MCP binary not found. Run 'cargo build -p masday-mcp' first."
  MCP_BIN="target/debug/masday-mcp"
fi

# --- Claude Code: .mcp.json ---
cat > .mcp.json << MCPEOF
{
  "mcpServers": {
    "masday": {
      "type": "stdio",
      "command": "${ROOT_DIR}/${MCP_BIN}",
      "cwd": "${ROOT_DIR}",
      "env": {
        "DATABASE_URL": "${DATABASE_URL:-}",
        "MASDAY_API_URL": "http://localhost:30101",
        "MASDAY_API_KEY": "CHANGE_ME",
        "RUST_LOG": "${RUST_LOG:-info}"
      }
    }
  }
}
MCPEOF
echo "  .mcp.json (Claude Code) — Rust MCP binary: ${MCP_BIN}"

# --- Gemini CLI: .gemini/settings.json ---
if [ ! -f ".gemini/settings.json" ]; then
  cat > .gemini/settings.json << GEMINI_EOF
{
  "context": "This project uses masday-workflow-rust MCP server (Rust implementation).",
  "mcpServers": {
    "masday": {
      "type": "stdio",
      "command": "${MCP_BIN}",
      "cwd": "${ROOT_DIR}",
      "env": {
        "DATABASE_URL": "${DATABASE_URL:-}",
        "MASDAY_API_URL": "http://localhost:30101",
        "MASDAY_API_KEY": "CHANGE_ME",
        "RUST_LOG": "${RUST_LOG:-info}"
      }
    }
  }
}
GEMINI_EOF
fi
echo "  .gemini/settings.json (Gemini CLI)"

# --- VS Code Copilot: .vscode/mcp.json ---
mkdir -p .vscode
cat > .vscode/mcp.json << VSCODEEOF
{
  "servers": {
    "masday": {
      "command": "${MCP_BIN}",
      "cwd": "${ROOT_DIR}",
      "env": {
        "DATABASE_URL": "${DATABASE_URL:-}",
        "MASDAY_API_URL": "http://localhost:30101",
        "MASDAY_API_KEY": "CHANGE_ME",
        "RUST_LOG": "${RUST_LOG:-info}"
      }
    }
  }
}
VSCODEEOF
echo "  .vscode/mcp.json (VS Code Copilot)"

# --- VS Code Copilot: .github/agents/masday.agent.md ---
mkdir -p .github/agents
cat > .github/agents/masday.agent.md << 'AGENTEOF'
---
name: masday
description: masday-workflow-rust workflow orchestration agent (Rust implementation)
tools: ['*']
model: ['Claude Sonnet 4.6', 'GPT-5.2']
---

# masday Agent (Rust)

You are the masday-workflow-rust orchestration agent. This project uses Rust for the backend (masday-mcp, masday-api) and Next.js for the frontend dashboard.

## Architecture

- **Backend**: Rust crates (masday-core, masday-db, masday-service, masday-api, masday-mcp)
- **Frontend**: Next.js dashboard (apps/dashboard)
- **Protocol**: Model Context Protocol (MCP) over stdio
- **Database**: PostgreSQL with pgvector

## Mandatory Protocol

1. **Check masday MCP tools first** — use MCP tools before falling back to shell commands
2. **Follow the workflow lifecycle** — INIT > ANALYZE > PLAN > EXECUTE > VERIFY > DONE
3. **Enforce review pipeline** — after completing work, run review_submit > policy_validate_completion > workflow_completeTask
4. **Use underscore tool names** — all MCP tools use underscore format (e.g., `workflow_create`, `memory_store`)

## Quick Commands

- Build all: `cargo build --workspace`
- Run tests: `cargo test --workspace`
- Run MCP server: `cargo run -p masday-mcp`
- Run API server: `cargo run -p masday-api`
- Run dashboard: `cd apps/dashboard && pnpm dev`
AGENTEOF

# --- VS Code Copilot: .github/hooks/masday-hooks.json ---
mkdir -p .github/hooks
cat > .github/hooks/masday-hooks.json << HOOKSEOF
{
  "hooks": {
    "preToolUse": [
      {
        "matcher": ".*",
        "command": "node",
        "args": [".github/hooks/skill-step-guard.cjs"]
      }
    ]
  }
}
HOOKSEOF
echo "  .github/hooks/masday-hooks.json"

# Summary
echo ""
echo "=== Setup Complete ==="
echo ""

# 9. Install git hooks + global Claude hooks
echo "[9/9] Installing hooks..."
bash "${ROOT_DIR}/scripts/install-hooks.sh"

# 10. Update global Claude settings
echo "[10/10] Updating global Claude settings..."
HOME_CLAUDE="${HOME}/.claude"
mkdir -p "${HOME_CLAUDE}/hooks" 2>/dev/null || true

# Copy global hooks (statusline, session-start, compact, context-warning, bash-guard)
GLOBAL_HOOKS=(
  "masday-statusline.js"
  "masday-session-start.js"
  "masday-pre-compact.js"
  "masday-post-compact.js"
  "masday-context-warning.js"
  "masday-pre-bash-guard.js"
)
copied_global=0
for hook in "${GLOBAL_HOOKS[@]}"; do
  if [ -f "${ROOT_DIR}/scripts/global-hooks/${hook}" ]; then
    cp "${ROOT_DIR}/scripts/global-hooks/${hook}" "${HOME_CLAUDE}/hooks/${hook}" 2>/dev/null && copied_global=$((copied_global + 1))
  fi
done
echo "  Global hooks: ${copied_global} → ${HOME_CLAUDE}/hooks/"

# Update statusline config in global settings.json (only the relevant fields)
SETTINGS_FILE="${HOME_CLAUDE}/settings.json"
if [ -f "$SETTINGS_FILE" ]; then
  echo "  Updating statusLine + autoCompact in global settings.json..."
  node -e "
    const fs = require('fs');
    const s = JSON.parse(fs.readFileSync('${SETTINGS_FILE}', 'utf8'));
    s.statusLine = { type: 'command', command: 'node \"${HOME_CLAUDE}/hooks/masday-statusline.js\"' };
    s.autoCompact = true;
    s.autoCompactThreshold = 0.9;
    fs.writeFileSync('${SETTINGS_FILE}', JSON.stringify(s, null, 2) + '\n');
  " 2>/dev/null || echo "  WARNING: Could not update settings.json"
else
  echo "  No global settings.json found — skipping settings update."
fi

echo ""
echo "Available commands:"
echo "  cargo run -p masday-mcp     # Start MCP server (stdio)"
echo "  cargo run -p masday-api     # Start API server (port 30101)"
echo "  cargo test --workspace      # Run tests"
echo "  cargo clippy --workspace    # Lint"
echo "  cargo fmt --all --check     # Check formatting"
echo ""
echo "Environment variables (.env):"
echo "  DATABASE_URL=postgresql://user:pass@localhost:54341/masday_workflow"
echo "  MUSDAY_API_URL=http://localhost:30101"
echo "  RUST_LOG=info               # Logging level"
echo ""
echo "MCP servers configured:"
echo "  Claude Code:  .mcp.json"
echo "  Gemini CLI:   .gemini/settings.json"
echo "  VS Code:      .vscode/mcp.json"
echo ""
echo "Git hooks installed:"
echo "  pre-commit: cargo fmt + clippy"
echo "  pre-push:   cargo build + test"
echo ""
echo "Next steps:"
echo "  1. Fill in .env with your DATABASE_URL"
echo "  2. Start PostgreSQL: docker compose up -d"
echo "  3. Start API server: cargo run --release -p masday-api"
echo "  4. Start MCP server: cargo run --release -p masday-mcp"
echo ""
