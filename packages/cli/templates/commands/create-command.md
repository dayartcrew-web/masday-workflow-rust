---
name: masday-create-command
description: Create a new slash command for Masday Workflow
argument-hint: [command-name] [description of what it does]
disable-model-invocation: true
allowed-tools: filesystem.read filesystem.write filesystem.list
---

Create a new Masday slash command.

## Input
$ARGUMENTS — command name and description.

## Steps

1. **Detect project root** — run `git rev-parse --show-toplevel` to find the masday-workflow-reborn repo root. Use this as `$ROOT` in subsequent steps.
2. **Parse input** — extract name and purpose
3. **Check for conflicts** — list existing `~/.claude/commands/masday-*.md`
4. **Generate command** with:
   - Frontmatter: name, description, argument-hint, disable-model-invocation, allowed-tools, context
   - Instructions Claude follows when invoked
   - Use $ARGUMENTS for runtime input
5. **Save to 3 locations**:
   - `~/.claude/commands/masday-<name>.md` (global, with `masday-` prefix)
   - `$ROOT/.claude/commands/<name>.md` (project, no prefix)
   - `$ROOT/packages/cli/templates/commands/<name>.md` (template, no prefix)
6. **Report what was created**
