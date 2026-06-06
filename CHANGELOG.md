# Changelog

All notable changes to Masday CLI are documented here.

## [v0.3.39] - 2026-06-06

### Removed
- `pre-tool-use.js` hook — redundant with `workflow-lock.js`
- `tdd-guard.js` hook — redundant with `skill-step-guard.cjs`
- `post-tool-use.js` hook — redundant with `skill-wrap-guard.js`
- `tool-name-guard.js` hook — broken (reads wrong file, false positives on `use_masday`)
- `on-notification.js` hook — empty no-op

### Fixed
- `masday-context-warning.cjs`: 90% threshold no longer blocks session (`continue:false` → `systemMessage`)
- `workflow-lock.js`: only fires when `.masday/state.json` has an active workflow (was firing on every edit)
- `skill-wrap-guard.js`: message shortened from 300+ chars to single-line reminder

### Changed
- `settings.json`: removed `PostToolUse` and `Notification` hook sections, merged `PreToolUse` matchers
- `release.sh`: install.sh now **mandatory** (errors if missing instead of silent skip)
- Release notes template: one-liner at top, environment variables table, PowerShell example

## [v0.3.38] - 2026-06-06

### Fixed
- `masday status` now counts only masday-owned agents, skills, and hooks (was counting all third-party assets)
- Agents counted from embedded templates filtered by `masday-`/`msd-` prefix (was 29 → 28)
- Skills counted from embedded templates filtered by `masday-`/`msd-` prefix (was 91 → 38)
- Hooks counted from disk filtered by `masday-` prefix (was 11 → 10)

## [v0.3.37] - 2026-06-06

### Fixed
- accept `branch_key` in `completeParallelBranch` (schema/impl mismatch)
- non-blocking PG sync — prevent MCP hang on `workflow_create`

### Performance
- speed up install script — gh CLI auth, shorter timeouts, non-fatal checksum

## [v0.3.36] - 2026-06-06

### Style
- cargo fmt fix for pg.rs (CI format check)

## [v0.3.33] - 2026-06-06

### Fixed
- update release repo to source repo (`dayartcrew-web/masday-workflow-rust`)
- remove `--all-features` from CI to avoid ONNX Runtime hang
- provide dummy static params for dynamic routes in dashboard
- split dynamic route pages into server + client components
- add `generateStaticParams` to dynamic routes for static export
- rewrite CI workflow to match actual project structure
- drop Node 20 from CI matrix (pnpm 11 requires Node >= 22.13)
- resolve CI/CD pipeline failures across all 3 workflows
- `workflow_execute` state validation + auto-transition + `skill_sync` global skip
- install.sh add curl timeouts + progress visibility
- changelog generation + install.sh version detection fallback
- masday-cli tests updated for streamableHttp transport + health logic

### Changed
- bump version to 0.3.33

## [v0.3.32] - 2026-06-05

### Fixed
- tags column is TEXT[] not JSONB — send Vec<String>
- PostgreSQL sync — use serde_json::Value for jsonb columns
- track migration SQL file in git (was ignored by *.sql pattern)
- use CARGO_MANIFEST_DIR for include_str! path
- embed PostgreSQL migrations in binary

## [v0.3.31] - 2026-06-05

### Changed
- PostgreSQL on-demand connect, not at startup

## [v0.3.30] - 2026-06-05

### Added
- MCP local mode Phase 2 — tools sync to PostgreSQL on-demand

## [v0.3.29] - 2026-06-05

### Fixed
- read database_url from config.toml in MCP pg.rs

## [v0.3.28] - 2026-06-05

### Added
- MCP local mode — PostgreSQL + SQLite dual mode
- enable Local mode in all builds (remove dev-mode feature gate)

## [v0.3.27] - 2026-06-05

### Added
- auto-generate changelog in release body from commit history

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
