Create or refine an implementation plan for the active workflow.

## Purpose

Generate a structured, executable plan with tasks, dependencies, acceptance criteria, and context requirements. Plans are the blueprint that agents follow during execution.

## Pre-conditions

- [ ] Active workflow exists (workflow.getActive)

If no workflow → STOP and suggest /msd-start-work first.

## Steps

### 1. Load Workflow Context
```
Call workflow.getActive → workflowId, title, description
Call workflow.getPlan → check if plan already exists

If plan exists:
  → Ask: "Refine existing plan or create new one?"
  → If refine → read existing tasks and identify gaps
  → If new → proceed with fresh planning

If no plan:
  → Proceed with fresh planning
```

### 1b. Collect PRD / Requirements (AskUserQuestion)
```
Ask the user for requirements input:

Question 1: "What are you building?"
  Options:
  - "I have a PRD" — paste or describe the Product Requirements Document
  - "I have a feature description" — describe the feature in plain text
  - "I have a list of requirements" — bullet list of what's needed
  - "Just plan from workflow title" — use existing workflow name only

Question 2: "Paste your PRD, feature description, or requirements here:"
  - Free text input (the user's full requirement document)
  - This becomes the plan's source of truth

Question 3: "Any constraints or priorities?"
  Options:
  - "Quality first" — prioritize tests and review gates
  - "Speed first" — minimal tasks, skip optional verification
  - "Balanced" — standard approach (default)

Store the PRD/requirements:
  Write to: .msd/context/prd.md
  Content: The full PRD text pasted by the user

Also call workflow.create_plan's autoDoc will store it as ContextDocument
with sourceType: "prd" automatically.

The PRD text is then passed to the planner agent as the primary input.
```

### 2. Apply Sequential Thinking
```
Before defining tasks, think through the implementation:

1. What is the END GOAL? (from PRD/requirements if provided, else workflow description)
2. What are the MAJOR COMPONENTS needed? (extract from PRD sections)
3. What is the ORDER of operations? (what depends on what?)
4. What EXTERNAL knowledge is needed?
5. What are the RISKS and unknowns?
6. What are the EXPLICIT REQUIREMENTS from PRD? (must-have vs nice-to-have)

If PRD was provided in step 1b:
  → Parse PRD sections: objectives, features, constraints, non-functional reqs
  → Map each requirement to one or more tasks
  → Ensure acceptance criteria trace back to PRD requirements
  → Non-functional requirements (performance, security) → dedicated tasks

Use this structure to organize tasks into phases:
- Phase 1: Foundation (types, schemas, database)
- Phase 2: Core logic (business rules, algorithms)
- Phase 3: Integration (MCP tools, API surface)
- Phase 4: Quality (tests, documentation)
```

### 3. Define Tasks
```
For each task, specify:

{
  title: "Imperative verb phrase (e.g., 'Add Zod schema for workflow inputs')",
  description: "1-3 sentences explaining what and why",
  acceptanceCriteria: [
    "Observable, testable condition",
    "Another condition",
    ...
  ],
  requiredContext: [
    "File or pattern to read before starting",
    "Documentation to reference",
    ...
  ],
  dependsOn: ["task-id-or-title"] | [],
  parallelizable: true | false,
  assignedAgent: "msd-executor" | "msd-researcher" | etc.
}

Task naming rules:
- Use imperative mood: "Add X", "Fix Y", "Implement Z"
- Be specific: not "Handle errors" but "Add Zod validation to workflow.create input"
- Keep titles under 80 characters
- Each task should be completable in one agent session
```

### 4. Mark Dependencies
```
For each pair of tasks, determine:
- Can they run in parallel? → parallelizable: true
- Must one finish before the other? → dependsOn: ["preceding task"]

Dependency patterns:
  Sequential:  A → B → C  (each depends on previous)
  Parallel:    A, B, C    (no dependencies between them)
  Mixed:       A → [B, C] → D  (A first, B and C parallel, D after all)

Rules:
- Tasks modifying the same file MUST be sequential
- Tasks touching independent modules CAN be parallel
- Database schema changes MUST come before ORM code
- Type definitions MUST come before implementations
- Tests SHOULD run after implementation (but can be written first in TDD)
```

### 5. Define Acceptance Criteria
```
Each task must have 2-5 acceptance criteria that are:
- Observable: can be verified by reading code or running tests
- Specific: not vague like "works correctly"
- Minimal: only what the task requires, no extras

Good examples:
  - "Zod schema validates all fields from shared-types/Workflow"
  - "pnpm build passes with zero errors"
  - "Test covers both success and error paths"
  - "Handler returns { success: true, data: Workflow } for valid input"

Bad examples:
  - "Code is clean"
  - "Everything works"
  - "Tests pass"
```

### 6. Define Required Context
```
For each task, list what the executor must read/know:

Context sources:
- File paths: "packages/shared-types/src/workflow.ts"
- Patterns: "Follow pattern from apps/workflow-orchestrator-mcp/src/tools/create-workflow.ts"
- External docs: "Context7: Prisma schema reference"
- Research: "memory.recall_by_task for prior research on X"

Every task should have at least 1 required context entry.
```

### 7. Submit Plan
```
Call workflow.create_plan with:
{
  workflowId: string,
  summary: string,  // Include PRD summary if provided
  tasks: [
    {
      title: string,
      description: string,
      acceptanceCriteria: string[],
      requiredContext: string[],
      dependsOn: string[],
      parallelizable: boolean,
      assignedAgent: string
    },
    ...
  ]
}

After submission:
  → Verify plan was stored correctly
  → Report task count and dependency graph

If PRD was provided in step 1b:
  → Save PRD to .msd/context/prd.md using Write tool
  → The autoDoc hook in workflow.create_plan will also store it
    as ContextDocument with sourceType: "plan"
  → Each task's acceptanceCriteria should trace back to PRD requirements
```

## Plan Quality Checklist

Before submitting, verify:
- [ ] Every task has at least 2 acceptance criteria
- [ ] Dependencies are correctly ordered (no circular deps)
- [ ] Parallelizable tasks don't modify the same files
- [ ] Task titles are specific and imperative
- [ ] Required context is actionable (file paths, not "read the docs")
- [ ] Total task count is reasonable (3-12 tasks per workflow)
- [ ] No task spans more than one agent session

## Output

```
Plan Created: {task count} tasks for "{workflow title}"

| # | Task | Depends On | Parallelizable | Agent |
|---|------|------------|----------------|-------|
| 1 | {title} | — | no | msd-executor |
| 2 | {title} | 1 | yes | msd-executor |
| ...

Next Step: /msd-implement to start executing
```
