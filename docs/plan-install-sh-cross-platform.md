# Plan: install.sh Cross-Platform OS Detection

## Status: IN PROGRESS

> **Update**: `release.yml` sudah di-update dengan macOS + Windows matrix.
> Tinggal setup `RELEASE_TOKEN` secret dan test.

## Problem

`install.sh` saat ini hanya mendukung **Linux** dan **macOS** via `detect_platform()`. Windows user tidak bisa menggunakan `curl | bash` dan harus download manual dari GitHub Releases.

User ingin:
1. OS detector yang lengkap (Linux, macOS, Windows)
2. Setelah install berhasil, langsung execute `masday quickstart`

## Current State

```bash
# detect_platform() saat ini
os="$(uname -s)"
case "$os" in
    Linux)  os="linux" ;;
    Darwin) os="macos" ;;
    *)      err "Unsupported OS" && exit 1 ;;  # ← Windows langsung exit
esac
```

**Platform artifacts di GitHub Releases:**
- `masday-linux-x86_64` (31MB, with ONNX)
- `masday-windows-x86_64.exe` (12MB, no ONNX)
- `checksums-sha256.txt`

**Gap:** Tidak ada artifact macOS, tidak ada Windows installer script.

---

## Plan

### Phase 1: Perbaiki `detect_platform()` — Full OS Coverage

Extend detection untuk semua kombinasi:

```bash
detect_platform() {
    local os arch

    # OS detection
    case "$(uname -s 2>/dev/null || echo unknown)" in
        Linux*)   os="linux"  ;;
        Darwin*)  os="macos"  ;;
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

    # Architecture detection
    case "$(uname -m 2>/dev/null || echo unknown)" in
        x86_64|amd64)   arch="x86_64"   ;;
        arm64|aarch64)  arch="aarch64"  ;;
        *)
            err "Unsupported architecture: $(uname -m)"
            err "Supported: x86_64, aarch64 (arm64)"
            exit 1
            ;;
    esac

    echo "${os}-${arch}"
}
```

**Windows detection path:**
- Git Bash (MINGW64/MSYS) → paling umum untuk dev user
- MSYS2 → same detection
- Cygwin → legacy tapi masih ada
- WSL → terdeteksi sebagai Linux (sudah OK)

**Windows-specific handling:**
```bash
# Jika os="windows", append .exe ke binary name
if [ "$os" = "windows" ]; then
    artifact="masday-windows-${arch}.exe"
    BINARY_NAME="masday.exe"
else
    artifact="masday-${os}-${arch}"
fi
```

### Phase 2: Windows PATH Registration

Di Windows (Git Bash), PATH registration berbeda:

```bash
add_to_path() {
    local install_dir="$1"
    local os="$2"

    if [ "$os" = "windows" ]; then
        # Git Bash: update .bashrc / .bash_profile
        # Also try: echo 'export PATH="$PATH:$HOME/.masday/bin"' >> ~/.bashrc
        # For CMD/PowerShell: user needs to add manually via System Settings
        # But we can try via PowerShell in background:
        if command -v powershell.exe &>/dev/null; then
            local win_path
            win_path="$(cygpath -w "$install_dir")"
            powershell.exe -NoProfile -Command \
                "[Environment]::SetEnvironmentVariable('Path', [Environment]::GetEnvironmentVariable('Path', 'User') + ';${win_path}', 'User')" \
                2>/dev/null && ok "Added to Windows PATH (User scope)"
        fi
    fi

    # Unix PATH (Linux/macOS/WSL) — existing logic
    # ...existing .bashrc/.zshrc handling...
}
```

### Phase 3: Quickstart Auto-Execute

Saat ini sudah ada prompt "Run quickstart now?" di akhir install. Tinggal pastikan jalan di semua OS:

```bash
# Di akhir main()
if [ -t 0 ]; then
    read -rp "Run 'masday quickstart' now? [Y/n] " answer
    case "$answer" in
        n*|N*) echo "Okay. Run 'masday quickstart' when ready." ;;
        *)     "${INSTALL_DIR}/${BINARY_NAME}" quickstart ;;
    esac
else
    # Non-interactive (piped curl | bash)
    info "Run 'masday quickstart' to complete setup"
fi
```

**Untuk non-interactive mode (pipe):**
- Tambah flag `MASDAY_QUICKSTART=1` agar auto-run quickstart tanpa prompt
- `MASDAY_QUICKSTART=1 curl -fsSL ... | bash`

### Phase 4: Platform-Specific Artifact Mapping

```bash
# Map platform string ke nama artifact di GitHub Release
get_artifact_name() {
    local platform="$1"  # e.g. "linux-x86_64", "windows-x86_64", "macos-aarch64"

    local os="${platform%%-*}"
    local arch="${platform##*-}"

    case "$os" in
        linux)   echo "masday-linux-${arch}"         ;;
        windows) echo "masday-windows-${arch}.exe"    ;;
        macos)
            # macOS artifact belum ada
            err "macOS builds are not available yet."
            err "Track: https://github.com/dayartcrew-web/masday-workflow-release/issues"
            exit 1
            ;;
        *)       echo "masday-${os}-${arch}"          ;;
    esac
}
```

### Phase 5: Additional Improvements

1. **Fallback binary name detection** — cek GitHub Release assets, cari pattern `masday-{os}-*`
2. **Existing install detection** — jika sudah terinstall, tanya update atau skip
3. **Version flag** — `MASDAY_VERSION=v0.3.11` sudah ada, dokumentasi di header script
4. **Uninstall script** — `uninstall.sh` atau `masday uninstall` (sudah ada di binary)

---

## Progress

### ✅ Done
- [x] `release.yml` updated — 4-platform matrix (Linux, macOS ARM, macOS Intel, Windows)
- [x] Release ke public repo via `RELEASE_TOKEN` PAT
- [x] Windows MSVC target (no more mingw cross-compile dependency)
- [x] macOS builds (Intel + Apple Silicon) via GitHub Actions runners

### 🔲 TODO
- [ ] Setup `RELEASE_TOKEN` secret di private repo settings
- [ ] Extend `install.sh` (`scripts/install-masday.sh`) untuk Windows detection
- [ ] Windows PATH registration (PowerShell + Git Bash)
- [ ] Non-interactive `MASDAY_QUICKSTART=1` flag
- [ ] Test end-to-end: push tag → Actions build → release ke public repo

## Testing Checklist

- [ ] Linux x86_64 — `curl | bash` → download → install → quickstart
- [ ] Linux aarch64 — error gracefully (no artifact yet)
- [ ] Windows Git Bash — detect MINGW → download `.exe` → PATH registration
- [ ] WSL — detect as Linux (expected behavior)
- [ ] macOS — clear error message about missing build
- [ ] Non-interactive pipe — `MASDAY_QUICKSTART=1 curl | bash`
- [ ] Checksum verification works on all platforms

## Notes

- Windows binary saat ini 12MB (no ONNX, uses remote provider) — user akan di-info saat quickstart
- macOS cross-compile dari Linux belum feasible — butuh macOS runner atau self-hosted
- WSL user akan dapat Linux binary yang jalan native di WSL
- PowerShell PATH registration perlu user re-login untuk take effect
