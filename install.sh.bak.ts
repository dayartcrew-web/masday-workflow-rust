#!/usr/bin/env bash
set -euo pipefail

# masday-workflow-rebuild installer
# Usage: curl -fsSL https://raw.githubusercontent.com/.../install.sh | bash
# Or:    bash install.sh

REPO_URL="${REPO_URL:-https://github.com/dayartcrew-web/masday-workflow-rebuild}"
INSTALL_DIR="${INSTALL_DIR:-${XDG_DATA_HOME:-$HOME}/.masday-workflow-rebuild}"

echo "=== masday-workflow-rebuild Installer ==="
echo ""

# Check prerequisites
for cmd in node git; do
  if ! command -v "$cmd" &>/dev/null; then
    echo "ERROR: $cmd is required but not installed."
    echo "  Install: https://nodejs.org (includes npm), then: npm install -g pnpm"
    exit 1
  fi
done

# Ensure pnpm is available
if ! command -v pnpm &>/dev/null; then
  echo "pnpm not found — attempting to enable via corepack..."
  corepack enable 2>/dev/null || npm install -g pnpm
fi

echo "[OK] Prerequisites: node $(node -v), pnpm $(pnpm -v), git $(git --version)"

# Clone or update
if [ -d "$INSTALL_DIR" ]; then
  echo "[1/5] Updating existing installation..."
  cd "$INSTALL_DIR"
  git pull --ff-only
else
  echo "[1/5] Cloning repository..."
  git clone "$REPO_URL" "$INSTALL_DIR"
  cd "$INSTALL_DIR"
fi

# Install dependencies
echo "[2/5] Installing dependencies..."
pnpm install --frozen-lockfile 2>/dev/null || pnpm install

# Build all packages
echo "[3/5] Building packages..."
pnpm build

# Sync platform configs and install git hooks
echo "[4/5] Syncing platform configs..."
bash scripts/setup.sh

# Configure environment
echo "[5/5] Configuring environment..."
if [ ! -f .env ]; then
  cp .env.example .env
  echo "  Created .env from .env.example — edit it with your settings."
else
  echo "  .env already exists, keeping current configuration."
fi

echo ""
echo "=== Installation complete ==="
echo ""
echo "Quick start:"
echo "  # Start PostgreSQL with Docker:"
echo "  docker compose up -d postgres"
echo ""
echo "  # Push database schema:"
echo "  pnpm db:push"
echo ""
echo "  # Start MCP server:"
echo "  npx tsx apps/agent-runner/src/runtime/mcp.ts"
echo ""
echo "  # Or start everything with Docker Compose:"
echo "  docker compose up -d"
echo ""
echo "Config: $INSTALL_DIR/.env"
echo "Setup:  $INSTALL_DIR/scripts/setup.sh"
