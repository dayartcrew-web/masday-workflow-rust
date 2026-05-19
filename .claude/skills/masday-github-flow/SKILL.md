---
name: masday-github-flow
description: >
  GitHub integration for PR and issue management. Handles the full flow: review changes,
  commit, check existing PRs, link issues, and create pull requests. Use when the user says
  "create PR", "make pull request", "open PR", "push and PR", or "GitHub flow".
allowed-tools:
  - github.pr_create
  - github.pr_list
  - github.issue_list
  - git.status
  - git.diff
  - git.commit
  - memory.store
  - memory.recall_documents
---

# Masday GitHub Flow

Manage GitHub PRs and issues as part of workflow execution.

## Steps

1. **Review current changes**
   - Call `git.status` to see branch, staged, and unstaged files
   - Call `git.diff` to review the exact changes
   - Verify no secrets, .env files, or build artifacts are included

2. **Recall workflow context**
   - Call `memory.recall_documents` to load the workflow context
   - Extract key decisions, task descriptions, and acceptance criteria for the PR body

3. **Check existing PRs**
   - Call `github.pr_list` to see if a PR already exists for this branch
   - If a PR exists, report it and ask whether to update or create a new one

4. **Find related issues**
   - Call `github.issue_list` with relevant labels
   - Match issues to the current changes for linking in the PR body

5. **Commit changes**
   - Call `git.commit` with a conventional commit message:
     - Format: `<type>: <description>`
     - Types: feat, fix, refactor, docs, test, chore, perf, ci
   - Stage only relevant files, not all changes

6. **Create pull request**
   - Call `github.pr_create` with:
     - `title`: conventional commit format (e.g., "feat: add caching layer")
     - `body`: include summary, test plan, and related issues
     - `base`: target branch (default: main)
     - `head`: current branch
   - PR body template:
     ```
     ## Summary
     - Key change 1
     - Key change 2

     ## Test Plan
     - [ ] Unit tests pass
     - [ ] Integration tests pass
     - [ ] Manual verification

     ## Related Issues
     Closes #<number>
     ```

7. **Store reference**
   - Call `memory.store` with `memory_type: "artifact"` containing:
     - PR number and URL
     - Branch name
     - Summary of changes

## Never

- Never create a PR with uncommitted changes
- Never include secrets or .env files in the commit
- Never skip checking for duplicate PRs
- Never use a generic PR title -- always follow conventional commit format

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
