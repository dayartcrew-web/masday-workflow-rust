---
name: masday-git-workflow
description: >
  Git version control operations for staging, diffing, committing, and tracking changes.
  Integrates with memory for workflow tracking. Use when the user says "commit changes",
  "git status", "review diff", "stage files", or "git operations".
allowed-tools:
  - git_status
  - git_diff
  - git_commit
  - filesystem_read
  - filesystem_list
  - memory_store
---

# Masday Git Workflow

Git operations integrated with Masday workflow tracking.

## Steps

This skill enforces **mandatory step completion**. Each step must be completed before proceeding. Do not skip steps.


1. **Check current state**
   - Call `git_status` to see: current branch, staged files, unstaged files, untracked files
   - Identify the working context: feature branch, main, detached HEAD

2. **Review changes before committing**
   - Call `git_diff` to see all staged and unstaged changes
   - For each changed file, verify:
     - No hardcoded secrets or credentials
     - No debug statements (console.log, debugger)
     - No unrelated changes mixed in
   - Call `filesystem_read` on any suspicious files for full context

3. **Stage relevant files**
   - Select only files related to the current task
   - Exclude: .env files, node_modules, build artifacts, IDE settings
   - Verify staging with `git_status`

4. **Commit with conventional format**
   - Call `git_commit` with message format:
     ```
     <type>: <description>

     <optional body explaining why>
     ```
   - Types: feat, fix, refactor, docs, test, chore, perf, ci
   - Description: imperative mood, under 72 characters
   - Body: explain the "why" not the "what"


**GATE**: Verify steps 1-4 are complete before proceeding.

5. **Verify commit**
   - Call `git_status` to confirm clean working tree
   - Call `git_diff` to verify no leftover unstaged changes

6. **Store for workflow tracking**
   - Call `memory_store` with `memory_type: "artifact"`:
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
- Never skip any step — complete each step before proceeding
- Never bypass a GATE marker without validating prior steps
- Never claim completion without executing all steps in order

- Never commit .env files, credentials, or API keys
- Never skip reviewing the diff with `git_diff` before committing
- Never use vague commit messages like "fix stuff" or "updates"
- Never commit unrelated changes in the same commit
- Never commit node_modules or build artifacts

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
