---
name: masday-cicd-check
description: >
  Check CI/CD pipeline status, inspect build results, and run quick security checks.
  Lightweight read-only check for current pipeline state and dependency health.
  Use when the user asks "check CI", "pipeline status", "build results",
  "CI failed", "is the build passing", "check security", or "audit deps".
allowed-tools:
  - cicd_pipeline_status
  - cicd_runs_view
  - cicd_pipeline_trigger
  - github_pr_list
  - github_issue_list
  - Bash
  - Read
  - Grep
  - Glob
  - memory_store
  - memory_recall_recent
  - memory_search
---

# Masday CI/CD Check

Quick CI/CD pipeline status check with security audit.

## Steps

### 1. Check Pipeline Status

```
# Get recent runs
cicd_pipeline_status({ branch: "main", limit: 5 })

# Get runs for current branch
cicd_pipeline_status({ limit: 5 })
```

Show:
- Last 5 runs for context
- Current branch's latest run
- Status: passing / failing / in-progress
- Duration and commit SHA

### 2. Inspect Failed Runs

```
# Get step-by-step results for a specific run
cicd_runs_view({ runId: 12345 })

# Read the workflow that failed
Read({ file_path: ".github/workflows/ci.yml" })
```

Highlight:
- Failed steps with error messages
- Step durations for performance tracking
- Root cause category: build error, test failure, lint issue, security

### 3. Check PR CI Status

```
# See CI check results on open PRs
github_pr_list({ state: "open", limit: 10 })
```

Report which PRs have passing vs failing checks.

### 4. Quick Security Audit

```
# Dependency vulnerability check
Bash({ command: "cd <project-root> && pnpm audit --audit-level=moderate 2>&1" })

# Scan for hardcoded secrets
Grep({
  pattern: "(password|secret|api_key|apikey|token|private_key|access_key)\\s*[:=]\\s*['\"][^'\"]{8,}['\"]",
  glob: "**/*.{ts,js,json,yml,yaml}",
  output_mode: "content"
})

# Check .env files are gitignored
Bash({ command: "cd <project-root> && git ls-files '*.env' '*.env.*' '.env.local'" })

# Check Dependabot is configured
Glob({ pattern: ".github/dependabot.yml" })

# Check security workflow exists
Glob({ pattern: ".github/workflows/*security*" })
Glob({ pattern: ".github/workflows/*codeql*" })
```

### 5. Analyze Failures

Common failure categories and suggested fixes:

| Category | Symptoms | Fix |
|----------|----------|-----|
| Build error | `tsc` errors, missing modules | Run `pnpm build` locally, fix type errors |
| Test failure | Assertion errors, timeouts | Run failing test locally, fix or mock |
| Lint issue | ESLint errors | Run `pnpm lint`, fix violations |
| Security | `pnpm audit` failures | Update vulnerable deps, run `pnpm update` |
| Install fail | Lockfile mismatch | Run `pnpm install`, commit updated lockfile |
| Action error | Version not found, permission denied | Pin action version, check permissions |

### 6. Monitor GitHub Remote

Keep checking remote status until clean — no auth errors, no rejected pushes, no connection issues.

```
# Check remote connectivity
Bash({ command: "cd <project-root> && git remote -v" })
Bash({ command: "cd <project-root> && git ls-remote origin 2>&1" })

# Check authentication
Bash({ command: "cd <project-root> && gh auth status 2>&1" })

# Try dry-run push to detect rejection issues
Bash({ command: "cd <project-root> && git push --dry-run origin <branch> 2>&1" })

# Check branch tracking and ahead/behind
Bash({ command: "cd <project-root> && git status -sb" })

# If behind remote, pull first
Bash({ command: "cd <project-root> && git pull --rebase origin <branch> 2>&1" })

# If ahead, push
Bash({ command: "cd <project-root> && git push origin <branch> 2>&1" })

# Verify remote is in sync
Bash({ command: "cd <project-root> && git status -sb" })
```

**Repeat until all checks pass:**
- `git ls-remote` — no connection errors
- `gh auth status` — authenticated
- `git status -sb` — branch in sync (no ahead/behind)
- `git push --dry-run` — no rejection

Common errors and fixes:

| Error | Fix |
|-------|-----|
| `fatal: not found` | Check remote URL, repo may not exist or is private |
| `Permission denied` | Run `gh auth login`, check SSH key or token |
| `rejected (fetch first)` | Pull with `git pull --rebase`, resolve conflicts, retry |
| `rejected (non-fast-forward)` | Rebase or merge remote changes first |
| `SSL certificate problem` | Check corporate proxy or run `gh auth setup-git` |
| `fatal: could not read Username` | Run `gh auth login` or configure credential helper |
| `Connection timed out` | Network issue, retry after checking connectivity |

### 7. Optionally Re-trigger

```
# Only after user confirms fix is pushed
cicd_pipeline_trigger({ workflow: "ci", ref: "main" })
```

### 8. Store and Report

```
# Store results for future reference
memory_store({
  workflow_id: "<id>",
  task_id: "<task_id>",
  memory_type: "artifact",
  summary: "CI check: main passing, 1 PR failing (test timeout)",
  content: "Run #124 passed in 3m12s. PR #44 failing due to test timeout in packages/store.",
  created_by_agent: "masday-cicd-check",
  importance_score: 0.5,
  tags: ["ci-cd", "status-check"]
})
```

```
══════════════════════════════════════════
   CI/CD Status Check
══════════════════════════════════════════

Latest: #124 (main) -- PASSING (3m 12s)
Previous: #123 (main) -- FAILED (lint error, fixed in #124)

PRs:
  #45 "Fix memory leak" -- checks PASSING
  #44 "Add tests" -- checks FAILING (test timeout)

Security:
  Audit: clean (0 moderate, 0 high)
  Secrets scan: clean
  Dependabot: configured / not configured
  Security workflow: found / missing

Workflows:
  ci.yml        -- PASSING
  docker_yml    -- not triggered
  security.yml  -- PASSING / not found

══════════════════════════════════════════
   ACTIONS NEEDED
══════════════════════════════════════════

{specific fixes for any failures or security issues}
```

## Never

- Never auto-trigger pipelines without user confirmation
- Never skip the failure analysis step when builds are failing
- Never report CI as green if any required checks are failing
- Never skip security audit when checking pipeline health
- Never ignore `pnpm audit` warnings — at minimum report them
- Never report secrets scan as clean without actually running it

## Mandatory Review Pipeline

When this skill completes work on a workflow task, it MUST follow this pipeline:

`
STEP 1: Save progress to PostgreSQL
  workflow_saveProgress({
    workflow_id: "<workflowId>",
    task_id: "<taskId>",
    agent_name: "<current-agent>",
    progress_note: "<summary of work done>",
    evidence: ["<files modified>", "<tests run>"]
  })

STEP 2: Submit for review
  review_submit({
    workflow_id: "<workflowId>",
    task_id: "<taskId>",
    reviewer_agent: "masday-reviewer",
    decision: "<APPROVED | REWORK_REQUIRED | BLOCKED>",
    notes: "<what was done, key decisions>",
    gaps: ["<any gaps found>"]
  })

STEP 3: If REWORK_REQUIRED — fix and loop
  - Fix the gaps identified in the review
  - Re-save progress (workflow_saveProgress)
  - Re-submit review (review_submit)
  - Max 2 rework attempts, then STOP

STEP 4: If APPROVED — validate completion
  policy_validate_completion({
    workflow_id: "<workflowId>",
    task_id: "<taskId>"
  })

STEP 5: Complete task
  workflow_completeTask({ workflow_id: "<workflowId>", task_id: "<taskId>" })

STEP 6: Sync local state
  local_sync({ cwd: process.cwd(), workflow_id: "<workflowId>" })
`

### Never
- Never call workflow_completeTask without review_submit (APPROVED)
- Never skip policy_validate_completion before completion
- Never skip local_sync after completing a task
- Never claim done without saving progress to PostgreSQL
