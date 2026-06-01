#!/usr/bin/env bash
#
# Masday CLI Installer
#
# Downloads the masday binary from GitHub Releases and installs it.
# Repo stays private — only the binary is downloaded.
#
# Usage:
#   curl -fsSL https://github.com/dayartcrew-web/masday-workflow-rust/releases/latest/download/install.sh | bash
#   curl -fsSL https://github.com/dayartcrew-web/masday-workflow-rust/releases/download/v0.1.0/install.sh | bash
#
set -euo pipefail

REPO="dayartcrew-web/masday-workflow-rust"
INSTALL_DIR="${HOME}/.masday/bin"
BINARY_NAME="masday"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
CYAN='\033[0;36m'
NC='\033[0m'

info()  { echo -e "${CYAN}$*${NC}"; }
ok()    { echo -e "${GREEN}✓ $*${NC}"; }
warn()  { echo -e "${YELLOW}⚠ $*${NC}"; }
err()   { echo -e "${RED}✗ $*${NC}" >&2; }

# Detect OS and architecture
detect_platform() {
    local os arch

    os="$(uname -s 2>/dev/null || echo unknown)"
    case "$os" in
        Linux)  os="linux" ;;
        Darwin) os="macos" ;;
        *)
            err "Unsupported OS: $os. Only Linux and macOS are supported."
            exit 1
            ;;
    esac

    arch="$(uname -m 2>/dev/null || echo unknown)"
    case "$arch" in
        x86_64|amd64)  arch="x86_64" ;;
        arm64|aarch64) arch="aarch64" ;;
        *)
            err "Unsupported architecture: $arch. Only x86_64 and aarch64 are supported."
            exit 1
            ;;
    esac

    echo "${os}-${arch}"
}

# Resolve the latest release tag
get_latest_version() {
    local api_url="https://api.github.com/repos/${REPO}/releases/latest"
    curl -fsSL "$api_url" 2>/dev/null | grep '"tag_name"' | head -1 | sed -E 's/.*"tag_name":\s*"([^"]+)".*/\1/'
}

# Download binary from GitHub Releases
download_binary() {
    local version="$1"
    local platform="$2"
    local artifact="masday-${platform}"
    local download_url="https://github.com/${REPO}/releases/download/${version}/${artifact}"

    local tmp_file
    tmp_file="$(mktemp)"

    info "Downloading masday ${version} for ${platform}..."

    if ! curl -fsSL --progress-bar -o "$tmp_file" "$download_url"; then
        rm -f "$tmp_file"
        err "Failed to download from ${download_url}"
        err "Make sure the release exists and the repo has access configured."
        exit 1
    fi

    echo "$tmp_file"
}

# Verify checksum if available
verify_checksum() {
    local binary_file="$1"
    local version="$2"
    local checksum_url="https://github.com/${REPO}/releases/download/${version}/checksums-sha256.txt"
    local actual_hash

    actual_hash="$(sha256sum "$binary_file" | awk '{print $1}')"

    info "Verifying checksum..."
    if checksums="$(curl -fsSL "$checksum_url" 2>/dev/null)"; then
        local platform
        platform="$(detect_platform)"
        local expected
        expected="$(echo "$checksums" | grep "masday-${platform}" | awk '{print $1}')"

        if [ -n "$expected" ] && [ "$actual_hash" = "$expected" ]; then
            ok "Checksum verified"
        else
            warn "Checksum mismatch or not found. Binary hash: ${actual_hash}"
        fi
    else
        warn "Checksum file not available — skipping verification"
    fi
}

# Main installation
main() {
    echo ""
    echo -e "${CYAN}=== Masday CLI Installer ===${NC}"
    echo ""

    # Detect platform
    local platform
    platform="$(detect_platform)"
    info "Platform: ${platform}"

    # Resolve version
    local version="${MASDAY_VERSION:-}"
    if [ -z "$version" ]; then
        info "Resolving latest version..."
        version="$(get_latest_version)"
        if [ -z "$version" ]; then
            err "Could not determine latest version. Set MASDAY_VERSION env var."
            err "Example: MASDAY_VERSION=v0.1.0 bash install.sh"
            exit 1
        fi
    fi
    ok "Version: ${version}"

    # Download
    local binary_file
    binary_file="$(download_binary "$version" "$platform")"

    # Verify
    verify_checksum "$binary_file" "$version"

    # Install
    mkdir -p "$INSTALL_DIR"
    chmod +x "$binary_file"
    mv "$binary_file" "${INSTALL_DIR}/${BINARY_NAME}"

    ok "Installed to ${INSTALL_DIR}/${BINARY_NAME}"

    # Add to PATH if not already present
    local shell_rc=""
    if [ -f "${HOME}/.bashrc" ]; then shell_rc="${HOME}/.bashrc"; fi
    if [ -f "${HOME}/.zshrc" ]; then shell_rc="${HOME}/.zshrc"; fi

    local path_line="export PATH=\"\${PATH}:${INSTALL_DIR}\""

    if echo ":${PATH}:" | grep -q ":${INSTALL_DIR}:"; then
        ok "Already in PATH"
    elif [ -n "$shell_rc" ] && ! grep -q "$INSTALL_DIR" "$shell_rc" 2>/dev/null; then
        echo "" >> "$shell_rc"
        echo "# Masday CLI" >> "$shell_rc"
        echo "$path_line" >> "$shell_rc"
        ok "Added to PATH in ${shell_rc}"
        warn "Run 'source ${shell_rc}' or start a new terminal to use masday"
    else
        info "Add to PATH manually:"
        echo "  export PATH=\"\${PATH}:${INSTALL_DIR}\""
    fi

    # Done
    echo ""
    ok "Installation complete!"
    echo ""
    echo "  ${BINARY_NAME} --version"
    echo "  ${BINARY_NAME} install --help"
    echo ""
    info "Next: cd your-project && masday install"
    echo ""
}

main "$@"
