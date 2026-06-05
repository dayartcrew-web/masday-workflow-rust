# Changelog

All notable changes to Masday CLI are documented here.

## [v0.3.23] - 2026-06-05

### Security
- Git history rewritten: all credentials removed from entire git history
- GitGuardian findings resolved (Generic Password, PostgreSQL URI, Database Assignment)

## [v0.3.22] - 2026-06-05

### Changed
- Release pipeline now publishes to source repo (no separate release repo needed)
- Install script URL updated to `masday-workflow-rust/scripts/install-masday.sh`
- All `PLACEHOLDER`/`CHANGE_ME_REMOVED` placeholders replaced with `CHANGE_ME` (GitGuardian fix)

### Fixed
- Windows `update.rs`: cfg conditional for `bail!()` (no unreachable statement on non-Windows)
- `status.rs`: removed unused `gemini_home` variable (clippy fix)

## [v0.3.19] - 2026-06-05

### Added
- MCP status shows actual tool count (90 tools) parsed from binary startup log
- MCP mode detection (stdio / local / remote) from `~/.claude.json`
- Multi-platform auto-detection (claude-code, gemini, opencode, vscode)
- `CODEOWNERS` and `SECURITY.md` for public repo

### Fixed
- `masday status` counts agents/skills/hooks from `~/.claude/` (was showing 0)
- MCP registration check looks in `~/.claude.json` (was only checking `~/.claude/settings.json`)
- Health status logic: API healthy + non-core issues = "Partial degradation" (was "Critical failure")
- Windows binary update: handle locked executable (access denied os error 5)

## [v0.3.15] - 2026-06-05

### Added
- Real embedding download via `ollama pull` (was simulated/mock)
- Real embedding test via Ollama API and OpenAI API (was fake vector)
- `embedding_base_url` and `embedding_api_key` config fields
- DB status: real PostgreSQL connection attempt (not just port check)
- Redis status: TCP connect + PING command (not just Docker check)

### Fixed
- MCP status checks registration in platform settings
- Config `get` masks API key values for security

## [v0.3.14] - 2026-06-05

### Added
- Redis health check via TCP + PING fallback
- Database health check with TCP port fallback when `database_url` not set
- MCP health checks binary path + registration in Claude Code settings

### Fixed
- All crate versions bumped from 0.1.0 to 0.3.13

## [v0.3.13] - 2026-06-04

### Added
- Cross-platform release pipeline: Linux x86_64, macOS aarch64, macOS x86_64, Windows x86_64
- GitHub Actions workflow builds 4 platforms and publishes to public release repo
- `install.sh` cross-platform OS detection (Linux, macOS, Windows Git Bash)
- `install.sh` auto-update when installed version differs from latest

### Fixed
- Windows build: unused variable warnings in `add_to_path()`
- macOS Intel build: use `macos-latest` runner (was `macos-13`, limited availability)
- Rust toolchain: correct `target` parameter (was `targets`)
- ONNX/ort-sys: skip for non-Linux platforms via `default-features = false`
- Release publish: use `token` input for `softprops/action-gh-release`

### Security
- PAT token configured as `RELEASE_TOKEN` secret for cross-repo publishing
