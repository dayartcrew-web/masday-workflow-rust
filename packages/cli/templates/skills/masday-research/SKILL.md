---
name: masday-research
description: Parallel external research using Context7, web search, and web fetch — synthesizes with codebase context
allowed-tools: mcp__plugin_context7_context7__resolve-library-id mcp__plugin_context7_context7__query-docs WebSearch mcp__web_reader__webReader Read Grep Glob
disable-model-invocation: false
---

# Research Skill

Parallel multi-source research synthesized against codebase and task.

## Trigger
Use when the user asks to research, investigate, or find information about libraries, frameworks, patterns, or approaches — especially before implementation.

## Workflow

### 1. Scope
Parse the research question. Identify:
- Libraries needing Context7 docs
- Keywords for web search
- Codebase areas to cross-reference
- Break into independent sub-queries

### 2. Parallel Research
Dispatch all independent queries at once:
- **Context7**: `resolve-library-id` → `query-docs` for each library
- **WebSearch**: broad discovery for patterns, alternatives, recent changes
- **Codebase**: `Grep`/`Glob` for existing implementations

If Context7 initial query is insufficient, retry with `researchMode: true`.

### 3. Deep Read
Fetch up to 3 specific URLs found via search using `webReader`.

### 4. Synthesize
- Cross-reference external findings with codebase (cite file:line)
- Identify gaps between research and current implementation
- Rank recommendations by task relevance

### 5. Report Format
```
🔍 Research: [topic]

## Findings
- Key fact 1 (source: Context7/docs)
- Key fact 2 (source: URL)

## Codebase Context
- src/path/file.ts:42 — existing pattern aligns with finding
- No implementation found for X

## Recommendations
1. [High] Action with code example
2. [Medium] Action

## Sources
- [Title](url)
```

## Constraints
- Resolve Context7 library ID before querying docs
- Max 3 URL fetches per session
- Never fabricate URLs — only cite URLs from tool results
- Always cross-reference with codebase
- Lead with actionable findings
