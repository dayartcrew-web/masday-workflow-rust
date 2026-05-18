---
name: masday-create-skill
description: >
  Create a new Masday skill (SKILL.md) from a description. Designs trigger conditions,
  allowed tools, step-by-step workflow, and constraints. Registers and saves to all locations.
  Use when the user says "create skill", "new skill", "add workflow skill", or "skill definition".
allowed-tools:
  - capability.create_skill
  - capability.list_skills
  - capability.list_templates
  - capability.scaffold_feature
  - filesystem.write
  - filesystem.list
---

# Masday Create Skill

Create a new Masday Workflow skill definition.

## Steps

1. **Check existing skills**
   - Call `capability.list_skills` to see all registered skills
   - Verify the proposed skill name does not conflict with existing ones

2. **Check templates**
   - Call `capability.list_templates` for available patterns
   - Select a template if one matches the skill type

3. **Design the skill based on user description**
   - Name: kebab-case (e.g., `masday-deploy-check`, `masday-security-scan`)
   - Description: WHAT it does + WHEN to use it + trigger phrases
   - Allowed tools: only actual MCP tool names from the 70 available
   - Steps: detailed numbered workflow with tool call examples
   - Never section: constraints and prohibitions

4. **Generate SKILL.md** with proper YAML frontmatter:
   ```yaml
   ---
   name: <skill-name>
   description: >
     WHAT the skill does. WHEN to trigger it.
     Use when the user says "trigger" or "trigger".
   allowed-tools:
     - actual.mcp.tool.name
   ---
   ```

5. **Register the skill**
   - Call `capability.create_skill` with:
     - `name`: skill name
     - `description`: what it does
     - `trigger`: when to activate
     - `steps`: array of step descriptions

6. **Save to project location**
   - Call `filesystem.write` to save:
     - `$ROOT/.claude/skills/<name>/SKILL.md`

7. **Or use scaffold for full feature**
   - Call `capability.scaffold_feature` if the skill needs an agent and command

8. **Report**
   ```
   Skill created: "<name>"
   Tools: <list of allowed tools>
   Trigger: <trigger condition>
   Registered via capability API
   Saved to: .claude/skills/<name>/SKILL.md
   ```

## Never

- Never use fake MCP tool names -- only use actual tool names from the project
- Never create a skill without a clear trigger condition
- Never skip registration with `capability.create_skill`
- Never omit the `allowed-tools` list from the frontmatter
