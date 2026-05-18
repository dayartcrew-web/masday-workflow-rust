Create a GitHub Pull Request for the current or completed task.

## Purpose

Create a PR from a task branch (worktree) to main. Called automatically by `/msd-worktree done` or manually after review approval.

## Pre-conditions

- [ ] Git repo with remote on GitHub
- [ ] `gh` CLI authenticated (`gh auth status`)
- [ ] Current branch is NOT main (must be on task branch)
- [ ] All changes committed

If any fails → STOP with instructions to fix.

## Steps

### 1. Gather Context
```
Step 1a: Get task info
  mcp__workflow-orchestrator__workflow_get_active({ cwd: process.cwd() })
  mcp__workflow-orchestrator__workflow_get_current_task({ workflow_id: workflowId })

  Record: task.id, task.title, task.acceptanceCriteria

Step 1b: Get branch info
  Bash: git branch --show-current → current branch name
  Bash: git log main..HEAD --oneline → commits on this branch
  Bash: git diff main...HEAD --stat → files changed summary

  If current branch IS main → STOP: "Switch to a task branch first."
```

### 2. Build PR Content
```
Title: task.title (imperative, under 72 chars)

Body sections:
1. Summary: What and why (from task description)
2. Changes: Files modified/created (from git diff)
3. Acceptance Criteria: Copied from task, as checklist
4. Test Plan: How to verify (from verification steps)
```

### 3. Push Branch
```
Bash: git push -u origin {branch-name}

If push fails due to remote not existing:
  STOP: "Remote not configured. Run: git remote add origin {url}"

If push fails due to auth:
  STOP: "Run: gh auth login"
```

### 4. Create PR
```
Bash: gh pr create \
  --title "{title}" \
  --body "$(cat <<'EOF'
  ## Summary
  {1-2 sentences what and why}

  ## Changes
  {file list from git diff --stat}

  ## Acceptance Criteria
  - [ ] {criterion 1}
  - [ ] {criterion 2}
  - [ ] {criterion 3}

  ## Test Plan
  - [ ] {verification step 1}
  - [ ] {verification step 2}

  ---
  Workflow: {workflow title} | Task: {task.title}
  EOF
  )"

Record: PR URL and PR number
```

### 5. Save Progress
```
mcp__workflow-orchestrator__workflow_save_progress({
  workflow_id: workflowId,
  task_id: taskId,
  agent_name: "msd-pr",
  progress_note: "PR created: #{prNumber} — {prUrl}",
  evidence: ["PR: {prUrl}"],
  status_after: "reviewing"
})

Save PR info to .msd/ using Write tool:
Write to: .msd/reports/pr-{task-slug}.md
Content:
  ## PR Report
  Task: {title}
  PR: #{number} — {url}
  Branch: {branch} → main
  Commits: {count}
  Files changed: {count}
```

### 6. Report
```
Print:
  PR Created: #{number}
  URL: {url}
  Branch: {branch} → main
  Commits: {count} ({file count} files)

  Next: Review on GitHub, then merge or /msd-worktree done
```

## Options

The command accepts these arguments:

- **draft**: Create as draft PR (`--draft` flag)
- **reviewer**: Assign reviewer (`--reviewer @username`)
- **labels**: Add labels (`--label bug,feature`)

If `args` contains "draft" → add `--draft` flag.
If `args` contains "reviewer=X" → add `--reviewer X`.

## Error Handling

| Error | Action |
|-------|--------|
| Not on task branch | STOP: switch to task branch or use /msd-worktree |
| gh not installed | STOP: install GitHub CLI |
| gh not authenticated | STOP: run `gh auth login` |
| Push rejected | Pull/rebase first, then retry |
| PR already exists | Print existing PR URL, skip creation |
| No commits to PR | STOP: commit changes first |
