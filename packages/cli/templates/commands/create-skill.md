---
name: masday-create-skill
description: Create a new Masday skill from a description — generates SKILL.md and saves to all locations
argument-hint: [description of what the skill should do]
disable-model-invocation: true
allowed-tools: filesystem.read filesystem.write filesystem.list
---

Create a new Masday Workflow skill.

## Input
$ARGUMENTS — describe what the skill should do.

## Steps

1. **Detect project root** — run `git rev-parse --show-toplevel` to find the masday-workflow-reborn repo root. Use this as `$ROOT` in subsequent steps.
2. **Parse the description** — understand purpose, triggers, needed tools
3. **Check for name conflicts** — list existing `~/.claude/skills/masday-*/SKILL.md`
4. **Generate SKILL.md** following the masday-workflow convention:
   - Frontmatter: name, description, disable-model-invocation, allowed-tools, context
   - Markdown: clear steps, rules, output format
5. **Save to 3 locations**:
   - `~/.claude/skills/masday-<name>/SKILL.md` (global)
   - `$ROOT/.claude/skills/<name>/SKILL.md` (project)
   - `$ROOT/packages/cli/templates/skills/<name>/SKILL.md` (template)
6. **Report what was created**

## Rules
- name: kebab-case, max 64 chars
- description: under 250 chars, front-load key use case
- Only list needed tools in allowed-tools
- Use `context: fork` for heavy/slow operations
