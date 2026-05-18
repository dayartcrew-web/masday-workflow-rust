---
name: masday-web-research
description: >
  Web research with persistent storage. Searches the web for current information, fetches
  specific pages for detail, and stores all findings in memory for future sessions.
  Cross-references with codebase via semantic search. Use when the user says "web search",
  "look up", "find online", "research this", or "search the web".
allowed-tools:
  - WebSearch
  - search.hybrid_context_pack
  - search.code_search
  - memory.store_research
  - memory.search
  - memory.recall_documents
---

# Masday Web Research

Web research with storage and codebase cross-referencing.

## Steps

1. **Parse the research query**
   - Extract the core topic and scope from the user's request
   - Identify if this is: a technology lookup, a how-to question, a comparison, or a debugging search
   - Formulate the search query to be descriptive and specific

2. **Check for existing research**
   - Call `memory.search` with the topic to find prior research
   - Call `memory.recall_documents` for stored research documents
   - If current research exists (within 7 days), summarize and ask if the user wants fresh results

3. **Search the web**
   - Call `WebSearch` with a descriptive query
   - Example: "MCP protocol TypeScript SDK authentication patterns 2026"
   - Collect the top results with titles, URLs, and snippets

4. **Cross-reference with codebase**
   - Call `search.code_search` with related queries
   - Call `search.hybrid_context_pack` for deeper context
   - Identify where external findings relate to existing code

5. **Synthesize findings**
   - Combine web results with codebase context
   - Identify actionable insights relevant to the current task
   - Rank findings by relevance and actionability

6. **Store for future sessions**
   - Call `memory.store_research` with:
     - `topic`: the research topic
     - `findings`: synthesized key findings with sources
     - `source`: primary URLs and code references
     - `relevance_score`: 0.0-1.0 for task relevance
   - This makes findings available across sessions via `memory.search`

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

- Never fabricate URLs or search results
- Only cite URLs returned by the `WebSearch` tool
- Never skip storing findings with `memory.store_research`
- Never skip the codebase cross-reference step
- Never present web opinions as verified facts without caveats
