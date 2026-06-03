# Release Guide

> **IMPORTANT:** This is the ONLY way to publish releases. Do NOT use CI workflows.

## Release Process

All releases are built **locally on VPS** and uploaded manually via `gh release`.

### One command to release:

```bash
cd ~/masday-workflow-release && bash release.sh v0.x.x
```

This builds **Linux x86_64** and **Windows x86_64** binaries, generates checksums, and uploads to the public release repo.

### Options

```bash
bash release.sh v0.x.x --dry-run      # Build only, don't upload
bash release.sh v0.x.x --linux-only   # Build Linux only (skip Windows cross-compile)
```

### Prerequisites

- VPS has `gh` CLI authenticated (`gh auth status`)
- VPS has `x86_64-w64-mingw32-gcc` for Windows cross-compile
- Source repo: `~/masday-workflow-rust/` (must be up to date)
- Release repo: `~/masday-workflow-release/`

## Repositories

| Repo | Visibility | Purpose |
|------|-----------|---------|
| `dayartcrew-web/masday-workflow-rust` | **Private** | Source code |
| `dayartcrew-web/masday-workflow-release` | **Public** | Binary releases + install.sh |

## What `release.sh` Does

1. `cargo build -p masday-cli --release` (Linux, with ONNX embeddings)
2. `cargo build -p masday-cli --release --target x86_64-pc-windows-gnu --no-default-features` (Windows, no ONNX)
3. `strip` binaries
4. Generate `checksums-sha256.txt`
5. `gh release create` on `masday-workflow-release` repo

## Binary Contents

Single binary `masday` contains everything:
- CLI commands: `quickstart`, `install`, `setup`, `serve`, `mcp`, `status`, `db`, `update`
- 28 agents (embedded)
- 30+ skills (embedded)
- 7 hooks (embedded)
- MCP server (`masday mcp`)

**No separate `masday-mcp` binary needed.**

## Versioning

Use **0.x.x** format. Do NOT use 0.7x or other schemes — those were from old CI workflow.

```
v0.1.0  Initial release
v0.2.0  Quickstart wizard
v0.3.0  Hooks rewrite
v0.3.4  Latest
...
```

## CI Workflows — DISABLED

The following workflows exist in `.github/workflows/` but are **disabled**:

- `release.yml` — was auto-publishing to release repo with old binary format (separate `masday-mcp`)
- `ci.yml` — unused
- `rust-ci.yml` — unused

**Do NOT re-enable these.** They conflict with manual `release.sh`.

If you need to check:
```bash
cd ~/masday-workflow-rust && gh workflow list
```

## User Install (One-liner)

```bash
# Linux/macOS
curl -fsSL https://raw.githubusercontent.com/dayartcrew-web/masday-workflow-release/main/install.sh | bash

# Windows PowerShell
Invoke-WebRequest -Uri "https://github.com/dayartcrew-web/masday-workflow-release/releases/latest/download/masday-windows-x86_64.exe" -OutFile "masday.exe"
.\masday.exe quickstart
```

## Windows Notes

- Built via mingw cross-compiler (`x86_64-pc-windows-gnu`)
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
└── compact-state.json # Hook state (auto-created)
```

## Config Fields

```toml
mode = "local"                    # local | remote | standalone
api_url = "http://localhost:30101"
api_key = "***"
database_url = "postgresql://..."
embedding_provider = "local"       # local | ollama | openai
embedding_model = "all-MiniLM-L6-v2"
embedding_dimensions = 384
api_port = 30101
db_port = 54341
redis_port = 63791
dashboard_port = 30101
platforms = ["claude-code"]
```
