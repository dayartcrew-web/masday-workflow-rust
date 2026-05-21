---
name: masday-create-skill
description: >
  Create a new Masday skill (SKILL.md) from a description. Designs trigger conditions,
  allowed tools, step-by-step workflow, and constraints. Uses shared-utils createSkill()
  for validation and file generation. Use when the user says "create skill", "new skill",
  "add workflow skill", or "skill definition".
allowed-tools:
  - capability_create_skill
  - capability_list_skills
  - capability_list_templates
  - capability_scaffold_feature
  - filesystem_write
  - filesystem_list
---

# Masday Create Skill

Create a new Masday Workflow skill definition.

## Validation Rules

Skill names must:
- Be kebab-case (e.g., `masday-deploy-check`, `masday-security-scan`)
- Start with a letter
- Be 2-64 characters
- Match regex: `^[a-z][a-z0-9-]{1,63}$`

Validation is enforced by `createSkill()` in `@mcp-rebuild/shared-utils`.

## Steps

1. **Check existing skills**
   - Call `capability_list_skills` to see all registered skills
   - Verify the proposed skill name does not conflict with existing ones

2. **Check templates**
   - Call `capability_list_templates` for available patterns
   - Select a template if one matches the skill type

3. **Design the skill based on user description**
   - Name: kebab-case (e.g., `masday-deploy-check`, `masday-security-scan`)
   - Description: WHAT it does + WHEN to use it + trigger phrases
   - Allowed tools: only actual MCP tool names from the 87 available
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
   - Call `capability_create_skill` with:
     - `projectRoot`: the project root directory
     - `name`: skill name (validated by shared-utils)
     - `description`: what it does
     - `trigger`: when to activate
     - `steps`: array of step descriptions
   - The tool internally calls `createSkill()` from shared-utils which:
     - Validates the name against kebab-case rules
     - Creates `.claude/skills/<name>/` directory if needed
     - Generates YAML frontmatter with name, description, trigger, allowed-tools
     - Writes the SKILL.md file

6. **Save to project location**
   - Call `filesystem_write` to save:
     - `$ROOT/.claude/skills/<name>/SKILL.md`

7. **Or use scaffold for full feature**
   - Call `capability_scaffold_feature` if the skill needs an agent and command
   - scaffold_feature internally calls both `createAgent()` and `createSkill()` from shared-utils

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
- Never skip registration with `capability_create_skill`
- Never omit the `allowed-tools` list from the frontmatter

## Mandatory Review Pipeline

When this skill completes work on a workflow task, it MUST follow this pipeline:

`
STEP 1: Save progress to PostgreSQL
  workflow_saveProgress({
    workflow_id: "<workflowId>",
    task_id: "<taskId>",
    agent_name: "<current-agent>",
    progress_note: "<summary of work done>",
    evidence: ["<files modified>", "<tests run>"]
  })

STEP 2: Submit for review
  review_submit({
    workflow_id: "<workflowId>",
    task_id: "<taskId>",
    reviewer_agent: "masday-reviewer",
    decision: "<APPROVED | REWORK_REQUIRED | BLOCKED>",
    notes: "<what was done, key decisions>",
    gaps: ["<any gaps found>"]
  })

STEP 3: If REWORK_REQUIRED — fix and loop
  - Fix the gaps identified in the review
  - Re-save progress (workflow_saveProgress)
  - Re-submit review (review_submit)
  - Max 2 rework attempts, then STOP

STEP 4: If APPROVED — validate completion
  policy_validate_completion({
    workflow_id: "<workflowId>",
    task_id: "<taskId>"
  })

STEP 5: Complete task
  workflow_completeTask({ workflow_id: "<workflowId>", task_id: "<taskId>" })

STEP 6: Sync local state
  local_sync({ cwd: process.cwd(), workflow_id: "<workflowId>" })
`

### Never
- Never call workflow_completeTask without review_submit (APPROVED)
- Never skip policy_validate_completion before completion
- Never skip local_sync after completing a task
- Never claim done without saving progress to PostgreSQL
