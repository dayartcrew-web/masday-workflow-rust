# AGENTS

## Release Process

⚠️ **Read [../docs/release-guide.md](../docs/release-guide.md) before touching releases.**

- **CI (`.github/workflows/release.yml`) is the active, recommended release path.** It triggers on tag push (`v*`), builds Linux x86_64 + macOS aarch64 + Windows x86_64, and publishes to GitHub Releases (~6 min). No manual release needed.
- **To release:** bump the version in all 6 crate `Cargo.toml` files + the matching `Cargo.lock` entries, commit as `chore: bump version to 0.3.XX`, then push master + the `v0.3.XX` tag. CI publishes automatically.
- `scripts/release.sh` is a **fallback only** — Linux + Windows, no macOS — for when CI is unavailable.
- Releases are published to **source repo** GitHub Releases: `dayartcrew-web/masday-workflow-rust/releases`
- **Do NOT create releases with version 0.7x** — use 0.x.x format
- Source repo: `dayartcrew-web/masday-workflow-rust` (private)

## Masday-First Protocol

1. **masday MCP tools** → `mcp__masday__*` for workflow, memory, search, policy, capability
2. **Sub-agents** → Agent tool for parallel/independent work
3. **masday skills** → `masday-*` skills for structured workflows
4. **Other skills** → superpowers, ecc, navigator (fallback)

| Need | Tool |
|------|------|
| ANY instruction | `mcp__masday__use_masday` |
| Workflow CRUD | `mcp__masday__workflow_create` |
| Search code | `mcp__masday__semantic-search_code_search` |
| Memory | `mcp__masday__memory_store` / `memory_search` |
| Agent routing | `mcp__masday__capability_match_agent` |
| Review | `mcp__masday__review_submit` |
| Verify done | `mcp__masday__policy_validate_completion` |

### Non-Masday Skill Wrap

After ANY non-masday skill: `workflow_saveProgress` → `review_submit` → `policy_validate_completion` → `workflow_completeTask` → `memory_store`

## Step Enforcement

Hooks enforce step ordering for 30 skills. Key chains:
- **TDD**: RED → RED_VERIFY → GREEN → GREEN_VERIFY → REFACTOR → COVERAGE
- **Workflow new**: 8 steps (enforced by `masday-skill-checkpoint.js`)
- **Workflow run**: 5 steps | **plan**: 4 | **fix**: 4 | **verify**: 5
