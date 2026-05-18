# Pre-Commit Hook: Quality Gate
# Triggers before git commit operations

Before any git commit in this project:

1. **Check changes** — call `git.diff` to see what will be committed (**WARNING**: `git.diff` is a stub — use `Bash: git diff` for real results)
2. **Check status** — call `git.status` to verify branch and staging state (**WARNING**: `git.status` is a stub — use `Bash: git status` for real results)
3. Run `pnpm build` — must pass
4. Check TypeScript compilation — no errors
5. **Run tests** — call `tests.run` to verify nothing is broken (**WARNING**: `tests.run` is a stub that always returns exitCode 0 — use `Bash: pnpm test` for real results)
6. Verify no `.only` in test files
7. Check no `console.log` in production code (src/ only, not test files)
8. Ensure no `TODO` or `FIXME` without an associated issue number

If any check fails, report and ask before proceeding.
