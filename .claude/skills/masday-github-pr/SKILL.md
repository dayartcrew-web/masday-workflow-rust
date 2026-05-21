---
name: masday-github-pr
description: >
  Create a GitHub PR from current changes. Reviews diff, stages files, commits with
  conventional messages, and creates a PR linked to related issues. Use when the user
  says "create PR", "make pull request", "open PR", or "push changes".
allowed-tools:
  - git_status
  - git_diff
  - git_commit
  - github_pr_create
  - github_pr_list
  - github_issue_list
  - memory_store
  - memory_recall_documents
---

# Masday GitHub PR

Create a GitHub pull request from current changes.

## Steps

1. **Review changes**
   - Call `git_status` to see all staged, unstaged, and untracked files
   - Call `git_diff` to review the full diff of all changes
   - Verify no sensitive files are included: .env, credentials, secrets

2. **Check existing PRs**
   - Call `github_pr_list` to check for existing PRs on this branch
   - If a PR already exists, report it and stop

3. **Find related issues**
   - Call `github_issue_list` to find issues related to the changes
   - Match by keywords from the commit messages and changed file paths

4. **Recall workflow context**
   - Call `memory_recall_documents` to load workflow decisions and artifacts
   - Use this context to write a comprehensive PR description

5. **Commit with conventional message**
   - Call `git_commit` with a message following the format:
     ```
     <type>: <description>

     <optional body with context>
     ```
   - Types: feat, fix, refactor, docs, test, chore, perf, ci
   - Stage only relevant files, excluding secrets and build artifacts

6. **Create the PR**
   - Call `github_pr_create` with:
     - `title`: matches commit message format
     - `body`: structured with Summary, Test Plan, Related Issues sections
     - `base`: main (default target branch)
     - `head`: current feature branch
   - Example body:
     ```
     ## Summary
     - Implements authentication middleware for the orchestrator
     - Adds token validation and role-based access control

     ## Test Plan
     - [ ] Unit tests for token validation (12 tests)
     - [ ] Integration tests for middleware chain
     - [ ] Manual verification with curl commands

     ## Related Issues
     Closes #42
     ```

7. **Store PR reference**
   - Call `memory_store` with `memory_type: "artifact"`:
     - PR number, URL, branch name, and change summary

## Never

- Never create a PR without reviewing the diff first
- Never include .env, credentials, or secrets in the commit
- Never create a duplicate PR without checking first
- Never use vague PR titles like "updates" or "changes"

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
