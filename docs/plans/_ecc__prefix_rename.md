# plugin_ecc_ Tools: Match & Replace with Global MCP

Date: 2026-05-20

## All 63 plugin_ecc_ tools by namespace

| # | Namespace | Tool Count | Global Match | Overlap |
|---|-----------|-----------|--------------|---------|
| 1 | plugin:ecc:context7 | 2 | plugin:context7 | EXACT |
| 2 | plugin:ecc:playwright | 23 | plugin:playwright | EXACT (22) + 1 renamed |
| 3 | plugin:ecc:exa | 2 | WebSearch + web_reader | FUNCTIONAL |
| 4 | plugin:ecc:github | 26 | masday__github (3) + zread (3) | PARTIAL |
| 5 | plugin:ecc:memory | 9 | masday__memory | NONE (different paradigm) |
| 6 | plugin:ecc:sequential-thinking | 1 | -- | UNIQUE |

## Detailed Match Table

### 1. context7 -- EXACT DUPLICATE (replace all)

| plugin_ecc | Global Replacement |
|-----------|-------------------|
| mcp__plugin_ecc_context7__resolve-library-id | mcp__plugin_context7_context7__resolve-library-id |
| mcp__plugin_ecc_context7__query-docs | mcp__plugin_context7_context7__query-docs |

### 2. playwright -- EXACT DUPLICATE (replace all)

| plugin_ecc | Global Replacement |
|-----------|-------------------|
| mcp__plugin_ecc_playwright__browser_click | mcp__plugin_playwright_playwright__browser_click |
| mcp__plugin_ecc_playwright__browser_close | mcp__plugin_playwright_playwright__browser_close |
| mcp__plugin_ecc_playwright__browser_console_messages | mcp__plugin_playwright_playwright__browser_console_messages |
| mcp__plugin_ecc_playwright__browser_drag | mcp__plugin_playwright_playwright__browser_drag |
| mcp__plugin_ecc_playwright__browser_drop | mcp__plugin_playwright_playwright__browser_drop |
| mcp__plugin_ecc_playwright__browser_evaluate | mcp__plugin_playwright_playwright__browser_evaluate |
| mcp__plugin_ecc_playwright__browser_file_upload | mcp__plugin_playwright_playwright__browser_file_upload |
| mcp__plugin_ecc_playwright__browser_fill_form | mcp__plugin_playwright_playwright__browser_fill_form |
| mcp__plugin_ecc_playwright__browser_handle_dialog | mcp__plugin_playwright_playwright__browser_handle_dialog |
| mcp__plugin_ecc_playwright__browser_hover | mcp__plugin_playwright_playwright__browser_hover |
| mcp__plugin_ecc_playwright__browser_navigate | mcp__plugin_playwright_playwright__browser_navigate |
| mcp__plugin_ecc_playwright__browser_navigate_back | mcp__plugin_playwright_playwright__browser_navigate_back |
| mcp__plugin_ecc_playwright__browser_network_request | mcp__plugin_playwright_playwright__browser_network_request |
| mcp__plugin_ecc_playwright__browser_network_requests | mcp__plugin_playwright_playwright__browser_network_requests |
| mcp__plugin_ecc_playwright__browser_press_key | mcp__plugin_playwright_playwright__browser_press_key |
| mcp__plugin_ecc_playwright__browser_resize | mcp__plugin_playwright_playwright__browser_resize |
| mcp__plugin_ecc_playwright__browser_run_code | mcp__plugin_playwright_playwright__browser_run_code_unsafe (renamed) |
| mcp__plugin_ecc_playwright__browser_select_option | mcp__plugin_playwright_playwright__browser_select_option |
| mcp__plugin_ecc_playwright__browser_snapshot | mcp__plugin_playwright_playwright__browser_snapshot |
| mcp__plugin_ecc_playwright__browser_tabs | mcp__plugin_playwright_playwright__browser_tabs |
| mcp__plugin_ecc_playwright__browser_take_screenshot | mcp__plugin_playwright_playwright__browser_take_screenshot |
| mcp__plugin_ecc_playwright__browser_type | mcp__plugin_playwright_playwright__browser_type |
| mcp__plugin_ecc_playwright__browser_wait_for | mcp__plugin_playwright_playwright__browser_wait_for |

### 3. exa -- FUNCTIONAL OVERLAP (REPLACED)

| plugin_ecc | Global Replacement | Note |
|-----------|-------------------|------|
| mcp__plugin_ecc_exa__web_search_exa | mcp__web-search-prime__web_search_prime | Keyword vs semantic search |
| mcp__plugin_ecc_exa__web_fetch_exa | mcp__web_reader__webReader | Already used alongside |

### 4. github -- PARTIAL OVERLAP (3 of 26 match masday)

| plugin_ecc | Global Replacement |
|-----------|-------------------|
| mcp__plugin_ecc_github__list_issues | mcp__masday__github_issue_list |
| mcp__plugin_ecc_github__list_pull_requests | mcp__masday__github_pr_list |
| mcp__plugin_ecc_github__create_pull_request | mcp__masday__github_pr_create |
| 23 others | No replacement -- keep plugin:ecc:github |

### 5. memory -- NO OVERLAP (keep both)

Knowledge graph (entities/relations) vs PostgreSQL memory store -- completely different systems.

### 6. sequential-thinking -- UNIQUE (no replacement exists)

## Project Files Changed

**File:** `.claude/agents/masday-researcher.md` -- 7 occurrences replaced

| Line | Before (plugin_ecc) | After (global) |
|------|---------------------|----------------|
| 12 | mcp__plugin_ecc_exa__web_search_exa | mcp__web-search-prime__web_search_prime |
| 13 | mcp__plugin_ecc_exa__web_fetch_exa | mcp__web_reader__webReader |
| 71 | mcp__plugin_ecc_exa__web_search_exa | mcp__web-search-prime__web_search_prime |
| 72 | mcp__plugin_ecc_exa__web_fetch_exa | mcp__web_reader__webReader |
| 104 | mcp__plugin_ecc_exa__web_search_exa | mcp__web-search-prime__web_search_prime |
| 109 | mcp__plugin_ecc_exa__web_fetch_exa | mcp__web_reader__webReader |

## Verification

```
grep -rn "plugin_ecc" --include="*.md" --include="*.ts" → 0 matches
```

## Summary

- 27 tools are exact duplicates (context7 + playwright) that duplicate global plugin tools
- 2 tools (exa) REPLACED with global equivalents in project files
- 23 tools (github deep operations) have no replacement -- keep plugin:ecc:github if needed
- 10 tools (memory + sequential-thinking) are unique -- no replacement exists
- 1 file modified, 0 remaining plugin_ecc references in project
