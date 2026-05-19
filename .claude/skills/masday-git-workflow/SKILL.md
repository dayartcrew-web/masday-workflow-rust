---
name: masday-git-workflow
description: >
  Git version control operations for staging, diffing, committing, and tracking changes.
  Integrates with memory for workflow tracking. Use when the user says "commit changes",
  "git status", "review diff", "stage files", or "git operations".
allowed-tools:
  - git.status
  - git.diff
  - git.commit
  - filesystem.read
  - filesystem.list
  - memory.store
---

# Masday Git Workflow

Git operations integrated with Masday workflow tracking.

## Steps

1. **Check current state**
   - Call `git.status` to see: current branch, staged files, unstaged files, untracked files
   - Identify the working context: feature branch, main, detached HEAD

2. **Review changes before committing**
   - Call `git.diff` to see all staged and unstaged changes
   - For each changed file, verify:
     - No hardcoded secrets or credentials
     - No debug statements (console.log, debugger)
     - No unrelated changes mixed in
   - Call `filesystem.read` on any suspicious files for full context

3. **Stage relevant files**
   - Select only files related to the current task
   - Exclude: .env files, node_modules, build artifacts, IDE settings
   - Verify staging with `git.status`

4. **Commit with conventional format**
   - Call `git.commit` with message format:
     ```
     <type>: <description>

     <optional body explaining why>
     ```
   - Types: feat, fix, refactor, docs, test, chore, perf, ci
   - Description: imperative mood, under 72 characters
   - Body: explain the "why" not the "what"

5. **Verify commit**
   - Call `git.status` to confirm clean working tree
   - Call `git.diff` to verify no leftover unstaged changes

6. **Store for workflow tracking**
   - Call `memory.store` with `memory_type: "artifact"`:
     - Commit hash (abbreviated)
     - Commit message
     - Files changed count
     - Branch name

7. **Report**
   ```
   Committed: abc1234 on feature/add-auth
   Message: feat: add authentication middleware

   Files: 5 changed
   - packages/core/src/auth.ts (new)
   - packages/orchestrator/src/middleware.ts (modified)
   - packages/core/src/index.ts (modified)
   - tests/auth.test.ts (new)
   - package.json (modified)
   ```

## Never

- Never commit .env files, credentials, or API keys
- Never skip reviewing the diff with `git.diff` before committing
- Never use vague commit messages like "fix stuff" or "updates"
- Never commit unrelated changes in the same commit
- Never commit node_modules or build artifacts

## Mandatory Review Pipeline

When this skill completes work on a workflow task, it MUST follow this pipeline:

`
STEP 1: Save progress to PostgreSQL
  workflow.saveProgress({
    workflow_id: "<workflowId>",
    task_id: "<taskId>",
    agent_name: "<current-agent>",
    progress_note: "<summary of work done>",
    evidence: ["<files modified>", "<tests run>"]
  })

STEP 2: Submit for review
  review.submit({
    workflow_id: "<workflowId>",
    task_id: "<taskId>",
    reviewer_agent: "masday-reviewer",
    decision: "<APPROVED | REWORK_REQUIRED | BLOCKED>",
    notes: "<what was done, key decisions>",
    gaps: ["<any gaps found>"]
  })

STEP 3: If REWORK_REQUIRED — fix and loop
  - Fix the gaps identified in the review
  - Re-save progress (workflow.saveProgress)
  - Re-submit review (review.submit)
  - Max 2 rework attempts, then STOP

STEP 4: If APPROVED — validate completion
  policy.validate_completion({
    workflow_id: "<workflowId>",
    task_id: "<taskId>"
  })

STEP 5: Complete task
  workflow.completeTask({ workflow_id: "<workflowId>", task_id: "<taskId>" })

STEP 6: Sync local state
  local.sync({ cwd: process.cwd(), workflow_id: "<workflowId>" })
`

### Never
- Never call workflow.completeTask without review.submit (APPROVED)
- Never skip policy.validate_completion before completion
- Never skip local.sync after completing a task
- Never claim done without saving progress to PostgreSQL
