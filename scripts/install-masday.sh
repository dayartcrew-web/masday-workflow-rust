#!/usr/bin/env bash
#
# Masday CLI Installer
#
# Downloads the masday binary from GitHub Releases and installs it.
# Repo stays private — only the binary is downloaded.
#
# Usage:
#   curl -fsSL https://raw.githubusercontent.com/dayartcrew-web/masday-workflow-rust/master/scripts/install-masday.sh | bash
#   MASDAY_VERSION=v0.3.22 curl -fsSL https://raw.githubusercontent.com/dayartcrew-web/masday-workflow-rust/master/scripts/install-masday.sh | bash
#
# Environment variables:
#   MASDAY_QUICKSTART=1  Auto-run 'masday quickstart' after install (non-interactive)
#   MASDAY_VERSION=X.Y.Z  Install specific version (default: latest)
#   MASDAY_FORCE=1       Force reinstall even if already installed
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
        Linux*)  os="linux" ;;
        Darwin*) os="macos" ;;
        MINGW*|MSYS*|CYGWIN*)
            os="windows" ;;
        *)
            # Fallback: check for Windows without Unix layer
            if [ -f /c/Windows/System32/cmd.exe ] 2>/dev/null; then
                os="windows"
            else
                err "Unsupported OS: $(uname -s)"
                err "Supported: Linux, macOS, Windows (via Git Bash / MSYS2 / WSL)"
                exit 1
            fi
            ;;
    esac

    arch="$(uname -m 2>/dev/null || echo unknown)"
    case "$arch" in
        x86_64|amd64)  arch="x86_64" ;;
        arm64|aarch64) arch="aarch64" ;;
        *)
            err "Unsupported architecture: $(uname -m)"
            err "Supported: x86_64, aarch64 (arm64)"
            exit 1
            ;;
    esac

    echo "${os}-${arch}"
}

# Map platform string to GitHub Release artifact name
# Input: platform string like "linux-x86_64", "windows-x86_64", "macos-aarch64"
get_artifact_name() {
    local platform="$1"
    local os="${platform%%-*}"
    local arch="${platform##*-}"

    case "$os" in
        linux)   echo "masday-linux-${arch}"         ;;
        windows) echo "masday-windows-${arch}.exe"    ;;
        macos)   echo "masday-macos-${arch}"          ;;
        *)       echo "masday-${os}-${arch}"          ;;
    esac
}

# Map platform string to MCP artifact name
get_mcp_artifact_name() {
    local platform="$1"
    local os="${platform%%-*}"
    local arch="${platform##*-}"

    case "$os" in
        linux)   echo "masday-mcp-linux-${arch}"         ;;
        windows) echo "masday-mcp-windows-${arch}.exe"    ;;
        macos)   echo "masday-mcp-macos-${arch}"          ;;
        *)       echo "masday-mcp-${os}-${arch}"          ;;
    esac
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
    local artifact
    artifact="$(get_artifact_name "$platform")"
    local download_url="https://github.com/${REPO}/releases/download/${version}/${artifact}"

    local tmp_file
    tmp_file="$(mktemp)"

    info "Downloading masday ${version} for ${platform}..." >&2

    if ! curl -fsSL --progress-bar -o "$tmp_file" "$download_url"; then
        rm -f "$tmp_file"
        err "Failed to download from ${download_url}"
        err "Make sure the release exists and the repo has access configured."
        exit 1
    fi

    # Only echo the tmp_file path to stdout (captured by caller)
    echo "$tmp_file"
}

# Verify checksum if available
verify_checksum() {
    local binary_file="$1"
    local version="$2"
    local checksum_url="https://github.com/${REPO}/releases/download/${version}/checksums-sha256.txt"
    local actual_hash

    actual_hash="$(sha256sum "$binary_file" | awk '{print $1}')"

    info "Verifying checksum..." >&2
    if checksums="$(curl -fsSL "$checksum_url" 2>/dev/null)"; then
        local platform
        platform="$(detect_platform)"
        local artifact
        artifact="$(get_artifact_name "$platform")"
        local expected
        expected="$(echo "$checksums" | grep "${artifact}" | awk '{print $1}')"

        if [ -n "$expected" ] && [ "$actual_hash" = "$expected" ]; then
            ok "Checksum verified"
        else
            err "Checksum verification failed. Expected: ${expected:-missing}, Got: ${actual_hash}"
            rm -f "$binary_file"
            exit 1
        fi
    else
        err "Checksum file not available — cannot verify binary integrity. Aborting."
        rm -f "$binary_file"
        exit 1
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

    # Set binary name for Windows
    if [[ "$platform" == windows-* ]]; then
        BINARY_NAME="masday.exe"
    else
        BINARY_NAME="masday"
    fi

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

    # Check for existing installation
    if [ -x "${INSTALL_DIR}/${BINARY_NAME}" ]; then
        local existing_version
        existing_version="$("${INSTALL_DIR}/${BINARY_NAME}" --version 2>/dev/null | grep -oE '[0-9]+\.[0-9]+\.[0-9]+' | head -1)" || true

        if [ -n "$existing_version" ]; then
            # Strip 'v' prefix for comparison
            local new_ver="${version#v}"

            if [ "$existing_version" = "$new_ver" ]; then
                warn "masday ${existing_version} is already up-to-date at ${INSTALL_DIR}/${BINARY_NAME}"
                if [ "${MASDAY_FORCE:-0}" != "1" ]; then
                    info "Already on latest. Use MASDAY_FORCE=1 to force reinstall."
                    exit 0
                fi
                info "MASDAY_FORCE=1 — reinstalling same version..."
            else
                # Different version — auto-update
                info "Updating masday ${existing_version} → ${new_ver}..."
            fi
        fi
    fi

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

    # Download MCP server binary (best-effort)
    local mcp_artifact
    mcp_artifact="$(get_mcp_artifact_name "$platform")"
    local mcp_binary_name
    case "$platform" in
        windows-*) mcp_binary_name="masday-mcp.exe" ;;
        *)         mcp_binary_name="masday-mcp" ;;
    esac
    local mcp_url="https://github.com/${REPO}/releases/download/${version}/${mcp_artifact}"
    local mcp_tmp
    mcp_tmp="$(mktemp)"

    if curl -fsSL --progress-bar -o "$mcp_tmp" "$mcp_url" 2>/dev/null; then
        chmod +x "$mcp_tmp"
        mv "$mcp_tmp" "${INSTALL_DIR}/${mcp_binary_name}"
        ok "MCP server installed to ${INSTALL_DIR}/${mcp_binary_name}"
    else
        rm -f "$mcp_tmp"
        warn "MCP binary not in this release (use 'masday mcp' subcommand)"
    fi

    # Add to PATH if not already present
    local path_added=0

    if echo ":${PATH}:" | grep -q ":${INSTALL_DIR}:"; then
        ok "Already in PATH"
        path_added=1
    fi

    # Windows PATH registration
    if [[ "$platform" == windows-* ]] && [ $path_added -eq 0 ]; then
        # Try PowerShell User PATH registration
        if command -v powershell.exe &>/dev/null; then
            local win_path
            win_path="$(cygpath -w "$INSTALL_DIR" 2>/dev/null || echo "$INSTALL_DIR")"
            if powershell.exe -NoProfile -Command \
                "[Environment]::SetEnvironmentVariable('Path', [Environment]::GetEnvironmentVariable('Path', 'User') + ';${win_path}', 'User')" \
                2>/dev/null; then
                ok "Added to Windows PATH (User scope)"
                path_added=1
            fi
        fi

        # Also update Git Bash profile
        if [ $path_added -eq 0 ]; then
            for rc_file in "$HOME/.bashrc" "$HOME/.bash_profile"; do
                if [ -f "$rc_file" ] && ! grep -q 'masday/bin' "$rc_file" 2>/dev/null; then
                    echo '' >> "$rc_file"
                    echo '# Masday CLI' >> "$rc_file"
                    echo 'export PATH="$HOME/.masday/bin:$PATH"' >> "$rc_file"
                    ok "Added to $(basename "$rc_file")"
                    path_added=1
                    break
                fi
            done
        fi
    fi

    # Unix PATH registration (Linux/macOS)
    if [ $path_added -eq 0 ]; then
        local shell_rc=""
        if [ -f "${HOME}/.bashrc" ]; then shell_rc="${HOME}/.bashrc"; fi
        if [ -f "${HOME}/.zshrc" ]; then shell_rc="${HOME}/.zshrc"; fi

        local path_line="export PATH=\"\${PATH}:${INSTALL_DIR}\""

        if [ -n "$shell_rc" ] && ! grep -q "$INSTALL_DIR" "$shell_rc" 2>/dev/null; then
            echo "" >> "$shell_rc"
            echo "# Masday CLI" >> "$shell_rc"
            echo "$path_line" >> "$shell_rc"
            ok "Added to PATH in ${shell_rc}"
            warn "Run 'source ${shell_rc}' or start a new terminal to use masday"
            path_added=1
        fi
    fi

    # Fallback: manual instructions
    if [ $path_added -eq 0 ]; then
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

    # Auto-execute quickstart
    if [ "${MASDAY_QUICKSTART:-0}" = "1" ]; then
        info "Auto-running 'masday quickstart'..."
        "${INSTALL_DIR}/${BINARY_NAME}" quickstart
    elif [ -t 0 ]; then
        # Interactive terminal — ask user
        echo ""
        read -rp "${GREEN}Run 'masday quickstart' now? [Y/n]${NC} " answer
        case "$answer" in
            n*|N*) info "Okay. Run 'masday quickstart' when ready." ;;
            *)     "${INSTALL_DIR}/${BINARY_NAME}" quickstart ;;
        esac
    else
        # Non-interactive (piped curl | bash)
        info "Run 'masday quickstart' to complete setup."
    fi
    echo ""
}

main "$@"
