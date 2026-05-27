---
name: masday-create-mcp-skill
description: >
  Create a new TypeScript MCP skill -- an actual executable skill with Zod schemas registered
  in the MCP server. Uses shared-utils for agent/skill scaffolding. Generates the skill file,
  test file, and updates package exports. Use when the user says "create MCP skill",
  "new MCP tool", "add tool capability", or "executable skill".
allowed-tools:
  - filesystem_write
  - filesystem_list
  - filesystem_read
  - capability_scaffold_mcp_server
  - capability_scaffold_feature
  - capability_list_templates
  - npm_run
---

# Masday Create MCP Skill

Create a new MCP-executable skill (TypeScript) for the Masday server.

## Architecture Notes

- `capability_scaffold_feature` now uses `createAgent()` + `createSkill()` from `@mcp-rebuild/shared-utils` internally
- Agent/skill files are generated with proper YAML frontmatter, kebab-case validation, and optional model/tools fields
- The MCP server registers underscore aliases for all tools via `ToolNameRegistry` for Copilot/Codex compatibility

## Steps

This skill enforces **mandatory step completion**. Each step must be completed before proceeding. Do not skip steps.


1. **Determine target package**
   - `packages/skills/src/` -- filesystem-type skills (read, write, list, delete, stat)
   - `packages/code-skills/src/` -- code-related skills (git, tests, npm, docker, github, cicd)
   - Check with the user which package is appropriate

2. **Check existing skills**
   - Call `filesystem_list` on the target package's `src/` directory
   - Verify no duplicate skill names exist

3. **Check templates**
   - Call `capability_list_templates` for available scaffolding patterns

4. **Generate TypeScript skill file**
   - Use Zod for input/output schemas
   - Follow the project pattern:
   ```typescript
   import { z } from 'zod';
   import type { Skill } from '@masday-workflow-reborn/core';

   export const categorySkills: Skill[] = [
     {
       name: 'category.action',
       description: 'What this tool does',
       inputSchema: z.object({
         param: z.string().describe('Parameter description'),
       }).shape,
       outputSchema: z.object({
         result: z.string().describe('Result description'),
       }).shape,
       execute: async (input) => {
         // Implementation
         return { result: 'output' };
       },
     },
   ];
   ```

5. **Generate test file**
   - Create `<name>.test.ts` alongside the source file
   - Follow vitest pattern with describe/it blocks
   - Test success and failure cases


**GATE**: Verify steps 1-5 are complete before proceeding.

6. **Register in package index**
   - Call `filesystem_read` on the package's `index.ts`
   - Add the new skill export
   - Call `filesystem_write` to update

7. **Register in MCP server**
   - Call `filesystem_read` on `apps/agent-runner/src/runtime/mcp.ts`
   - Import and register the new skill in the tool map
   - The registerTool wrapper auto-creates underscore aliases via `ToolNameRegistry`
   - Call `filesystem_write` to update

8. **Build and verify**
   - Call `npm_run` with script `build` to verify compilation
   - Fix any TypeScript errors

9. **Store registration**
   - Report the new skill name, package location, and registered tool count

## Never
- Never skip any step — complete each step before proceeding
- Never bypass a GATE marker without validating prior steps
- Never claim completion without executing all steps in order

- Never skip writing the test file
- Never use `any` types -- use Zod schemas for all inputs and outputs
- Never forget to update both `index.ts` and `mcp.ts` registrations
- Never skip the build verification step
- Never create skills without proper error handling in the execute function

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
