---
name: masday-workflow-plan
description: Plan a workflow using cached .masday/ context. Saves plan locally before execution.
allowed-tools: workflow.create workflow.addTask workflow.get filesystem.read filesystem.list filesystem.write
---

# Workflow Plan

Plan tasks using cached project context for accuracy and token efficiency.

## Steps

1. **Load cached context**:
   - `.masday/research/codebase-analysis.md` — structure, patterns
   - `.masday/context/project-context.md` — current state
   - Previous plans in `.masday/plans/` for reference

2. **Analyze the request** against cached knowledge

3. **If cache is stale** (older than 1 day or major changes):
   - Quick scan with `filesystem.list`
   - Update `.masday/research/codebase-analysis.md`

4. **Generate task plan** — assign agents, skills, dependencies

5. **Save plan** → `.masday/plans/<date>-<name>.md`:
   ```markdown
   # Plan: <name>
   Created: <date>
   Status: DRAFT
   
   ## Tasks
   #1 [backend] Task name → skill, input, deps
   #2 [frontend] Task name → skill, input, deps
   ...
   
   ## Critical Path
   #1 → #2 → #4
   Parallel: #2 and #3
   ```

6. **Add tasks to workflow** using `workflow.addTask`

7. **Present plan** and ask: "Execute? `/masday-workflow-run <id>`"

## Token Savings
- Cache hit = skip codebase scan (~2-4K tokens saved)
- Research summaries = 1 file instead of 20+ file reads
