Manage per-task git worktrees for isolated implementation.

## Purpose

Create isolated git worktrees for each task. Follows the finishing-a-development-branch pattern: verify tests → present options → execute choice → cleanup.

## Pre-conditions

- [ ] Git repo exists
- [ ] Active workflow with a task loaded (workflow.getCurrentTask)

## Commands

### /msd-worktree create

Create an isolated worktree for the current task.

```
Step 1: Get task info
  mcp__workflow-orchestrator__workflow_get_active({ cwd: process.cwd() })
  mcp__workflow-orchestrator__workflow_get_current_task({ workflow_id: workflowId })

  Record: task.id, task.title, task.acceptanceCriteria

Step 2: Generate branch name
  slug = task.title
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, '-')
    .replace(/^-|-$/g, '')
    .slice(0, 40)

  branch = "task/{slug}"
  worktreeDir = ".msd/worktrees/{slug}"

Step 3: Create worktree
  Bash: git worktree add {worktreeDir} -b {branch} HEAD

  If branch already exists:
    Bash: git worktree add {worktreeDir} {branch}

  If worktree dir already exists:
    Print: "Worktree already exists at {worktreeDir}"
    Skip to Step 4

Step 4: Record worktree info
  Write to .msd/worktrees/{slug}.json:
  {
    "taskId": "{task.id}",
    "taskTitle": "{task.title}",
    "branch": "{branch}",
    "worktreePath": "{worktreeDir}",
    "acceptanceCriteria": [...],
    "createdAt": "{ISO date}",
    "status": "active"
  }

Step 5: Report
  Print: "Worktree created: {worktreeDir} (branch: {branch})"
  Print: "Implement in that directory. Use /msd-worktree done when finished."
```

### /msd-worktree done

Finish the worktree. Follows finishing-a-development-branch pattern:
verify tests → present options → execute choice → cleanup.

```
Step 1: Find active worktree
  Read .msd/worktrees/{slug}.json (find the one matching current task)
  If not found → STOP: "No active worktree for current task."

Step 2: Auto-commit uncommitted changes
  Bash: cd {worktreePath} && git status --porcelain
  If changes exist:
    Bash: cd {worktreePath} && git add -A && git commit -m "feat: {task.title}"

Step 3: Verify Tests (MANDATORY — do not skip)
  Run the project's test suite from the worktree:
  Bash: cd {worktreePath} && {test command: pnpm test / pytest / go test / cargo test}

  If tests FAIL:
    Print: "Tests failing ({count} failures). Must fix before completing."
    STOP. Do NOT present options.

  If tests PASS: Continue to Step 4.

Step 4: Determine base branch
  Bash: cd {worktreePath} && git merge-base HEAD main 2>/dev/null || git merge-base HEAD master 2>/dev/null
  base = "main" or "master"

Step 5: Present Options (AskUserQuestion)
  Ask the user:

  "Implementation complete. What would you like to do?"

  Options:
  1. "Merge to {base} locally" — merge feature branch to base, delete branch
  2. "Push and create PR" — push to origin, create GitHub PR
  3. "Keep branch as-is" — keep worktree and branch for later
  4. "Discard this work" — delete branch and all changes

Step 6: Execute Choice
```

#### Option 1: Merge Locally

```
  Bash: cd {worktreePath} && git checkout {base}
  Bash: git pull
  Bash: git merge {branch}

  Verify tests on merged result:
  Bash: {test command}

  If tests pass:
    Bash: git branch -d {branch}
    Bash: git worktree remove {worktreePath}

  Update .msd/worktrees/{slug}.json:
    status: "merged-locally", completedAt: "{ISO date}"

  Report: "Merged to {base}. Branch {branch} deleted. Worktree cleaned up."
```

#### Option 2: Push and Create PR

```
  Bash: cd {worktreePath} && git push -u origin {branch}

  Bash: gh pr create
    --title "{task.title}"
    --body "$(cat <<'EOF'
    ## Summary
    {2-3 bullets from task description and changes}

    ## Changes
    {git diff --stat output}

    ## Acceptance Criteria
    - [ ] {criterion 1}
    - [ ] {criterion 2}

    ## Test Plan
    - [ ] All tests pass
    - [ ] Review against acceptance criteria

    ---
    Workflow: {workflow title} | Task: {task.title}
    EOF
    )"

  Record: PR URL and PR number

  DO NOT remove worktree yet — keep until PR is merged.
  Update .msd/worktrees/{slug}.json:
    status: "pr-created", prUrl: "{url}", completedAt: "{ISO date}"

  Save progress:
  mcp__workflow-orchestrator__workflow_save_progress({
    workflow_id: workflowId, task_id: taskId,
    agent_name: "msd-worktree",
    progress_note: "PR created: #{prNumber} — {prUrl}",
    evidence: ["PR: {prUrl}"]
  })

  Report: "PR #{prNumber} created: {url}. Worktree kept until merge."
```

#### Option 3: Keep As-Is

```
  DO NOT cleanup worktree. DO NOT delete branch.

  Update .msd/worktrees/{slug}.json:
    status: "kept"

  Report: "Branch {branch} preserved. Worktree at {worktreePath}."
  Report: "Run /msd-worktree done again when ready to merge."
```

#### Option 4: Discard

```
  Require confirmation first:
  AskUserQuestion: "This will permanently delete branch {branch} and all commits. Confirm?"

  If NOT confirmed → abort, return to options.

  If confirmed:
    Bash: git checkout {base}
    Bash: git branch -D {branch}
    Bash: git worktree remove {worktreePath}

    Update .msd/worktrees/{slug}.json:
      status: "discarded", completedAt: "{ISO date}"

    Report: "Discarded. Branch {branch} deleted."
```

### /msd-worktree list

```
Step 1: Glob: .msd/worktrees/*.json
Step 2: Print table:

  | # | Task | Branch | Status | PR |
  |---|------|--------|--------|----|
  | 1 | {title} | task/{slug} | active | — |
  | 2 | {title} | task/{slug} | pr-created | #{num} |

Step 3: Bash: git worktree list
```

### /msd-worktree clean

```
Step 1: Find worktrees with status in ["completed", "merged-locally", "discarded"]
Step 2: For each:
  Bash: git worktree remove {worktreePath} --force 2>/dev/null || true
  Delete the .json file
Step 3: Bash: git worktree prune
Step 4: Report: "Cleaned {count} worktrees"
```

## Quick Reference

| Option | Merge | Push | Keep Worktree | Delete Branch |
|--------|-------|------|---------------|---------------|
| 1. Merge locally | yes | — | no | yes |
| 2. Create PR | — | yes | yes | no |
| 3. Keep as-is | — | — | yes | no |
| 4. Discard | — | — | no | yes (force) |

## Integration with /msd-autopilot

When autopilot runs with worktrees enabled:
```
For each task:
  1. /msd-worktree create → isolated branch + directory
  2. Executor works inside worktree directory
  3. Review validates the changes
  4. If APPROVED:
     - In autopilot mode: auto-choose Option 2 (create PR)
     - In manual mode: run /msd-worktree done (user picks option)
  5. If REWORK → executor continues in same worktree
```

## Red Flags

- Never proceed with failing tests
- Never merge without verifying tests on merged result
- Never delete work without explicit confirmation (Option 4)
- Never force-push without explicit request

## Error Handling

| Error | Action |
|-------|--------|
| Branch name conflict | Append task ID to make unique |
| Worktree dir exists | Reuse if active, error if stale |
| gh not authenticated | Fall back to Option 1 (merge locally) |
| Tests fail on done | STOP — must fix before presenting options |
