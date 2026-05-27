---
name: masday-web-research
description: >
  Web research with persistent storage. Searches the web for current information, fetches
  specific pages for detail, and stores all findings in memory for future sessions.
  Cross-references with codebase via semantic search. Use when the user says "web search",
  "look up", "find online", "research this", or "search the web".
allowed-tools:
  - WebSearch
  - semantic-search_search_hybrid_context_pack
  - semantic-search_code_search
  - memory_store_research
  - memory_search
  - memory_recall_documents
---

# Masday Web Research

Web research with storage and codebase cross-referencing.

## Steps

This skill enforces **mandatory step completion**. Each step must be completed before proceeding. Do not skip steps.


1. **Parse the research query**
   - Extract the core topic and scope from the user's request
   - Identify if this is: a technology lookup, a how-to question, a comparison, or a debugging search
   - Formulate the search query to be descriptive and specific

2. **Check for existing research**
   - Call `memory_search` with the topic to find prior research
   - Call `memory_recall_documents` for stored research documents
   - If current research exists (within 7 days), summarize and ask if the user wants fresh results

3. **Search the web**
   - Call `WebSearch` with a descriptive query
   - Example: "MCP protocol TypeScript SDK authentication patterns 2026"
   - Collect the top results with titles, URLs, and snippets

4. **Cross-reference with codebase**
   - Call `semantic-search_code_search` with related queries
   - Call `semantic-search_search_hybrid_context_pack` for deeper context
   - Identify where external findings relate to existing code


**GATE**: Verify steps 1-4 are complete before proceeding.

5. **Synthesize findings**
   - Combine web results with codebase context
   - Identify actionable insights relevant to the current task
   - Rank findings by relevance and actionability

6. **Store for future sessions**
   - Call `memory_store_research` with:
     - `topic`: the research topic
     - `findings`: synthesized key findings with sources
     - `source`: primary URLs and code references
     - `relevance_score`: 0.0-1.0 for task relevance
   - This makes findings available across sessions via `memory_search`

7. **Report**
   ```
   === Web Research ===
   Topic: <topic>

   ## Key Findings
   1. Finding from web (source: URL)
   2. Related codebase pattern (source: file.ts:42)

   ## Codebase Context
   - Existing implementation at packages/core/src/auth.ts uses similar pattern
   - Gap: no rate limiting implemented yet

   ## Recommendations
   1. [High] Specific action
   2. [Medium] Specific action

   ## Sources
   - [Title](url)

   Stored in memory for future sessions.
   ```

## Never
- Never skip any step — complete each step before proceeding
- Never bypass a GATE marker without validating prior steps
- Never claim completion without executing all steps in order

- Never fabricate URLs or search results
- Only cite URLs returned by the `WebSearch` tool
- Never skip storing findings with `memory_store_research`
- Never skip the codebase cross-reference step
- Never present web opinions as verified facts without caveats

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
