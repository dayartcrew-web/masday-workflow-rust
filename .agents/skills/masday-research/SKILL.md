---
name: masday-research
description: >
  Multi-source parallel research with codebase cross-referencing. Searches the web, semantic
  code search, and stored memory to synthesize findings. Persists results for future sessions.
  Use when the user says "research", "investigate", "find documentation", "compare approaches",
  or "look into".
allowed-tools:
  - WebSearch
  - semantic-search.code_search
  - semantic-search.search_hybrid_context_pack
  - memory.store_research
  - memory.search
  - memory.recall_documents
  - memory.recall_recent
---

# Masday Research

Multi-source parallel research synthesized against codebase and task.

## Steps

1. **Parse the research question**
   - Extract the core topic, keywords, and scope
   - Identify libraries, frameworks, or patterns to investigate
   - Break into independent sub-queries for parallel research

2. **Check past research**
   - Call `memory.search` with the topic keywords to find existing research
   - Call `memory.recall_documents` for stored research documents
   - Call `memory.recall_recent` for recent session context
   - If sufficient past research exists, summarize and ask if the user wants to update it

3. **Search the web**
   - Call `WebSearch` with a descriptive query for each sub-topic
   - Focus on: official documentation, recent blog posts, GitHub repos, package registries
   - Collect URLs for deep reading

4. **Search the codebase**
   - Call `semantic-search.code_search` with queries matching the research topic
   - Call `semantic-search.search_hybrid_context_pack` for deep context on related code
   - Identify existing implementations, patterns, and gaps

5. **Synthesize findings**
   - Cross-reference external findings with codebase state
   - Identify gaps between research and current implementation
   - Rank recommendations by task relevance
   - Cite specific file paths and line numbers for codebase references

6. **Persist findings**
   - Call `memory.store_research` with:
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

- Never fabricate URLs -- only cite URLs returned by search tools
- Never skip the codebase cross-reference step
- Never skip storing findings with `memory.store_research`
- Never present opinions as facts -- distinguish findings from recommendations
