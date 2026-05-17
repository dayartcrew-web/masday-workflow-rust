---
name: masday-skill-builder
description: Create new skills and agents for Masday Workflow. Use when you need to add custom skills or agent definitions to extend the platform.
disable-model-invocation: false
allowed-tools: filesystem.read filesystem.write filesystem.list
---

# Masday Skill Builder

Create new skills, agents, and commands for Masday Workflow interactively.

## Pre-flight

1. **Detect project root** — run `git rev-parse --show-toplevel` to find the masday-workflow-reborn repo root. Use this as `$ROOT` in subsequent steps.

## Skill Builder

### Input
The user describes what the new skill should do (natural language).

### Steps

1. **Understand the requirement**:
   - What does this skill do?
   - What triggers it? (automatic or manual)
   - What MCP tools does it need?
   - Does it need supporting files?

2. **Generate SKILL.md** with proper structure:
   ```yaml
   ---
   name: <skill-name>
   description: <when to use, what it does — under 250 chars>
   disable-model-invocation: <true|false>
   allowed-tools: <space-separated list>
   context: <fork|inline>
   ---
   
   # <Skill Name>
   
   <Detailed instructions for Claude to follow>
   
   ## Steps
   <numbered steps>
   
   ## Rules
   <constraints>
   
   ## Output Format
   <expected output>
   ```

3. **Save locations** (all three for full coverage):
   - `~/.claude/skills/masday-<name>/SKILL.md` — global
   - `$ROOT/.claude/skills/<name>/SKILL.md` — project
   - `$ROOT/packages/cli/templates/skills/<name>/SKILL.md` — installer template

4. **If skill needs supporting files**, create them in the skill directory:
   - `templates/` — output templates
   - `scripts/` — executable scripts
   - `references/` — reference docs

## Agent Builder

### Input
The user describes what the new agent should specialize in.

### Steps

1. **Define the agent**:
   - Name and specialization
   - Preferred skills and tools
   - Task routing rules
   - Constraints and conventions

2. **Generate agent definition**:
   ```markdown
   # <Agent Name> Agent
   
   <Specialization summary>
   
   ## Capabilities
   - <what it can do>
   
   ## Preferred Skills
   - `<skill-name>` — <purpose>
   
   ## Task Execution Style
   <numbered workflow>
   
   ## Constraints
   <rules>
   ```

3. **Save to all locations**:
   - `~/.claude/agents/masday-<name>.md`
   - `$ROOT/.claude/agents/<name>.md`
   - `$ROOT/packages/cli/templates/agents/<name>.md`

## Command Builder

### Input
The user describes the slash command and its behavior.

### Steps

1. **Define the command**:
   - Name (kebab-case)
   - Argument hints
   - Auto/manual invocation
   - Required tools
   - Context mode (fork for heavy tasks)

2. **Generate command**:
   ```yaml
   ---
   name: <command-name>
   description: <what it does>
   argument-hint: [required-args]
   disable-model-invocation: <true|false>
   allowed-tools: <tools>
   context: <fork|inline>
   ---
   
   <Instructions Claude follows when command is invoked>
   ```

3. **Save to all locations**:
   - `~/.claude/commands/masday-<name>.md`
   - `$ROOT/.claude/commands/<name>.md`
   - `$ROOT/packages/cli/templates/commands/<name>.md`

## MCP Skill Builder

For creating actual MCP-executable skills (TypeScript, not Claude prompt skills):

1. **Generate TypeScript skill file** in the appropriate package:
   - `packages/skills/src/<name>.ts` — filesystem-type skills
   - `packages/code-skills/src/<name>.ts` — code-related skills

2. **Skill template**:
   ```typescript
   import { z } from 'zod';
   import { Skill } from '@masday-workflow-reborn/core';
   
   export const <name>Skills: Skill[] = [
     {
       name: '<category>.<action>',
       description: '<what it does>',
       inputSchema: z.object({
         // params with zod
       }).shape,
       outputSchema: z.object({}).shape,
       execute: async (input) => {
         // implementation
       },
     },
   ];
   ```

3. **Register in package index** — update `index.ts` exports
4. **Register in MCP server** — update `apps/agent-runner/src/mcp.ts`
5. **Write test** — `<name>.test.ts` alongside source
6. **Build and verify**: `pnpm build`

## Validation Checklist

After creating any new component:

- [ ] File saved to all 3 locations (global, project, template)
- [ ] Name follows kebab-case convention
- [ ] Description under 250 characters
- [ ] allowed-tools lists only needed tools
- [ ] No duplicate names with existing skills/agents/commands
- [ ] Tested in Claude Code session
- [ ] `pnpm build` passes (for MCP skills)

## Report

After creation:
```
✅ Created: <type> "<name>"
📁 Saved to:
   ~/.claude/<type>s/masday-<name>.md
   $ROOT/.claude/<type>s/<name>.md
   $ROOT/packages/cli/templates/<type>s/<name>.md

Usage:
→ /masday-<name> [args]
```
