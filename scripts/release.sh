#!/usr/bin/env bash
#
# Masday Local Release
#
# Builds release binaries (Linux + Windows) and creates a GitHub Release.
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

# --- Build ---
FAILED=0

# Linux
info "Building Linux x86_64 release..."
if [ "$DRY_RUN" = true ]; then
  ok "(dry-run) Would build: cargo build -p masday-cli --release --target x86_64-unknown-linux-gnu"
else
  if source ~/.cargo/env 2>/dev/null; then true; fi
  if cargo build -p masday-cli --release --target x86_64-unknown-linux-gnu 2>&1; then
    # Strip
    strip "target/x86_64-unknown-linux-gnu/release/masday"
    cp "target/x86_64-unknown-linux-gnu/release/masday" "${DIST_DIR}/masday-linux-x86_64"
    chmod +x "${DIST_DIR}/masday-linux-x86_64"
    ok "Linux binary built ($(du -h "${DIST_DIR}/masday-linux-x86_64" | cut -f1))"
  else
    err "Linux build failed"
    FAILED=1
  fi
fi

# Windows
if [ "$BUILD_WINDOWS" = true ]; then
  info "Building Windows x86_64 release..."
  if [ "$DRY_RUN" = true ]; then
    ok "(dry-run) Would build: cargo build -p masday-cli --release --target x86_64-pc-windows-gnu --no-default-features"
  else
    if cargo build -p masday-cli --release --target x86_64-pc-windows-gnu --no-default-features 2>&1; then
      # Windows strip with mingw
      x86_64-w64-mingw32-strip "target/x86_64-pc-windows-gnu/release/masday.exe" 2>/dev/null || true
      cp "target/x86_64-pc-windows-gnu/release/masday.exe" "${DIST_DIR}/masday-windows-x86_64.exe"
      ok "Windows binary built ($(du -h "${DIST_DIR}/masday-windows-x86_64.exe" | cut -f1))"
    else
      err "Windows build failed"
      FAILED=1
    fi
  fi
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
  echo "    --title \"Masday CLI ${TAG}\" \\"
  echo "    --notes \"...\""
  echo ""
  ok "Dry run complete — no changes made"
  rm -rf "$DIST_DIR"
  exit 0
fi

info "Creating GitHub Release ${TAG}..."

# Generate release notes
RELEASE_NOTES="## Masday CLI ${TAG}

Self-contained binary with embedded templates (agents, skills, hooks).
No source code required — just download and run.

### Install
\`\`\`bash
# One-line install (Linux/macOS)
curl -fsSL https://github.com/dayartcrew-web/masday-workflow-rust/releases/download/${TAG}/install.sh | bash
\`\`\`

### Manual download
\`\`\`bash
# Linux
curl -fsSL -o masday https://github.com/dayartcrew-web/masday-workflow-rust/releases/download/${TAG}/masday-linux-x86_64
chmod +x masday

# Windows
curl -fsSL -o masday.exe https://github.com/dayartcrew-web/masday-workflow-rust/releases/download/${TAG}/masday-windows-x86_64.exe
\`\`\`

### Usage
\`\`\`bash
masday install                          # Local mode (requires cargo)
masday install --remote <url> --api-key <key>  # Remote mode
masday --version
\`\`\`

### Binary contents
- 28 agent definitions (embedded)
- 30+ skill definitions (embedded)
- Global + project hooks (embedded)
- MCP config generation (4 platforms)"

if gh release create "$TAG" "${DIST_DIR}"/* \
  --title "Masday CLI ${TAG}" \
  --notes "$RELEASE_NOTES"; then
  ok "Release ${TAG} created successfully!"
  echo ""
  echo "  https://github.com/dayartcrew-web/masday-workflow-rust/releases/tag/${TAG}"
  echo ""
else
  err "GitHub release creation failed"
  err "Manual command:"
  echo "  gh release create ${TAG} ${DIST_DIR}/* --title \"Masday CLI ${TAG}\""
  rm -rf "$DIST_DIR"
  exit 1
fi

# --- Cleanup ---
rm -rf "$DIST_DIR"
ok "Cleaned up dist/"
