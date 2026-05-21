---
name: masday-cicd-ops
description: >
  Full CI/CD pipeline operations. Check status, trigger builds, inspect run results,
  debug failures, run security audits, and correlate with GitHub PRs. Use when
  managing CI/CD pipelines, debugging build failures, running security scans,
  or when the user says "pipeline status", "trigger build", "CI/CD ops",
  "check security", "audit deps", or "pipeline management".
allowed-tools:
  - cicd_pipeline_status
  - cicd_pipeline_trigger
  - cicd_runs_view
  - github_pr_list
  - github_issue_list
  - github_pr_create
  - git_status
  - git_diff
  - git_commit
  - Bash
  - Read
  - Glob
  - Grep
  - memory_store
  - memory_recall_recent
  - memory_search
  - workflow_saveProgress
---

# Masday CI/CD Ops

Manage CI/CD pipelines, inspect build results, run security audits, and debug failures.

## Steps

### 1. Check Pipeline Status

```
# Get recent pipeline runs for current branch
cicd_pipeline_status({ branch: "main", limit: 5 })

# Check all open PRs and their CI status
github_pr_list({ state: "open", limit: 10 })

# Check for specific workflow
cicd_pipeline_status({ branch: "feature/auth", limit: 3 })
```

Identify:
- Passing / failing / in-progress pipelines
- Which workflow (CI, Docker, Security, Release)
- Commit SHA and duration
- Branch or PR association

### 2. Inspect Failed Runs

```
# Get detailed step-by-step results for a failed run
cicd_runs_view({ runId: 12345 })

# Read the workflow file that failed
Read({ file_path: ".github/workflows/ci.yml" })

# Check what changed since last passing run
git_diff({ repoPath: ".", file: "packages/store/src/sqlite-backend.ts" })
```

### 3. Trigger Pipeline

```
# Trigger workflow on current branch
cicd_pipeline_trigger({ workflow: "ci", ref: "feature/auth" })

# Trigger with inputs
cicd_pipeline_trigger({
  workflow: "deploy",
  ref: "main",
  inputs: { environment: "staging" }
})

# Monitor the triggered run
cicd_pipeline_status({ branch: "feature/auth", limit: 1 })
```

**IMPORTANT**: Always confirm with user before triggering deployment pipelines.

### 4. Security Audit

```
# Run npm dependency audit
Bash({ command: "cd <project-root> && pnpm audit --audit-level=moderate" })

# Check for known vulnerabilities
Bash({ command: "cd <project-root> && pnpm audit --json" })

# Scan for hardcoded secrets in codebase
Grep({ pattern: "(password|secret|api_key|token|private_key)\\s*[:=]\\s*['\"][^'\"]+['\"]", glob: "**/*.{ts,js,json,yml,yaml,env}", output_mode: "content" })

# Check .gitignore covers sensitive files
Read({ file_path: ".gitignore" })

# Verify no .env files are tracked
Bash({ command: "cd <project-root> && git ls-files '*.env*' '*.env.*'" })

# Check Dependabot config exists
Glob({ pattern: ".github/dependabot.yml" })
```

### 5. Debug Build Failures

Common failure patterns and fixes:

| Failure Type | How to Diagnose | Fix |
|-------------|-----------------|-----|
| TypeScript error | Read the error line, check types | `tsc --noEmit` locally, fix type mismatch |
| pnpm install fail | Check lockfile sync | Run `pnpm install`, commit updated lockfile |
| Test timeout | Check which test hangs | Add `--testTimeout`, mock external deps |
| Action not found | Check action version | Pin to SHA or valid tag |
| Permission denied | Check `permissions:` in workflow | Add minimal required permissions |
| Docker build fail | Read Dockerfile, check base image | Fix Dockerfile, test locally with `docker build` |
| Secret missing | Check workflow `${{ secrets.* }}` | Add secret to repo settings |
| Path issue (Windows) | Check path separators | Use `path.join()` instead of string concat |

### 6. Recall and Store Context

```
# Recall prior pipeline context
memory_recall_recent({ limit: 5 })
memory_search({ query: "pipeline failure build", limit: 5 })

# Store pipeline results
memory_store({
  workflow_id: "<id>",
  task_id: "<task_id>",
  memory_type: "artifact",
  summary: "CI pipeline fixed: TypeScript strict mode errors in sqlite-backend",
  content: "Build was failing due to null safety issues. Fixed by adding explicit null checks.",
  created_by_agent: "masday-cicd-ops",
  importance_score: 0.7,
  tags: ["ci-cd", "pipeline", "build-fix"]
})
```

### 7. Report

```
══════════════════════════════════════════
   CI/CD Pipeline Report
══════════════════════════════════════════

Pipeline: {name}
Status: PASSING / FAILING
Run: #{id} (duration: {time})
Commit: {sha}
Branch: {branch}

Steps:
  ✅ build (30s)
  ✅ test (45s)
  ❌ lint (10s) — "Unexpected any in src/types.ts:42"
  ⏳ integration (running)

Security:
  Audit: {clean / N moderate, M high}
  Secrets scan: {clean / N findings}
  Dependabot: {configured / not configured}

PRs:
  #{pr} "{title}" — checks {passing/failing}

══════════════════════════════════════════
   ACTIONS
══════════════════════════════════════════

{specific next steps to fix failures}
```

## Never

- Never trigger deployment pipelines without user confirmation
- Never ignore failing pipeline steps
- Never trigger multiple pipelines simultaneously on the same branch
- Never store sensitive pipeline secrets in memory
- Never skip security audit when adding new dependencies
- Never commit workflow changes without YAML validation first
- Always call `workflow_saveProgress` after completing pipeline operations to persist results
- Never expose secrets in workflow files or memory
- Never run `pnpm audit` with `--audit-level=low` in CI (too noisy, use moderate+)

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
