---
name: masday-sequential-thinking
description: >
  Step-by-step reasoning and analysis using codebase reading tools. Breaks down complex
  problems into sequential analysis steps, reading relevant code at each stage to build
  understanding incrementally. Use when the user says "think through this", "analyze step
  by step", "reason about", "walk through", or "break down this problem".
allowed-tools:
  - filesystem.read
  - filesystem.list
  - filesystem.stat
---

# Masday Sequential Thinking

Step-by-step reasoning with codebase analysis.

## Steps

1. **Define the problem**
   - Parse the user's question or problem statement
   - Break it into sequential analysis steps
   - Identify what code or files need to be examined at each step

2. **Step 1: Survey the landscape**
   - Call `filesystem.list` to understand the project structure
   - Identify which packages and directories are relevant
   - Note the scale: how many files, how deep is the hierarchy

3. **Step 2: Read entry points**
   - Call `filesystem.stat` on key files to check sizes and dates
   - Call `filesystem.read` on entry points: package.json, index.ts, main files
   - Understand the public API surface

4. **Step 3: Trace the flow**
   - For each relevant module, call `filesystem.read` on the source file
   - Trace imports, function calls, and data flow
   - Map the dependency graph: what depends on what

5. **Step 4: Identify patterns**
   - Look for recurring patterns: error handling, state management, type definitions
   - Note conventions: naming, file organization, export structure
   - Check for consistency across modules

6. **Step 5: Locate the relevant code**
   - Focus reading on the files most relevant to the problem
   - Call `filesystem.read` on specific files mentioned in the problem
   - Cross-reference with related modules

7. **Step 6: Build the mental model**
   - Combine all readings into a coherent understanding
   - Identify: inputs, outputs, side effects, error paths
   - Map the flow from user request through the system

8. **Step 7: Formulate the answer**
   - Present the reasoning chain: what was found at each step
   - Support conclusions with specific file paths and line references
   - If the analysis is inconclusive, state what additional information is needed

9. **Report**
   ```
   === Sequential Analysis ===
   Problem: <problem statement>

   Step 1: Survey -> identified 3 relevant packages
   Step 2: Entry points -> core exports 12 types, orchestrator has 3 engine tiers
   Step 3: Flow traced -> user -> MCP protocol -> OrchestratingEngine -> tasks
   Step 4: Patterns -> immutable state, Zod validation, EventBus pub/sub
   Step 5: Located -> packages/core/src/types.ts defines WorkflowState
   Step 6: Model -> state machine with INIT->ANALYZE->PLAN->EXECUTE->VERIFY->DONE
   Step 7: Conclusion -> <answer with evidence>
   ```

## Never

- Never skip steps -- each step builds on the previous one
- Never read more than needed -- focus on files relevant to the problem
- Never fabricate file contents -- only reference what was actually read
- Never present conclusions without supporting evidence from the codebase
