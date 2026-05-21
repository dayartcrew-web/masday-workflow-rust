# Pre-Commit Hook: Quality Gate
# Triggers before git commit operations

Before any git commit in this project:

1. **Check changes** — call `git_diff` to see what will be committed (runs real `git diff` via execSync)
2. **Check status** — call `git_status` to verify branch and staging state (runs real `git status` via execSync)
3. Run `pnpm build` — must pass
4. Check TypeScript compilation — no errors
5. **Run tests** — call `tests_run` to verify nothing is broken (runs real `pnpm test` via execSync)
6. Verify no `.only` in test files
7. Check no `console.log` in production code (src/ only, not test files)
8. Ensure no `TODO` or `FIXME` without an associated issue number

If any check fails, report and ask before proceeding.
