#!/usr/bin/env bash
set -euo pipefail

# masday-workflow-rebuild installer
# Usage: curl -fsSL https://raw.githubusercontent.com/.../install.sh | bash
# Or:    bash install.sh

REPO_URL="https://github.com/user/masday-workflow-rebuild"
INSTALL_DIR="${INSTALL_DIR:-$HOME/.masday-workflow-rebuild}"

echo "=== masday-workflow-rebuild Installer ==="
echo ""

# Check prerequisites
for cmd in node pnpm git; do
  if ! command -v $cmd &>/dev/null; then
    echo "ERROR: $cmd is required but not installed."
    echo "  Install: https://nodejs.org (includes npm), then: npm install -g pnpm"
    exit 1
  fi
done
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

# Generate Prisma client
echo "[3/5] Generating Prisma client..."
pnpm db:generate

# Build all packages
echo "[4/5] Building packages..."
pnpm build

# Sync to platform directories
echo "[5/5] Syncing platform configs..."
bash scripts/setup.sh

echo ""
echo "=== Installation complete ==="
echo ""
echo "Quick start:"
echo "  # Start PostgreSQL:"
echo "  docker-compose up -d"
echo ""
echo "  # Start unified MCP server (70 tools):"
echo "  npx tsx apps/unified-mcp/src/index.ts"
echo ""
echo "  # Or start individual servers:"
echo "  npx tsx apps/workflow-orchestrator-mcp/src/index.ts"
echo ""
echo "Config: $INSTALL_DIR/.mcp.json"
echo "Docs:   $INSTALL_DIR/guides/getting-started.md"
