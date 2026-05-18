---
name: masday-skill-builder
description: >
  Master skill builder that creates skills, agents, commands, and MCP server packages.
  Handles registration, file generation, and scaffolding. Use when the user says
  "build skill", "create agent", "new command", "scaffold feature", or "add capability".
allowed-tools:
  - capability.create_agent
  - capability.create_skill
  - capability.list_agents
  - capability.list_skills
  - capability.list_templates
  - capability.scaffold_feature
  - capability.scaffold_mcp_server
  - filesystem.read
  - filesystem.write
  - filesystem.list
  - filesystem.stat
  - filesystem.delete
---

# Masday Skill Builder

Create new skills, agents, commands, and MCP packages for Masday Workflow.

## Pre-flight

1. **Detect project root** -- find the masday-workflow-reborn repo root
2. **List existing** -- call `capability.list_agents` and `capability.list_skills` to avoid duplicates
3. **Check templates** -- call `capability.list_templates` for available scaffolds

## Skill Builder

### Steps

1. **Understand the requirement**
   - What does the skill do? What triggers it?
   - What MCP tools does it need? (choose from the 70 available)
   - Does it need supporting files (templates, scripts, references)?

2. **Generate SKILL.md** with proper structure:
   ```yaml
   ---
   name: <skill-name>
   description: >
     WHAT it does. WHEN to use it. Key capabilities.
     Use when the user says "trigger phrase" or "trigger phrase".
   allowed-tools:
     - tool.name
   ---
   ```

3. **Save to project location**
   - `$ROOT/.claude/skills/<name>/SKILL.md`

4. **Register** -- call `capability.create_skill` with name, description, trigger, and steps

## Agent Builder

### Steps

1. **Define the agent**
   - Name, role, and specialization
   - Preferred skills and tools
   - Task routing rules and constraints

2. **Register** -- call `capability.create_agent` with name, role, description, and instructions

3. **Save to project**
   - `$ROOT/.claude/agents/<name>.md`

## Scaffold Builder

### Steps

1. **Full feature** -- call `capability.scaffold_feature` for agent + skill + command + MCP tool stub
2. **MCP server** -- call `capability.scaffold_mcp_server` for a new server package
3. **Review generated code** and adjust as needed

## Validation Checklist

- [ ] No duplicate names with existing skills/agents
- [ ] Description under 250 characters
- [ ] `allowed-tools` lists only needed tools (actual MCP tool names)
- [ ] Name follows kebab-case convention
- [ ] Registered via capability API

## Never

- Never overwrite existing skills without user confirmation
- Never use fake tool names in `allowed-tools` -- only actual MCP tool names
- Never skip the duplicate check before creating
- Never create skills without a clear trigger condition
