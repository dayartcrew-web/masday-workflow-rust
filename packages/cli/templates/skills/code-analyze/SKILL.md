---
name: masday-code-analyze
description: Analyze codebase and cache results in .masday/research/. Use when starting new work, debugging, or before planning a workflow.
allowed-tools: Bash filesystem.read filesystem.list filesystem.write
---

# Code Analyze

Analyze codebase and save results to `.masday/research/` for token efficiency.

## Steps

1. **Check `.masday/`** — if missing, create structure:
   ```bash
   mkdir -p .masday/{research,context,state/{workflows,tasks},plans,notes}
   ```

2. **Scan structure** using `filesystem.list`

3. **Read key files**: package.json, tsconfig, config files, entry points, README

4. **Analyze**:
   - Package structure and exports
   - Dependencies between modules
   - Tech stack and frameworks
   - Test coverage areas
   - Patterns and conventions

5. **Write analysis** → `.masday/research/codebase-analysis.md`:
   ```markdown
   # Codebase Analysis
   > Updated: <date>
   
   ## Structure
   <tree overview>
   
   ## Tech Stack
   - Runtime, framework, tools
   
   ## Entry Points
   - Main files and their roles
   
   ## Key Patterns
   - Architecture patterns used
   
   ## Dependencies Map
   - Internal: package relationships
   - External: key libraries
   
   ## Test Coverage
   - What's tested, what's not
   
   ## Recent Changes
   - What changed since last analysis
   ```

6. **Update context** → `.masday/context/project-context.md` (if exists, append changes)

## Benefits
- Next session reads `.masday/research/` instead of scanning entire codebase
- Saves ~60-80% tokens on repeated analysis
- Tracks project evolution over time
