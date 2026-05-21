# AGENTS

## Mandatory Intake and Workflow Protocol

Before any substantive response or action:

1. Re-read the relevant `AGENTS.md` instructions for the current task.
2. **Check masday MCP tools first** — if a masday MCP tool can handle the task directly, use it before anything else (e.g., `mcp__masday__workflow_create`, `mcp__masday__memory_search`, `mcp__masday__semantic-search_code_search`, `mcp__masday__capability_match_agent`).
3. **Use agent orchestrator** — for multi-step work, delegate to the appropriate masday agent via `mcp__masday__capability_match_agent` or dispatch a specialized sub-agent via the Agent tool.
4. **Use sub-agents** — for independent parallel tasks, dispatch sub-agents (Agent tool) rather than doing everything sequentially.
5. **Check masday skills** — search for `masday-*` skills in the available skills list and invoke the best matching one via the Skill tool.
6. Only then search for other relevant skills (superpowers, ecc, navigator, etc.).

### Priority Order (Masday Ecosystem First)

```
1. masday MCP tools      → Direct tool calls (workflow, memory, search, policy, capability)
2. Agent orchestrator    → mcp__masday__capability_match_agent for task routing
3. Sub-agents            → Agent tool for parallel/independent work
4. masday skills         → masday-workflow-*, masday-research, masday-create-*, etc.
5. Other skills          → superpowers, ecc, navigator, etc. (fallback)
```

### When to Use What

| Situation | First Choice | Fallback |
|-----------|--------------|----------|
| Create/manage workflow | `mcp__masday__workflow_create` | `masday-workflow-new` skill |
| Search codebase | `mcp__masday__semantic-search_code_search` | Grep/Glob tools |
| Store/recall memory | `mcp__masday__memory_store` / `mcp__masday__memory_search` | File-based memory |
| Route to agent | `mcp__masday__capability_match_agent` | Agent tool directly |
| Research | `masday-research` / `masday-web-research` skill | WebSearch tool |
| Parallel tasks | `masday-parallel-execution` / `masday-parallel-research` skill | Agent tool dispatch |
| Code analysis | `mcp__masday__capability_system_readiness` | Manual exploration |
| Review code | `mcp__masday__review_submit` | code-reviewer agent |
| Verify completion | `mcp__masday__policy_validate_completion` | verification skill |

For non-trivial work, follow this order:

1. Analyze the request and local instructions
2. **Try masday MCP tools first** — check if `mcp__masday__*` tools can handle it
3. Research codebase patterns via `mcp__masday__semantic-search_code_search`
4. Plan via `masday-workflow-plan` or `masday-workflow-new` skill
5. Execute with masday agents/sub-agents for multi-step work
6. Use `test-driven-development` before testable code changes
7. Use `verification-before-completion` before any completion claim

### Non-Masday Skill Wrap Rule

When using ANY non-masday skill (superpowers, ecc, navigator, etc.), you MUST return to the masday pipeline after the skill completes:

```
Non-masday skill completes
  → workflow_startTask (if in workflow)
  → workflow_saveProgress (log what the skill did)
  → review_submit (quality gate)
  → policy_validate_completion (check readiness)
  → workflow_completeTask (close task)
  → memory_store (persist findings)
```

**Why:** Non-masday skills operate outside the workflow pipeline. Without wrapping back, progress is lost — no review, no persistence, no audit trail. Every external skill output must be captured into the masday lifecycle.

**Examples:**
- `brainstorming` skill finishes → `mcp__masday__workflow_saveProgress` + `mcp__masday__memory_store` the design decisions
- `test-driven-development` skill finishes → `mcp__masday__review_submit` + `mcp__masday__workflow_completeTask`
- `code-review` skill finishes → `mcp__masday__review_submit` the findings + `mcp__masday__workflow_saveProgress`
- `systematic-debugging` skill finishes → `mcp__masday__memory_store` the root cause + `mcp__masday__workflow_saveProgress`

If unsure whether a task is trivial, treat it as non-trivial.

## Skills

Skills are auto-discovered by the Claude Code Skill tool. Full list available in system-reminder.

**Skill invocation priority (follows masday-first rule):**
1. `masday-*` skills first (masday-workflow-new, masday-research, masday-create-*, etc.)
2. Process skills (brainstorming, systematic-debugging, tdd)
3. Implementation skills (executing-plans, writing-plans, verification)
4. Platform skills (ecc:*, navigator:*, superpowers:*) — last resort

**Key skill triggers:**
- Before creative work → `brainstorming`
- Before writing code → `test-driven-development`
- Before claiming done → `verification-before-completion`
- Bug/failure → `systematic-debugging`
- Multi-step plan → `executing-plans` or `masday-workflow-new`
- After any non-masday skill → wrap back to masday pipeline (see Non-Masday Skill Wrap Rule above)

## Pre-Commit Discipline (No Hooks Available)

GitHub Copilot coding agent does not support Claude-style hooks. You MUST enforce these checks manually before every commit:

1. **Typecheck**: Run `pnpm typecheck` — zero errors
2. **Tests**: Run `pnpm test` — all tests pass
3. **No secrets**: No hardcoded API keys, passwords, or tokens
4. **No console.log**: No debug statements in production code
5. **Review pipeline**: After completing a task in a workflow:
   - `review_submit` → `policy_validate_completion` → `workflow_completeTask`
6. **Tool names**: All MCP tool names use underscore format (`workflow_create`, not `workflow.create`)

## Universal Guard Script

Since Claude Code hooks are NOT available in GitHub Copilot, use the guard script to replicate their behavior:

```bash
# Before editing a source file — checks TDD, secrets, console.log
node scripts/guard.mjs pre-edit <file-path>

# After editing a source file — reminds to saveProgress/review
node scripts/guard.mjs post-edit <file-path>

# Before completing a workflow task — checks review gate
node scripts/guard.mjs pre-complete
```

**You MUST run `pre-edit` before writing code and `post-edit` after writing code.** This replaces the Claude hooks that enforce TDD, secret detection, and the review pipeline.

## Git Hooks (Cross-Platform)

Git hooks are installed via `scripts/install-hooks.sh` and run automatically:
- `pre-commit`: guard checks + typecheck + tests
- `pre-push`: build + full test suite

Install hooks: `bash scripts/install-hooks.sh`
