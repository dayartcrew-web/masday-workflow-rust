---
name: masday-create-mcp-skill
description: >
  Create a new TypeScript MCP skill -- an actual executable skill with Zod schemas registered
  in the MCP server. Generates the skill file, test file, and updates package exports.
  Use when the user says "create MCP skill", "new MCP tool", "add tool capability",
  or "executable skill".
allowed-tools:
  - filesystem.write
  - filesystem.list
  - filesystem.read
  - capability.scaffold_mcp_server
  - capability.list_templates
  - npm.run
---

# Masday Create MCP Skill

Create a new MCP-executable skill (TypeScript) for the Masday server.

## Steps

1. **Determine target package**
   - `packages/skills/src/` -- filesystem-type skills (read, write, list, delete, stat)
   - `packages/code-skills/src/` -- code-related skills (git, tests, npm, docker, github, cicd)
   - Check with the user which package is appropriate

2. **Check existing skills**
   - Call `filesystem.list` on the target package's `src/` directory
   - Verify no duplicate skill names exist

3. **Check templates**
   - Call `capability.list_templates` for available scaffolding patterns

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

6. **Register in package index**
   - Call `filesystem.read` on the package's `index.ts`
   - Add the new skill export
   - Call `filesystem.write` to update

7. **Register in MCP server**
   - Call `filesystem.read` on `apps/agent-runner/src/mcp.ts`
   - Import and register the new skill in the tool map
   - Call `filesystem.write` to update

8. **Build and verify**
   - Call `npm.run` with script `build` to verify compilation
   - Fix any TypeScript errors

9. **Store registration**
   - Report the new skill name, package location, and registered tool count

## Never

- Never skip writing the test file
- Never use `any` types -- use Zod schemas for all inputs and outputs
- Never forget to update both `index.ts` and `mcp.ts` registrations
- Never skip the build verification step
- Never create skills without proper error handling in the execute function
