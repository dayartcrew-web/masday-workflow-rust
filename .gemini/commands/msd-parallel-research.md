Run independent research branches in parallel and merge results.

## Purpose

When a task requires multiple independent research questions, dispatch parallel researcher agents to investigate simultaneously, then merge results into one coherent research output.

## When to Use

Use this command when:
- A task has 2+ independent research questions
- Research questions don't depend on each other's results
- Time/efficiency matters (parallel > sequential)

Do NOT use when:
- Research questions depend on each other (use sequential research)
- Only one research question (use msd-researcher directly)
- Research is trivial (can be answered by reading existing code)

## Steps

### 0. Get Workflow Context (MANDATORY FIRST STEP)
```
CRITICAL: Always start by getting the active workflow and task.

mcp__workflow-orchestrator__workflow_get_active({ cwd: process.cwd() })
  → Returns: { id, name, status }
  Record: workflowId

mcp__workflow-orchestrator__workflow_get_current_task({ workflow_id: workflowId })
  → Returns: { id, title, status, acceptanceCriteria }
  Record: taskId, title, acceptanceCriteria

If no active workflow or task:
  → STOP: "No active workflow/task. Run /msd-start-work first."
```

### 1. Identify Research Branches
```
Read the current task's acceptance criteria and required context.
Identify independent research questions.

Example:
  Task: "Add OpenAI embedding support"
  Acceptance Criteria: ["Supports OpenAI embeddings", "Stores in pgvector", "Migrates existing data"]
  Branches:
    1. "How does the current embedding provider interface work?"
    2. "What is the OpenAI embeddings API format and auth?"
    3. "How does pgvector store and query embeddings in Prisma?"

Rules for branch splitting:
- Each branch must answer ONE independent question
- Branches must NOT depend on each other
- Each branch should produce standalone, consumable output
- Each branch maps to one or more acceptance criteria
```

### 2. Dispatch Parallel Researchers
```
For each branch, dispatch an msd-researcher agent using:

Agent({ subagent_type: "msd-researcher" }) with prompt:

"Research question: {question 1}
WorkflowId: {workflowId}
TaskId: {taskId}
Task title: {taskTitle}

Your research should address these acceptance criteria:
{list of relevant acceptance criteria}

Research independently and store findings via:
1. memory.store_research() with sourceType 'research'
2. Write to .msd/context/research/YYYY-MM-DD-{topic-slug}.md

Return your findings with memory.store_research response (includes id)."

IMPORTANT: Dispatch ALL agents in ONE message with multiple Agent() calls.
Each agent independently:
1. Searches for information (Context7, WebSearch, codebase)
2. Validates findings
3. Stores results via memory.store_research
4. Returns memory ID for synthesis
```

### 3. Wait for All Branches
```
Wait until all parallel agents complete.
Collect each agent's findings.

Each agent should return:
- Memory ID from memory.store_research
- Local file path (.msd/context/research/YYYY-MM-DD-{topic}.md)
- Summary of findings

If any branch fails:
  → Note the failure
  → Continue with successful branches
  → Re-dispatch failed branch if possible
```

### 4. Trigger Synthesis
```
After all branches complete, dispatch msd-synthesizer:

Agent({ subagent_type: "msd-synthesizer" }) with prompt:

"Synthesize {N} research branches for task '{taskTitle}'.

WorkflowId: {workflowId}
TaskId: {taskId}
Task acceptance criteria: {list}

Research branches completed:
{numbered list of each branch question}

The msd-synthesizer has access to:
- mcp__memory__memory_recall_document_by_type (to fetch research)
- mcp__workflow-orchestrator__workflow_get_active (for context)
- mcp__workflow-orchestrator__workflow_get_current_task (for criteria)

Synthesis requirements:
1. Fetch all research documents with sourceType 'research'
2. Detect and resolve contradictions
3. Remove duplication
4. Verify all acceptance criteria are addressed
5. Produce one coherent research report
6. Save progress via workflow.saveProgress
7. Write output to .msd/reports/YYYY-MM-DD-{task-slug}-synthesis.md"

The synthesizer will:
1. Get workflow context automatically
2. Fetch research results from memory
3. Merge findings, resolve conflicts
4. Verify against acceptance criteria
5. Save synthesis report and progress
```

### 5. Validate Merged Result
```
Dispatch msd-reviewer to validate the synthesis:

Agent: msd-reviewer
prompt: "Review research synthesis for task '{task title}'.
Verify: completeness (all branches represented), accuracy,
no contradictions remaining, actionable for executor."

If reviewer APPROVES → research is complete, ready for implementation
If reviewer REJECTS → re-run failed branches with feedback
```

## Parallel Dispatch Pattern

```
// CRITICAL: All agents dispatched in ONE message with subagent_type parameter
// Get workflow context first (Step 0), then:

Agent({ subagent_type: "msd-researcher", prompt: "Research branch 1: ..." })
Agent({ subagent_type: "msd-researcher", prompt: "Research branch 2: ..." })
Agent({ subagent_type: "msd-researcher", prompt: "Research branch 3: ..." })

// After all complete, collect results (memory IDs, summaries)

// Then dispatch synthesizer with full context:
Agent({ subagent_type: "msd-synthesizer", prompt: "Synthesize {N} research branches for task '{title}' | WorkflowId: {id} | TaskId: {id} | Branches: {list}" })

// After synthesis:
Agent({ subagent_type: "msd-reviewer", prompt: "Validate synthesis against acceptance criteria ..." })
```

## Output

```
══════════════════════════════════════════
   Parallel Research Complete: {task title}
══════════════════════════════════════════

Workflow: {workflowId} | Task: {taskId}
Branches: {count} dispatched, {count} completed, {count} failed
Synthesis: {merged/conflicts found}
Review: {APPROVED/REWORK_REQUIRED/BLOCKED}

══════════════════════════════════════════
   KEY FINDINGS (from synthesis)
══════════════════════════════════════════

1. {from branch 1 - memory ID: xxx}
2. {from branch 2 - memory ID: xxx}
3. {from branch 3 - memory ID: xxx}

══════════════════════════════════════════
   ACCEPTANCE CRITERIA COVERAGE
══════════════════════════════════════════

✅ {AC 1} - Covered by branches {1,2}
✅ {AC 2} - Covered by branches {2,3}
⚠️  {AC 3} - Partial coverage, additional research needed

══════════════════════════════════════════
   ARTIFACTS
══════════════════════════════════════════

Research stored:
- .msd/context/research/YYYY-MM-DD-{topic1}.md
- .msd/context/research/YYYY-MM-DD-{topic2}.md
- .msd/context/research/YYYY-MM-DD-{topic3}.md

Synthesis report:
- .msd/reports/YYYY-MM-DD-{task-slug}-synthesis.md

Memory entries:
- {count} research documents stored via memory.store_research

══════════════════════════════════════════
   NEXT STEPS
══════════════════════════════════════════

{if APPROVED}
→ Research complete! Ready for implementation.
→ Run: /msd-implement (research context will be auto-loaded)

{if REWORK_REQUIRED}
→ Gaps identified. See synthesis report for details.
→ Re-run failed branches or continue with available findings.

{if BLOCKED}
→ Critical issue found. See review output for details.
→ Manual intervention required before proceeding.
```
