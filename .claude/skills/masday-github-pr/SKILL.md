---
name: masday-github-pr
description: >
  Create a GitHub PR from current changes. Reviews diff, stages files, commits with
  conventional messages, and creates a PR linked to related issues. Use when the user
  says "create PR", "make pull request", "open PR", or "push changes".
allowed-tools:
  - git.status
  - git.diff
  - git.commit
  - github.pr_create
  - github.pr_list
  - github.issue_list
  - memory.store
  - memory.recall_documents
---

# Masday GitHub PR

Create a GitHub pull request from current changes.

## Steps

1. **Review changes**
   - Call `git.status` to see all staged, unstaged, and untracked files
   - Call `git.diff` to review the full diff of all changes
   - Verify no sensitive files are included: .env, credentials, secrets

2. **Check existing PRs**
   - Call `github.pr_list` to check for existing PRs on this branch
   - If a PR already exists, report it and stop

3. **Find related issues**
   - Call `github.issue_list` to find issues related to the changes
   - Match by keywords from the commit messages and changed file paths

4. **Recall workflow context**
   - Call `memory.recall_documents` to load workflow decisions and artifacts
   - Use this context to write a comprehensive PR description

5. **Commit with conventional message**
   - Call `git.commit` with a message following the format:
     ```
     <type>: <description>

     <optional body with context>
     ```
   - Types: feat, fix, refactor, docs, test, chore, perf, ci
   - Stage only relevant files, excluding secrets and build artifacts

6. **Create the PR**
   - Call `github.pr_create` with:
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
   - Call `memory.store` with `memory_type: "artifact"`:
     - PR number, URL, branch name, and change summary

## Never

- Never create a PR without reviewing the diff first
- Never include .env, credentials, or secrets in the commit
- Never create a duplicate PR without checking first
- Never use vague PR titles like "updates" or "changes"
