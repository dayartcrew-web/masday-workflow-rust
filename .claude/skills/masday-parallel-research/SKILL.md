---
name: masday-parallel-research
description: >
  Orchestrates multi-branch research in parallel using masday workflow tools,
  stores branch results in memory, synthesizes the outputs, and saves one final
  report locally.
allowed-tools:
  - Agent
  - workflow_getActive
  - workflow_getCurrentTask
  - workflow_getPlan
  - workflow_createParallelBranches
  - workflow_listParallelBranches
  - workflow_completeParallelBranch
  - workflow_saveProgress
  - memory_recall_documents
  - memory_recall_document_by_type
  - memory_store
  - local_save_artifact
---

# Masday Parallel Research

Use only when the task naturally splits into 2+ independent research questions.

## Steps

This skill enforces **mandatory step completion**. Each step must be completed before proceeding. Do not skip steps.


1. **Get workflow context**
   - Call `workflow_getActive`.
   - Call `workflow_getCurrentTask`.
   - Call `workflow_getPlan`.

2. **Split research branches**
   - Identify independent research questions that can run without shared mutable state.
   - Define one stable branch key and one explicit scope per branch.

3. **Create branches**
   - Call `workflow_createParallelBranches` with the branch definitions.

4. **Dispatch branch workers**
   - Dispatch one `masday-researcher` agent per branch.
   - Give each branch worker only its branch scope plus the shared workflow context.
   - Branch workers persist findings via `memory_store_research` for synthesis.


**GATE**: Verify steps 1-4 are complete before proceeding.

5. **Monitor and complete branches**
   - Call `workflow_listParallelBranches` to monitor progress.
   - Call `workflow_completeParallelBranch` as each branch finishes.
   - Call `workflow_saveProgress` for key orchestration milestones.

6. **Synthesize results**
   - Dispatch `masday-synthesizer` after all research branches complete.
   - Merge branch findings into one coherent research result.

7. **Write the final artifact**
   - Store the synthesis summary in memory.
   - Write exactly one final local report with `local_save_artifact`.

## Never
- Never skip any step — complete each step before proceeding
- Never bypass a GATE marker without validating prior steps
- Never claim completion without executing all steps in order

- Never use this for a single research question.
- Never create dependent branches.
- Never write branch-level local artifacts.
- Never skip synthesis after branch completion.

## Mandatory Review Pipeline

When this skill completes work on a workflow task, it MUST follow the Masday review pipeline.
