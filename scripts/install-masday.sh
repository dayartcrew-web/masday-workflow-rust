#!/usr/bin/env bash
#
# Masday CLI Installer
#
# Downloads the masday binary from GitHub Releases and installs it.
# Uses curl first (public repo), falls back to gh CLI if needed.
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
DIM='\033[0;2m'
BOLD='\033[1m'
NC='\033[0m'

info()  { echo -e "${CYAN}$*${NC}" >&2; }
ok()    { echo -e "${GREEN}✓ $*${NC}" >&2; }
warn()  { echo -e "${YELLOW}⚠ $*${NC}" >&2; }
err()   { echo -e "${RED}✗ $*${NC}" >&2; }

# ─── Progress spinner ────────────────────────────────────────────────────────
# Works in any terminal (no external deps). Silently skipped if no TTY.
_SPINNER_PID=""
_SPINNER_MSG=""

_start_spinner() {
    _SPINNER_MSG="${1:-Loading...}"
    # Only spin if stderr is a terminal (stdout may be captured by $())
    if [ ! -t 2 ]; then
        echo -e "${DIM}  ${_SPINNER_MSG}${NC}" >&2
        return
    fi
    local frames="⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏"
    _spin() {
        local i=0
        # Only hide cursor if stderr is truly a terminal
        if [ -t 2 ]; then
            tput civis 2>/dev/null || true  # hide cursor
        fi
        while true; do
            local frame="${frames:$((i % ${#frames})):1}"
            printf "\r  ${CYAN}%s${NC} %s" "$frame" "$_SPINNER_MSG" >&2
            i=$((i + 1))
            sleep 0.08
        done
    }
    # Redirect all spinner output to /dev/null when not on a real TTY
    # Prevents ANSI escapes from leaking into $() captures
    if [ -t 2 ]; then
        _spin &
    else
        _spin &>/dev/null &
    fi
    _SPINNER_PID=$!
}

_stop_spinner() {
    local result="${1:-ok}"  # ok | fail | warn
    local msg="${2:-$_SPINNER_MSG}"

    if [ -n "$_SPINNER_PID" ]; then
        kill "$_SPINNER_PID" 2>/dev/null || true
        wait "$_SPINNER_PID" 2>/dev/null || true
        _SPINNER_PID=""
        tput cnorm 2>/dev/null || true  # restore cursor
    fi

    case "$result" in
        ok)   printf "\r  %s %s\n" "${GREEN}✓${NC}" "$msg" >&2 ;;
        fail) printf "\r  %s %s\n" "${RED}✗${NC}" "$msg" >&2 ;;
        warn) printf "\r  %s %s\n" "${YELLOW}⚠${NC}" "$msg" >&2 ;;
    esac
}

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
    local version

    # Method 1: gh CLI (instant — already authenticated)
    if command -v gh &>/dev/null; then
        version=$(gh release list --repo "${REPO}" --limit 1 --json tagName --jq '.[0].tagName' 2>/dev/null) || true
        if [ -n "$version" ]; then
            echo "$version"
            return
        fi
    fi

    # Method 2: GitHub API (fast, structured)
    version=$(curl -fsSL --connect-timeout 5 --max-time 8 "https://api.github.com/repos/${REPO}/releases/latest" 2>/dev/null \
        | grep '"tag_name"' | head -1 | sed -E 's/.*"tag_name":\s*"([^"]+)".*/\1/')
    if [ -n "$version" ]; then
        echo "$version"
        return
    fi

    # Method 3: Scrape release page HTML (fallback for rate-limited API)
    version=$(curl -fsSL --connect-timeout 5 --max-time 8 "https://github.com/${REPO}/releases/latest" 2>/dev/null \
        | grep -oE '/releases/tag/v[0-9]+\.[0-9]+\.[0-9]+' \
        | head -1 | sed 's|.*/v||' | sed 's/^/v/')
    if [ -n "$version" ]; then
        echo "$version"
        return
    fi

    echo ""
}

# Download binary from GitHub Releases with progress
download_binary() {
    local version="$1"
    local platform="$2"
    local artifact
    artifact="$(get_artifact_name "$platform")"
    local download_url="https://github.com/${REPO}/releases/download/${version}/${artifact}"

    local tmp_file
    tmp_file="$(mktemp 2>/dev/null)"
    # Windows Git Bash: ensure Windows-native tools can resolve the temp path
    if [[ "$(uname -s)" == MINGW* || "$(uname -s)" == MSYS* ]]; then
        tmp_file="$(cygpath -u "$(cygpath -w "$tmp_file")" 2>/dev/null || echo "$tmp_file")"
    fi

    info "Downloading masday ${version} for ${platform}..."

    # Method 1: curl (direct download — works for public repos)
    _start_spinner "Downloading ${artifact}..."
    if curl -fSL -s --connect-timeout 10 --max-time 120 -o "$tmp_file" "$download_url" 2>/dev/null; then
        local size
        size=$(du -h "$tmp_file" 2>/dev/null | cut -f1 || echo "???")
        _stop_spinner ok "Downloaded ${artifact} (${size})"
        echo "$tmp_file"
        return
    fi
    _stop_spinner fail "curl download failed"
    warn "Trying gh CLI fallback..."

    # Method 2: gh CLI (authenticated — fallback for private repos or restricted networks)
    if command -v gh &>/dev/null; then
        _start_spinner "Downloading via gh CLI..."
        if gh release download "$version" --repo "${REPO}" --pattern "${artifact}" --output "$tmp_file" --clobber 2>/dev/null; then
            local size
            size=$(du -h "$tmp_file" 2>/dev/null | cut -f1 || echo "???")
            _stop_spinner ok "Downloaded ${artifact} via gh (${size})"
            echo "$tmp_file"
            return
        fi
        _stop_spinner fail "gh CLI download failed"
    fi

    rm -f "$tmp_file"
    err "Failed to download from ${download_url}"
    err "For private repos, install gh CLI and run: gh auth login"
    exit 1
}

# Verify checksum if available
verify_checksum() {
    local binary_file="$1"
    local version="$2"
    local checksum_url="https://github.com/${REPO}/releases/download/${version}/checksums-sha256.txt"
    local actual_hash

    # Use certified path for sha256sum (Windows Git Bash safe)
    if command -v sha256sum &>/dev/null; then
        actual_hash="$(sha256sum "$binary_file" 2>/dev/null | awk '{print $1}')"
    elif command -v sha256sum.exe &>/dev/null; then
        # Windows: use .exe variant with Windows-native path
        local win_path
        win_path="$(cygpath -w "$binary_file" 2>/dev/null || echo "$binary_file")"
        actual_hash="$(sha256sum.exe "$win_path" 2>/dev/null | awk '{print $1}')"
    elif command -v certutil &>/dev/null; then
        # Windows fallback: certutil
        actual_hash="$(certutil -hashfile "$binary_file" SHA256 2>/dev/null | grep -v ':' | tr -d ' \r\n' | tr 'A-F' 'a-f')"
    fi

    _start_spinner "Verifying checksum..."
    if checksums="$(curl -fSL --connect-timeout 5 --max-time 15 "$checksum_url" 2>/dev/null)"; then
        local platform
        platform="$(detect_platform)"
        local artifact
        artifact="$(get_artifact_name "$platform")"
        local expected
        expected="$(echo "$checksums" | grep "${artifact}" | awk '{print $1}')"

        if [ -n "$expected" ] && [ "$actual_hash" = "$expected" ]; then
            _stop_spinner ok "Checksum verified"
        else
            _stop_spinner fail "Checksum mismatch. Expected: ${expected:-missing}, Got: ${actual_hash}"
            rm -f "$binary_file"
            exit 1
        fi
    else
        _stop_spinner warn "Checksum file not available — skipped (hash: ${actual_hash:0:16}...)"
    fi
}

# Main installation
main() {
    echo ""
    echo -e "${CYAN}${BOLD}=== Masday CLI Installer ===${NC}"
    echo ""

    # Detect platform
    _start_spinner "Detecting platform..."
    local platform
    platform="$(detect_platform)"
    _stop_spinner ok "Platform: ${platform}"

    # Set binary name for Windows
    if [[ "$platform" == windows-* ]]; then
        BINARY_NAME="masday.exe"
    else
        BINARY_NAME="masday"
    fi

    # Resolve version
    local version="${MASDAY_VERSION:-}"
    if [ -z "$version" ]; then
        _start_spinner "Resolving latest version..."
        version="$(get_latest_version)"
        if [ -z "$version" ]; then
            _stop_spinner fail "Could not determine latest version"
            err "Set MASDAY_VERSION env var. Example: MASDAY_VERSION=v0.1.0 bash install.sh"
            exit 1
        fi
        _stop_spinner ok "Version: ${version}"
    else
        ok "Version: ${version} (pinned)"
    fi

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
    _start_spinner "Installing to ${INSTALL_DIR}..."
    mkdir -p "$INSTALL_DIR"
    chmod +x "$binary_file"
    mv "$binary_file" "${INSTALL_DIR}/${BINARY_NAME}"
    _stop_spinner ok "Installed to ${INSTALL_DIR}/${BINARY_NAME}"

    # MCP server runs as subcommand: 'masday mcp' — no separate binary needed
    ok "MCP server available via 'masday mcp' subcommand"

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
        info "Auto-running 'masday quickstart' (non-interactive)..."
        # Build quickstart flags from env vars
        QUICKSTART_FLAGS=""
        [ -n "${MASDAY_MODE:-}" ] && QUICKSTART_FLAGS="$QUICKSTART_FLAGS --mode ${MASDAY_MODE}"
        [ -n "${MASDAY_PLATFORM:-}" ] && QUICKSTART_FLAGS="$QUICKSTART_FLAGS --platform ${MASDAY_PLATFORM}"
        [ -n "${MASDAY_EMBEDDING:-}" ] && QUICKSTART_FLAGS="$QUICKSTART_FLAGS --embedding ${MASDAY_EMBEDDING}"
        [ -n "${MASDAY_DATABASE_URL:-}" ] && QUICKSTART_FLAGS="$QUICKSTART_FLAGS --database-url ${MASDAY_DATABASE_URL}"
        # Default: standalone mode with --yes (no TTY needed)
        if [ -z "$QUICKSTART_FLAGS" ]; then
            QUICKSTART_FLAGS="--mode standalone --yes"
        else
            QUICKSTART_FLAGS="$QUICKSTART_FLAGS --yes"
        fi
        "${INSTALL_DIR}/${BINARY_NAME}" quickstart $QUICKSTART_FLAGS
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
        info "  Non-interactive: masday quickstart --mode standalone --yes"
        info "  Local mode:      masday quickstart --mode local --yes"
    fi
    echo ""
}

main "$@"
