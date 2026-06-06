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

## CI Workflows — DISABLED

The following workflows exist in `.github/workflows/` but are **disabled**:

- `release.yml` — was auto-publishing with old binary format
- `ci.yml` — unused
- `rust-ci.yml` — unused

**Do NOT re-enable these.** They conflict with manual `release.sh`.

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
