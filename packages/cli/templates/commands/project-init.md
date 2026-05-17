---
name: masday-project-init
description: Initialize .masday/ data directory in current project. Creates research, context, plans, and notes folders and generates initial codebase analysis.
disable-model-invocation: true
allowed-tools: Bash filesystem.read filesystem.list filesystem.write
---

Initialize Masday Workflow data in the current project.

## Steps

1. **Create directory structure**:

   ```bash
    mkdir -p .masday/{research,context,plans,notes}
   ```

2. **Add .gitignore rules only if your project wants to ignore generated local artifacts**
   - Do not assume a `.masday/state/` runtime directory exists.

3. **Generate codebase analysis** → `.masday/research/codebase-analysis.md`:
   - Scan with `filesystem.list` (recursive)
   - Read key files: package.json, tsconfig, entry points, README
   - Write summary: structure, tech stack, entry points, patterns, test coverage

4. **Create project context** → `.masday/context/project-context.md`:
   - Project name and description
   - Key directories and their purpose
   - Dependencies and their roles
   - Architecture decisions
   - Current development status

5. **Create initial state** → `.masday/context/current-workflow.json`:

   ```json
   {
     "initializedAt": "<timestamp>",
     "lastAnalysis": "<timestamp>",
     "activeWorkflows": [],
     "completedWorkflows": 0
   }
   ```

6. **Report**:

   ```
   ✅ Masday Workflow initialized
   📁 .masday/ created

   Generated:
   - research/codebase-analysis.md (2.1KB — codebase summary)
   - context/project-context.md (1.8KB — project overview)
   - context/current-workflow.json (state tracker)

   Commands:
   → /masday-workflow-new "build feature X"  — Start a workflow
   → /masday-project-sync                    — Refresh analysis
   ```

## Rules

- Never overwrite existing research/ unless explicitly asked
- Append to notes/ never replace
- `.masday/` contains local artifacts; operational workflow state lives in SQLite
