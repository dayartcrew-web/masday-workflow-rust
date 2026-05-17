# Pre-Commit Hook: Quality Gate
# Triggers before git commit operations

Before any git commit in this project:

1. Run `pnpm build` — must pass
2. Check TypeScript compilation — no errors
3. Verify no `.only` in test files
4. Check no `console.log` in production code (src/ only, not test files)
5. Ensure no `TODO` or `FIXME` without an associated issue number

If any check fails, report and ask before proceeding.
