# AGENTS

## Release Process

⚠️ **Read [docs/release-guide.md](docs/release-guide.md) before touching releases.**

- Releases are published to **source repo** GitHub Releases: `dayartcrew-web/masday-workflow-rust/releases`
- Built locally via `~/masday-workflow-release/release.sh`
- **Do NOT use CI workflows** — they are disabled
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
