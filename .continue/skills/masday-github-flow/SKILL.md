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
