---
name: masday-create-mcp-skill
description: Create a new TypeScript MCP skill — actual executable skill registered in the MCP server
argument-hint: [category.action] [description]
disable-model-invocation: true
allowed-tools: filesystem.read filesystem.write filesystem.list Bash
---

Create a new MCP-executable skill (TypeScript).

## Input
$ARGUMENTS — skill name (e.g., "docker.compose") and description.

## Steps

1. **Detect project root** — run `git rev-parse --show-toplevel` to find the masday-workflow-reborn repo root. Use this as `$ROOT` in subsequent steps.
2. **Parse input** — extract category, action, description
3. **Choose package**:
   - `filesystem` type → `$ROOT/packages/skills/src/<name>.ts`
   - `code` type → `$ROOT/packages/code-skills/src/<name>.ts`
3. **Generate TypeScript**:
   ```typescript
   import { z } from 'zod';
   import { createLogger } from '@masday-workflow-reborn/core';

   const logger = createLogger('<name>');

   export const <name>Skills = [
     {
       name: '<category>.<action>',
       description: '<desc>',
       inputSchema: z.object({
         // params
       }).shape,
       outputSchema: z.object({}).shape,
       execute: async (input: unknown) => {
         const parsed = z.object({}).parse(input);
         // implementation
         return { success: true };
       },
     },
   ];
   ```
4. **Update package index.ts** — add export
5. **Update MCP server** (`$ROOT/apps/agent-runner/src/mcp.ts`) — register skill
6. **Write test** — `<name>.test.ts` alongside source
   ```typescript
   import { z } from 'zod';
   import { createLogger } from '@masday-workflow-reborn/core';
   
   const logger = createLogger('<name>');
   
   export const <name>Skills = [
     {
       name: '<category>.<action>',
       description: '<desc>',
       inputSchema: z.object({
         // params
       }).shape,
       outputSchema: z.object({}).shape,
       execute: async (input: unknown) => {
         const parsed = z.object({}).parse(input);
         // implementation
         return { success: true };
       },
     },
   ];
   ```
4. **Update package index.ts** — add export
5. **Update MCP server** (`apps/agent-runner/src/mcp.ts`) — register skill
6. **Write test** — `<name>.test.ts` alongside source
7. **Build**: `cd $ROOT && pnpm build`
8. **Verify**: no compilation errors
9. **Report what was created**

## Rules
- Use Zod for all input/output validation
- Always parse input with z.object().parse()
- Use createLogger for structured logging
- Handle errors gracefully — return { error } not throw
- Write at least one test per skill
