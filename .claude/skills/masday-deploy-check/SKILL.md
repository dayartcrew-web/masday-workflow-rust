---
name: masday-deploy-check
description: >
  Pre-deployment validation suite. Runs dependency install, build, tests, lint, type checking,
  Docker build verification, and CI/CD pipeline checks. Use before pushing changes or when
  the user says "pre-deploy check", "ready to deploy", "deployment validation", or "check before push".
allowed-tools:
  - npm_install
  - npm_run
  - tests_run
  - git_status
  - git_diff
  - git_commit
  - docker_build
  - docker_ps
  - cicd_pipeline_status
  - cicd_pipeline_trigger
  - cicd_runs_view
  - github_pr_list
  - github_pr_create
  - github_issue_list
---

# Masday Deploy Check

Pre-flight checks before deployment.

## Steps

1. **Install dependencies**
   - Call `npm_install` if pnpm-lock.yaml has changed
   - Verify no vulnerability warnings in output

2. **Build**
   - Call `npm_run` with script `build`
   - Must pass with zero errors and zero warnings
   - If build fails, report the error and stop

3. **Run tests**
   - Call `tests_run` to execute the full test suite
   - All tests must pass (1017+ tests across 82+ files)
   - Report any failures with file names and error messages

4. **Check git state**
   - Call `git_status` to see branch, staged, and unstaged changes
   - Call `git_diff` to review all changes before committing
   - Flag any: uncommitted changes, hardcoded values, debug statements

5. **Type checking**
   - Call `npm_run` with script `typecheck` (or `tsc --noEmit` via Bash)
   - Verify TypeScript compiles without errors
   - No `any` types, no implicit any, strict mode enforced

6. **Lint**
   - Call `npm_run` with script `lint` (or `eslint` via Bash)
   - Verify no critical lint issues
   - Check for: unused imports, missing return types, console.log statements

7. **Docker verification** (if Dockerfile exists)
   - Call `docker_build` with appropriate tag to verify image builds
   - Call `docker_ps` to check current running containers

8. **CI/CD pipeline**
   - Call `cicd_pipeline_status` to check current pipeline state
   - If pipeline is failing, call `cicd_runs_view` for error details
   - Optionally call `cicd_pipeline_trigger` to start a new run

9. **GitHub integration**
   - Call `github_pr_list` to check existing PRs
   - Call `github_issue_list` for related issues

10. **Report**
    ```
    === Deploy Check ===
    OK   Dependencies: installed
    OK   Build: clean (2.3s)
    OK   Tests: 47/47 passing
    WARN Git: 3 uncommitted files
    OK   Types: no errors
    OK   Lint: clean
    OK   Docker: image built successfully
    OK   CI/CD: pipeline passing

    Status: READY TO DEPLOY (fix WARN items first)
    ```

11. **Deploy** (if all checks pass and user confirms)
    - Call `git_commit` to commit all changes
    - Call `cicd_pipeline_trigger` to start deployment
    - Call `github_pr_create` if PR workflow is used

## Never

- Never deploy if build or tests are failing
- Never skip the git diff review before committing
- Never commit .env files, credentials, or secrets
- Never force-deploy past failing CI/CD checks

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
