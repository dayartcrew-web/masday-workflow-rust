---
name: masday-deploy-check
description: >
  Pre-deployment validation suite. Runs dependency install, build, tests, lint, type checking,
  Docker build verification, and CI/CD pipeline checks. Use before pushing changes or when
  the user says "pre-deploy check", "ready to deploy", "deployment validation", or "check before push".
allowed-tools:
  - npm.install
  - npm.run
  - tests.run
  - git.status
  - git.diff
  - git.commit
  - docker.build
  - docker.ps
  - cicd.pipeline_status
  - cicd.pipeline_trigger
  - cicd.runs_view
  - github.pr_list
  - github.pr_create
  - github.issue_list
---

# Masday Deploy Check

Pre-flight checks before deployment.

## Steps

1. **Install dependencies**
   - Call `npm.install` if pnpm-lock.yaml has changed
   - Verify no vulnerability warnings in output

2. **Build**
   - Call `npm.run` with script `build`
   - Must pass with zero errors and zero warnings
   - If build fails, report the error and stop

3. **Run tests**
   - Call `tests.run` to execute the full test suite
   - All tests must pass (1017+ tests across 82+ files)
   - Report any failures with file names and error messages

4. **Check git state**
   - Call `git.status` to see branch, staged, and unstaged changes
   - Call `git.diff` to review all changes before committing
   - Flag any: uncommitted changes, hardcoded values, debug statements

5. **Type checking**
   - Verify TypeScript compiles without errors
   - No `any` types, no implicit any, strict mode enforced

6. **Lint**
   - Verify no critical lint issues
   - Check for: unused imports, missing return types, console.log statements

7. **Docker verification** (if Dockerfile exists)
   - Call `docker.build` with appropriate tag to verify image builds
   - Call `docker.ps` to check current running containers

8. **CI/CD pipeline**
   - Call `cicd.pipeline_status` to check current pipeline state
   - If pipeline is failing, call `cicd.runs_view` for error details
   - Optionally call `cicd.pipeline_trigger` to start a new run

9. **GitHub integration**
   - Call `github.pr_list` to check existing PRs
   - Call `github.issue_list` for related issues

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
    - Call `git.commit` to commit all changes
    - Call `cicd.pipeline_trigger` to start deployment
    - Call `github.pr_create` if PR workflow is used

## Never

- Never deploy if build or tests are failing
- Never skip the git diff review before committing
- Never commit .env files, credentials, or secrets
- Never force-deploy past failing CI/CD checks
