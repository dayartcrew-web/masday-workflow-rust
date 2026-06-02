#!/usr/bin/env bash
#
# Masday Local Release
#
# Builds release binaries (CLI + MCP server) for Linux + Windows and creates a GitHub Release.
# Called automatically by pre-push hook on tag push, or manually:
#
#   bash scripts/release.sh v0.2.0
#   bash scripts/release.sh v0.2.0 --dry-run
#
set -euo pipefail

# --- Colors ---
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
CYAN='\033[0;36m'
NC='\033[0m'
info()  { echo -e "${CYAN}[release]$NC $*"; }
ok()    { echo -e "${GREEN}[release]$NC ✓ $*"; }
warn()  { echo -e "${YELLOW}[release]$NC ⚠ $*"; }
err()   { echo -e "${RED}[release]$NC ✗ $*" >&2; }

# --- Args ---
TAG="${1:-}"
DRY_RUN=false
if [ "${2:-}" = "--dry-run" ]; then
  DRY_RUN=true
fi

if [ -z "$TAG" ]; then
  err "Usage: bash scripts/release.sh <tag> [--dry-run]"
  err "Example: bash scripts/release.sh v0.2.0"
  exit 1
fi

# Validate tag format
if [[ ! "$TAG" =~ ^v[0-9]+\.[0-9]+\.[0-9]+.*$ ]]; then
  err "Invalid tag format: $TAG (expected v*.*.*)"
  exit 1
fi

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT_DIR"
DIST_DIR="${ROOT_DIR}/dist"

# --- Cleanup previous ---
rm -rf "$DIST_DIR"
mkdir -p "$DIST_DIR"

# --- Check gh CLI ---
if ! command -v gh &>/dev/null; then
  err "gh CLI not installed. Install: https://cli.github.com/"
  exit 1
fi

if ! gh auth status &>/dev/null 2>&1; then
  err "gh CLI not authenticated. Run: gh auth login"
  exit 1
fi
ok "gh CLI authenticated"

# --- Detect platforms ---
BUILD_LINUX=true
BUILD_WINDOWS=false

if command -v x86_64-w64-mingw32-gcc &>/dev/null; then
  BUILD_WINDOWS=true
else
  warn "mingw-w64 not found — Windows cross-compile skipped"
  warn "Install: sudo apt install mingw-w64"
fi

# --- Ensure rust targets ---
info "Checking Rust targets..."
if ! rustup target list --installed | grep -q "x86_64-unknown-linux-gnu"; then
  info "Adding target x86_64-unknown-linux-gnu..."
  rustup target add x86_64-unknown-linux-gnu
fi

if [ "$BUILD_WINDOWS" = true ]; then
  if ! rustup target list --installed | grep -q "x86_64-pc-windows-gnu"; then
    info "Adding target x86_64-pc-windows-gnu..."
    rustup target add x86_64-pc-windows-gnu
  fi
fi

# --- Build function ---
# Usage: build_binary <package> <binary_name> <target> <dist_name> [extra_cargo_args...]
build_binary() {
  local pkg="$1"
  local bin_name="$2"
  local target="$3"
  local dist_name="$4"
  shift 4

  local is_windows=false
  [[ "$target" == *windows* ]] && is_windows=true

  info "Building ${pkg} for ${target}..."
  if [ "$DRY_RUN" = true ]; then
    ok "(dry-run) Would build: cargo build -p ${pkg} --release --target ${target} $*"
    return 0
  fi

  if source ~/.cargo/env 2>/dev/null; then true; fi
  if cargo build -p "${pkg}" --release --target "${target}" "$@" 2>&1; then
    local bin_path="target/${target}/release/${bin_name}"

    # Strip
    if [ "$is_windows" = true ]; then
      x86_64-w64-mingw32-strip "${bin_path}" 2>/dev/null || true
    else
      strip "${bin_path}" 2>/dev/null || true
    fi

    cp "${bin_path}" "${DIST_DIR}/${dist_name}"
    chmod +x "${DIST_DIR}/${dist_name}"
    ok "${dist_name} built ($(du -h "${DIST_DIR}/${dist_name}" | cut -f1))"
  else
    err "${pkg} build failed for ${target}"
    return 1
  fi
}

# --- Build all binaries ---
FAILED=0

# ── masday-cli (Linux) ──
build_binary masday-cli masday x86_64-unknown-linux-gnu masday-linux-x86_64 || FAILED=1

# ── masday-cli (Windows) ──
if [ "$BUILD_WINDOWS" = true ]; then
  build_binary masday-cli masday.exe x86_64-pc-windows-gnu masday-windows-x86_64.exe --no-default-features || FAILED=1
fi

# ── masday-mcp (Linux) ──
build_binary masday-mcp masday-mcp x86_64-unknown-linux-gnu masday-mcp-linux-x86_64 || FAILED=1

# ── masday-mcp (Windows) ──
if [ "$BUILD_WINDOWS" = true ]; then
  build_binary masday-mcp masday-mcp.exe x86_64-pc-windows-gnu masday-mcp-windows-x86_64.exe --no-default-features || FAILED=1
fi

if [ "$FAILED" -eq 1 ]; then
  err "Build failed — aborting release"
  rm -rf "$DIST_DIR"
  exit 1
fi

# --- Checksums ---
if [ "$DRY_RUN" = true ]; then
  ok "(dry-run) Would generate checksums-sha256.txt"
else
  info "Generating checksums..."
  (cd "$DIST_DIR" && sha256sum masday-* > checksums-sha256.txt)
  ok "Checksums generated:"
  cat "${DIST_DIR}/checksums-sha256.txt"
fi

# --- Install script ---
if [ -f "${ROOT_DIR}/scripts/install-masday.sh" ]; then
  cp "${ROOT_DIR}/scripts/install-masday.sh" "${DIST_DIR}/install.sh"
  ok "Install script copied"
fi

# --- GitHub Release ---
if [ "$DRY_RUN" = true ]; then
  echo ""
  ok "(dry-run) Would create release:"
  echo "  gh release create ${TAG} \\"
  echo "    ${DIST_DIR}/* \\"
  echo "    --title \"Masday ${TAG}\" \\"
  echo "    --notes \"...\""
  echo ""
  echo "  Artifacts:"
  ls -1 "${DIST_DIR}/" | sed 's/^/    /'
  echo ""
  ok "Dry run complete — no changes made"
  rm -rf "$DIST_DIR"
  exit 0
fi

info "Creating GitHub Release ${TAG}..."

# Generate release notes
RELEASE_NOTES="## Masday ${TAG}

Self-contained binaries for the Masday workflow orchestration platform.
No source code required — just download and run.

### Binaries

| File | Description |
|------|-------------|
| \`masday-linux-x86_64\` | CLI installer (Linux) |
| \`masday-windows-x86_64.exe\` | CLI installer (Windows) |
| \`masday-mcp-linux-x86_64\` | MCP server (Linux) |
| \`masday-mcp-windows-x86_64.exe\` | MCP server (Windows) |

### Install CLI
\`\`\`bash
# One-line install (Linux/macOS)
curl -fsSL https://github.com/dayartcrew-web/masday-workflow-rust/releases/download/${TAG}/install.sh | bash

# Manual download
# Linux
curl -fsSL -o masday https://github.com/dayartcrew-web/masday-workflow-rust/releases/download/${TAG}/masday-linux-x86_64
chmod +x masday

# Windows
curl -fsSL -o masday.exe https://github.com/dayartcrew-web/masday-workflow-rust/releases/download/${TAG}/masday-windows-x86_64.exe
\`\`\`

### Setup MCP Server (stdio mode)
\`\`\`bash
# Linux — place in PATH or ~/.masday/bin/
curl -fsSL -o ~/.masday/bin/masday-mcp https://github.com/dayartcrew-web/masday-workflow-rust/releases/download/${TAG}/masday-mcp-linux-x86_64
chmod +x ~/.masday/bin/masday-mcp

# Windows — place in PATH or %USERPROFILE%\\.masday\\bin\\
curl -fsSL -o masday-mcp.exe https://github.com/dayartcrew-web/masday-workflow-rust/releases/download/${TAG}/masday-mcp-windows-x86_64.exe
\`\`\`

Then run the CLI installer to configure MCP in your editor:
\`\`\`bash
masday install                          # Standalone mode (agents + skills only)
masday install --local                  # Local mode (builds from source)
masday install --remote <url> --api-key <key>  # Remote mode
\`\`\`

### What's Included

**CLI (\`masday\`)**
- 28 agent definitions (embedded)
- 30+ skill definitions (embedded)
- Global + project hooks (embedded)
- MCP config generation (4 platforms: Claude Code, Gemini, VS Code, OpenCode)

**MCP Server (\`masday-mcp\`)**
- 20 tool domains (96+ MCP tools)
- stdio transport
- Requires PostgreSQL + running API server for full functionality

### Prerequisites
- PostgreSQL 16 on port 54341
- API server (\`masday-api\`) running for full MCP tool access
- Node.js for hooks"

if gh release create "$TAG" "${DIST_DIR}"/* \
  --title "Masday ${TAG}" \
  --notes "$RELEASE_NOTES"; then
  ok "Release ${TAG} created successfully!"
  echo ""
  echo "  https://github.com/dayartcrew-web/masday-workflow-rust/releases/tag/${TAG}"
  echo ""
else
  err "GitHub release creation failed"
  err "Manual command:"
  echo "  gh release create ${TAG} ${DIST_DIR}/* --title \"Masday ${TAG}\""
  rm -rf "$DIST_DIR"
  exit 1
fi

# --- Cleanup ---
rm -rf "$DIST_DIR"
ok "Cleaned up dist/"
