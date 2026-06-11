# Plan: Port /msd-worktree → /masday-worktree

## Overview

Port the `msd-worktree` command from [msd-mcp](https://github.com/dayartcrew-web/msd-mcp) to masday-workflow-rust as a skill (`masday-worktree`).

Provides per-task git worktree isolation: each workflow task gets its own branch + worktree directory, enabling concurrent feature development without conflicts.

## Current State (msd-mcp)

| Feature | Status |
|---------|--------|
| `/msd-worktree create` | Create worktree + branch per task |
| `/msd-worktree done` | Verify tests → merge/PR/keep/discard |
| `/msd-worktree list` | Show all worktrees with status |
| `/msd-worktree clean` | Remove completed worktrees |
| Autopilot integration | Auto-create PR on APPROVED |

## Implementation Plan

### Phase 1: Create Skill File

**File:** `.claude/skills/masday-worktree/SKILL.md`

Port the 4 sub-commands from msd-worktree.md:
- `create` — git worktree add, branch from HEAD, record metadata JSON
- `done` — auto-commit → verify tests → present 4 options → execute
- `list` — glob `.masday/worktrees/*.json` + git worktree list
- `clean` — remove completed/discarded worktrees

Key differences from msd-mcp:
- Path: `.msd/worktrees/` → `.masday/worktrees/`
- MCP tools: `mcp__workflow-orchestrator__*` → `mcp__masday__*`
- Branch naming: `task/{slug}` → `task/{slug}`
- Agent name: `msd-worktree` → `masday-worktree`

### Phase 2: Update .gitignore

Add `.masday/worktrees/` to `.gitignore` (worktree state is local).

### Phase 3: Update Agent Routing

Add `masday-worktree` to relevant skill lists in:
- `masday-autopilot/SKILL.md` — autopilot can use worktree mode
- `masday-git-workflow/SKILL.md` — git operations include worktree
- `.claude/CLAUDE.md` — add to available skills list

### Phase 4: Test

Manual test flow:
1. Create workflow + add task
2. `/masday-worktree create` → verify branch + worktree created
3. Make changes in worktree
4. `/masday-worktree done` → verify tests → merge/PR
5. `/masday-worktree list` → verify status
6. `/masday-worktree clean` → verify cleanup

## Files to Create/Modify

| Action | File |
|--------|------|
| CREATE | `.claude/skills/masday-worktree/SKILL.md` |
| MODIFY | `.gitignore` (add `.masday/worktrees/`) |
| MODIFY | `.claude/CLAUDE.md` (add skill reference) |

## Sub-commands

### `/masday-worktree create`
1. Get current task via `mcp__masday__workflow_getCurrentTask`
2. Generate branch: `task/{slug}` from task title
3. `git worktree add .masday/worktrees/{slug} -b task/{slug} HEAD`
4. Write `.masday/worktrees/{slug}.json` with task metadata
5. Report worktree path + branch

### `/masday-worktree done`
1. Find active worktree JSON matching current task
2. Auto-commit uncommitted changes
3. Run tests (MANDATORY — stop if failing)
4. Present 4 options: merge locally / push+PR / keep / discard
5. Execute chosen option + cleanup

### `/masday-worktree list`
1. Glob `.masday/worktrees/*.json`
2. Print table: task, branch, status, PR
3. `git worktree list`

### `/masday-worktree clean`
1. Find worktrees with completed/merged/discarded status
2. `git worktree remove --force` + delete JSON
3. `git worktree prune`

## Autopilot Integration

When autopilot runs with worktrees:
```
For each task:
  /masday-worktree create → isolated branch + directory
  Executor works inside worktree
  Review validates changes
  APPROVED → auto PR (Option 2)
  REWORK → executor continues in same worktree
```

## Acceptance Criteria

- [ ] Skill file created with all 4 sub-commands
- [ ] `.gitignore` updated
- [ ] `masday update` syncs the skill to all platforms
- [ ] Manual test: create → implement → done → list → clean
