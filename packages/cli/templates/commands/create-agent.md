---
name: masday-create-agent
description: Create a new Masday agent definition from a description
argument-hint: [description of agent specialization]
disable-model-invocation: true
allowed-tools: filesystem.read filesystem.write filesystem.list
---

Create a new Masday Workflow agent.

## Input
$ARGUMENTS — describe the agent's specialization and role.

## Steps

1. **Detect project root** — run `git rev-parse --show-toplevel` to find the masday-workflow-reborn repo root. Use this as `$ROOT` in subsequent steps.
2. **Parse the description** — understand specialization, routing, tools
3. **Check for conflicts** — list existing `~/.claude/agents/masday-*.md`
4. **Generate agent definition**:
   - Specialization summary
   - Capabilities list
   - Preferred skills
   - Task execution style (numbered workflow)
   - Constraints
5. **Save to 3 locations**:
   - `~/.claude/agents/masday-<name>.md` (global, with `masday-` prefix)
   - `$ROOT/.claude/agents/<name>.md` (project, no prefix)
   - `$ROOT/packages/cli/templates/agents/<name>.md` (template, no prefix)
6. **Report what was created**
