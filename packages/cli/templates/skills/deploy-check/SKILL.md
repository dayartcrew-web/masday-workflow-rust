---
name: masday-deploy-check
description: Pre-deployment validation using cached .masday/ context
allowed-tools: Bash
disable-model-invocation: true
---

# Deploy Check

Pre-flight checks with project context from `.masday/`.

## Steps

1. **Read context** from `.masday/context/project-context.md`
2. **Build**: `pnpm build` or project-equivalent
3. **Tests**: `pnpm test` or project-equivalent
4. **Git**: check uncommitted changes
5. **TypeScript**: no type errors
6. **Lint**: no critical issues

## Report
```
✅ Build: clean (2.3s)
✅ Tests: 47/47
⚠️ Git: 3 uncommitted files
✅ Types: clean
✅ Lint: clean

→ Fix uncommitted files before deploy
```

Save result → `.masday/notes/<date>-deploy-check.md`
