---
name: masday-research
description: >
  Multi-source parallel research with codebase cross-referencing. Searches the web, semantic
  code search, and stored memory to synthesize findings. Persists results for future sessions.
  Use when the user says "research", "investigate", "find documentation", "compare approaches",
  or "look into".
allowed-tools:
  - WebSearch
  - semantic-search_code_search
  - semantic-search_search_hybrid_context_pack
  - memory_store_research
  - memory_search
  - memory_recall_documents
  - memory_recall_recent
---

# Masday Research

Multi-source parallel research synthesized against codebase and task.

If the task requires 2+ independent research questions with separate branch outputs, Use masday-parallel-research instead of this skill.

## Steps

This skill enforces **mandatory step completion**. Each step must be completed before proceeding. Do not skip steps.


1. **Parse the research question**
   - Extract the core topic, keywords, and scope
   - Identify libraries, frameworks, or patterns to investigate
   - Break into independent sub-queries for parallel research

2. **Check past research**
   - Call `memory_search` with the topic keywords to find existing research
   - Call `memory_recall_documents` for stored research documents
   - Call `memory_recall_recent` for recent session context
   - If sufficient past research exists, summarize and ask if the user wants to update it

3. **Search the web**
   - Call `WebSearch` with a descriptive query for each sub-topic
   - Focus on: official documentation, recent blog posts, GitHub repos, package registries
   - Collect URLs for deep reading

4. **Search the codebase**
   - Call `semantic-search_code_search` with queries matching the research topic
   - Call `semantic-search_search_hybrid_context_pack` for deep context on related code
   - Identify existing implementations, patterns, and gaps


**GATE**: Verify steps 1-4 are complete before proceeding.

5. **Synthesize findings**
   - Cross-reference external findings with codebase state
   - Identify gaps between research and current implementation
   - Rank recommendations by task relevance
   - Cite specific file paths and line numbers for codebase references

6. **Persist findings**
   - Call `memory_store_research` with:
     - `workflow_id`: current workflow ID (if in workflow context)
     - `summary`: brief research summary (1-2 sentences)
     - `content`: full research findings with sources
     - `created_by_agent`: "masday-researcher"

7. **Report**
   ```
   Research: [topic]

   ## Findings
   1. Key finding from web research (source: URL)
   2. Key finding from codebase analysis (source: file.ts:42)
   3. Gap identified: no existing implementation for X

   ## Recommendations
   1. [High] Specific action with code example
   2. [Medium] Specific action

   ## Sources
   - [Title](url)
   - packages/core/src/types.ts (existing pattern)
   ```

## Never
- Never skip any step — complete each step before proceeding
- Never bypass a GATE marker without validating prior steps
- Never claim completion without executing all steps in order

- Never fabricate URLs -- only cite URLs returned by search tools
- Never skip the codebase cross-reference step
- Never skip storing findings with `memory_store_research`
- Never present opinions as facts -- distinguish findings from recommendations

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
