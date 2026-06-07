# Release Guide

> **IMPORTANT:** This is the ONLY way to publish releases. Do NOT use CI workflows.

## Release Process

All releases are built **locally on VPS** and uploaded via `gh release`.

### One command to release:

```bash
bash scripts/release.sh v0.x.x
```

This builds **Linux x86_64** and **Windows x86_64** binaries for both `masday` (CLI) and `masday-mcp` (standalone MCP server), generates checksums, and uploads to GitHub Releases.

### Options

```bash
bash scripts/release.sh v0.x.x --dry-run      # Build only, don't upload
bash scripts/release.sh v0.x.x --linux-only   # Build Linux only (skip Windows cross-compile)
```

### Prerequisites

- VPS has `gh` CLI authenticated (`gh auth status`)
- VPS has `x86_64-w64-mingw32-gcc` for Windows cross-compile
- Working directory is the project root (`masday-workflow-rust/`)

## Repository

| Repo | Visibility | Purpose |
|------|-----------|---------|
| `dayartcrew-web/masday-workflow-rust` | **Private** | Source code + releases |

## Git Hooks

The project uses pre-commit and pre-push hooks for quality gates.

### pre-commit hook (`.git/hooks/pre-commit`)

Runs on every `git commit`. Fast checks only (~5-10s):

| Stack | Checks |
|-------|--------|
| Rust | `cargo fmt --check`, `cargo clippy` |
| TypeScript | `pnpm lint` |
| Python | `ruff check` |
| Go | `go vet` |
| Docs | Dead link check, crate references |

### pre-push hook (`.git/hooks/pre-push`)

Runs on every `git push`. Quality gate (~1-2 min):

| Stack | Checks |
|-------|--------|
| Rust | `cargo fmt --check`, `cargo clippy`, `cargo check`, `cargo test` |
| TypeScript | `pnpm build`, `pnpm test` |
| Python | `pytest` |
| Go | `go vet`, `go test` |

**Note:** Pre-push does NOT run `cargo build --release`. Release builds are only for actual releases via `release.sh`. The hook uses `cargo check` (fast, incremental) instead.

### Skipping hooks

```bash
git commit --no-verify    # Skip pre-commit
git push --no-verify      # Skip pre-push (use sparingly)
```

### Hook installation

Hooks are installed via `masday install` or manually:

```bash
# From project root
cp .agents/hooks/pre-commit .git/hooks/pre-commit
cp .agents/hooks/pre-push .git/hooks/pre-push
chmod +x .git/hooks/pre-commit .git/hooks/pre-push
```

## What `release.sh` Does

1. `cargo build -p masday-cli --release` (Linux, with ONNX embeddings)
2. `cargo build -p masday-cli --release --target x86_64-pc-windows-gnu --no-default-features` (Windows, no ONNX)
3. `cargo build -p masday-mcp --release` (Linux standalone)
4. `cargo build -p masday-mcp --release --target x86_64-pc-windows-gnu --no-default-features` (Windows standalone)
5. `strip` all binaries
6. Generate `checksums-sha256.txt`
7. Copy `install-masday.sh` → `install.sh` (**mandatory** — errors if missing)
8. `gh release create` on source repo

## Release Artifacts

| Binary | Linux | Windows | Size | Description |
|--------|-------|---------|------|-------------|
| **masday** (CLI) | `masday-linux-x86_64` | `masday-windows-x86_64.exe` | ~39MB / ~15MB | CLI + MCP server wrapper |
| **masday-mcp** (standalone) | `masday-mcp-linux-x86_64` | `masday-mcp-windows-x86_64.exe` | ~3.3MB / ~2.9MB | Lightweight MCP server |
| **install.sh** | — | — | ~8KB | Multi-platform installer |
| **checksums-sha256.txt** | — | — | ~0.5KB | SHA-256 checksums |

## Binary Contents

The `masday` binary contains everything:
- CLI commands: `quickstart`, `install`, `setup`, `serve`, `mcp`, `status`, `db`, `update`, `embed`, `doctor`, `config`, `dev`
- MCP server (`masday mcp`) — wraps `masday-mcp` crate
- 28 agents (embedded)
- 30+ skills (embedded)
- 10 hooks (embedded)
- 90 MCP tools across 20 domains

The standalone `masday-mcp` binary is for lightweight PATH-only deployments.

## Install.sh (Mandatory)

Every release **MUST** include `install.sh`. The release script errors if `scripts/install-masday.sh` is missing.

The install script auto-detects:
- **OS:** Linux, macOS, Windows (Git Bash/MSYS2)
- **Architecture:** x86_64, aarch64 (arm64)
- **Existing install:** auto-updates if newer version available
- **Checksum verification:** SHA-256 from `checksums-sha256.txt`

## Versioning

Use **0.x.x** format. Do NOT use 0.7x or other schemes — those were from old CI workflow.

## CI Workflows — STATUS

| Workflow | Status | Purpose |
|----------|--------|---------|
| `ci.yml` | ✅ Active | Dashboard lint + build on push/PR |
| `release.yml` | ✅ Active | Cross-platform release on tag push |
| `rust-ci.yml` | ❌ Disabled | Was old Rust CI |

## Release Flow: Tag → CI

**Recommended:** Push a tag, let CI build all platforms.

```bash
# 1. Update version in Cargo.toml files
#    (or let CI auto-sync from tag)

# 2. Commit and push
git add -A
git commit -m "release: v0.x.x"
git push origin main

# 3. Tag and push — triggers CI release
git tag v0.x.x
git push origin v0.x.x
```

CI will build:

| Artifact | Platforms |
|----------|-----------|
| `masday` (CLI) | Linux x86_64, macOS aarch64, Windows x86_64 |
| `masday-mcp` (standalone) | Linux x86_64, macOS aarch64, Windows x86_64 |
| `install.sh` | Included in release |
| `checksums-sha256.txt` | Auto-generated |

### Manual dispatch (no tag needed)

```bash
gh workflow run release.yml -f version=v0.x.x
```

### VPS local release (fallback)

Use `release.sh` if CI is down or for Linux + Windows only:

```bash
bash scripts/release.sh v0.x.x
bash scripts/release.sh v0.x.x --linux-only
```

### CI vs VPS release

| | CI (recommended) | VPS (`release.sh`) |
|---|---|---|
| Platforms | Linux + macOS + Windows | Linux + Windows |
| macOS support | ✅ Native runner | ❌ No |
| masday-mcp builds | ✅ All platforms | ✅ Linux + Windows |
| Disk usage on VPS | None | 5-12G |
| Speed | ~10-15 min | ~10 min |
| Trigger | Tag push or manual | Manual |

## User Install (One-liner)

```bash
# Linux/macOS/Windows Git Bash
curl -fsSL https://github.com/dayartcrew-web/masday-workflow-rust/releases/latest/download/install.sh | bash

# Windows PowerShell
Invoke-WebRequest -Uri "https://github.com/dayartcrew-web/masday-workflow-rust/releases/latest/download/masday-windows-x86_64.exe" -OutFile "masday.exe"
.\masday.exe quickstart
```

## Environment Variables for Install

| Variable | Default | Description |
|----------|---------|-------------|
| `MASDAY_VERSION` | latest | Install specific version |
| `MASDAY_QUICKSTART` | 0 | Auto-run quickstart after install |
| `MASDAY_FORCE` | 0 | Force reinstall same version |

## Developer Setup

```bash
git clone https://github.com/dayartcrew-web/masday-workflow-rust
cd masday-workflow-rust

# Build
cargo build --workspace

# MCP config (dev mode — uses cargo run directly)
# .mcp.json is already configured for dev:
#   cargo run -p masday-mcp --bin masday-mcp

# Quickstart
cargo run -p masday-cli -- quickstart --dev
```

## Windows Notes

- Built via mingw cross-compiler (`x86_64-w64-mingw32-gcc`)
- **No local ONNX embeddings** — `ort-sys` doesn't support cross-compile
- Users must configure remote embedding provider (Ollama/OpenAI)
- Binary auto-installs to `%USERPROFILE%\.masday\bin\` on first run
- `masday quickstart` shows manual embedding config hint on Windows

## Config Location

```
~/.masday/
├── config.toml       # Configuration (ports, mode, api_url, etc.)
├── bin/
│   └── masday        # CLI binary
└── data.db           # SQLite database (auto-created)
```
