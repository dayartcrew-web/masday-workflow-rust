---
name: masday-frontend-review
description: >
  Visual review and frontend code audit. Analyzes running dev servers via Playwright browser
  screenshots, audits CSS/TSX/Vue codebase for design token consistency, hardcoded values,
  accessibility issues, and responsive problems. Outputs scored PASS/WARN/FAIL report.
  Dispatches to masday-visual-frontend for design token regeneration when gaps are found.
disable-model-invocation: false
allowed-tools: >
  Read Write Edit Bash Glob Grep Agent
  mcp__plugin_playwright_playwright__browser_navigate
  mcp__plugin_playwright_playwright__browser_snapshot
  mcp__plugin_playwright_playwright__browser_take_screenshot
  mcp__plugin_playwright_playwright__browser_click
  mcp__plugin_playwright_playwright__browser_evaluate
  mcp__plugin_playwright_playwright__browser_console_messages
  mcp__plugin_playwright_playwright__browser_network_requests
  mcp__plugin_playwright_playwright__browser_resize
  mcp__4_5v_mcp__analyze_image
  mcp__masday__memory_store
  mcp__masday__memory_search
  mcp__masday__workflow_saveProgress
  mcp__masday__review_submit
  mcp__masday__policy_validate_completion
  mcp__masday__workflow_completeTask
  mcp__masday__local_sync
  mcp__masday__local_init
  mcp__masday__semantic-search_code_search
  filesystem_read filesystem_write filesystem_list
context: fork
---

# MSD Frontend Review

Visual review and frontend code audit with scored reporting. Analyzes running apps via Playwright, audits codebase for design token consistency, and dispatches to `masday-visual-frontend` for fixes.

## Quick Start

> No input detected. Choose a review mode:

| Mode | Example |
|------|---------|
| **Browser Review** | `/masday-frontend-review browser` |
| **Code Audit** | `/masday-frontend-review code` |
| **Full Review** | `/masday-frontend-review full` (browser + code) |
| **Compare vs Reference** | `/masday-frontend-review compare https://stripe.com` |

## Execution Model

This skill **dispatches to the `masday-frontend-reviewer` agent** for actual review work. The skill handles mode detection and routing; the agent executes the audit with full tool access.

```
User calls /masday-frontend-review <mode>
  → Skill detects mode from arguments
  → Skill auto-detects project config (framework, routes, tokens)
  → Skill dispatches to masday-frontend-reviewer agent with context
  → Agent executes review step-by-step (visible progress)
  → Agent returns scored report
  → Skill handles fix dispatch if needed
```

**Dispatch pattern:**
```
Agent({
  subagent_type: "masday-frontend-reviewer",
  prompt: "Execute {mode} review for project at {cwd}.
    Framework: {framework}
    Styling: {styling}
    Pages: {routes}
    Token files: {token_files}
    Dev server: {url}

    Follow your 5-phase workflow. Report findings with severity levels."
})
```

## Modes

### `browser` — Visual Browser Review

Audit a running dev server using Playwright:

1. **Start dev server** (if not running)
2. **Navigate to each page route** — screenshot + accessibility snapshot
3. **Visual checks per page**:
   - Layout rendering (no overflow, alignment issues)
   - Color contrast (WCAG AA minimum 4.5:1)
   - Touch target sizes (44x44px minimum)
   - Keyboard navigation (tab order, focus visible)
   - Console errors / warnings
   - Failed network requests (4xx/5xx)
4. **Responsive test** at 375 / 768 / 1440 breakpoints
5. **Output**: Scored report per page + overall score

### `code` — Codebase Consistency Audit

Scan frontend source files for design token discipline:

1. **Auto-detect stack**: framework, styling approach, token location
2. **Token extraction**:
   - Read CSS custom properties from `:root`
   - Read `tailwind.config.ts` theme.extend
   - Read token files (`tokens.ts`, `theme.ts`, `design-tokens.*`)
3. **Consistency checks**:
   - **Hardcoded colors**: Find hex/rgb/hsl values NOT referencing tokens
   - **Hardcoded spacing**: Find px/rem values NOT using spacing scale
   - **Hardcoded typography**: Find font-size/weight NOT using type scale
   - **Duplicate values**: Same color/spacing used with different variable names
   - **Dead tokens**: Tokens defined but never referenced
   - **Inconsistent patterns**: Same component type using different token references
4. **Accessibility check**:
   - Missing `alt` attributes on images
   - Missing `aria-label` on interactive elements
   - Missing `role` attributes where needed
   - Color contrast violations in CSS
5. **Output**: Scored report per category + file-level findings

### `full` — Browser + Code Review

Runs both modes sequentially, then merges into a unified report.

### `compare <url>` — Compare Against Reference

Compare the local dev server against a reference website:

1. Navigate to reference URL → screenshot + extract CSS vars
2. Navigate to local dev server → screenshot + extract CSS vars
3. Side-by-side comparison:
   - Color palette match
   - Typography match
   - Spacing patterns match
   - Component styling match
4. **Output**: Similarity percentage per category + gaps list
5. **Dispatch**: If gaps found → delegate to `masday-visual-frontend` for token regeneration

## Pipeline (5 Phases)

### Phase 1: Setup & Discovery

```
1. Parse mode from arguments (browser / code / full / compare <url>)
2. Auto-detect project:
   - Read package.json for framework, styling, scripts
   - Glob for page routes (app/**/page.tsx, pages/**/*.tsx, src/routes/*)
   - Detect dev server port from scripts or .env
3. If browser/full/compare mode:
   - Check if dev server is running (curl localhost:<port>)
   - If not running: suggest start command, wait for user
4. Store config:
   - FRONTEND_URL, PAGES[], FRAMEWORK, STYLING_APPROACH, TOKEN_FILES[]
```

**GATE 1**: Mode identified, project config loaded, dev server accessible (if browser mode).

### Phase 2: Execute Review

#### Browser Review Execution

```
For each page route:
  1. browser_navigate({ url: FRONTEND_URL + route })
  2. browser_take_screenshot() — desktop baseline
  3. browser_snapshot() — accessibility tree
  4. browser_evaluate({ script: contrastCheckScript }) — WCAG contrast
  5. browser_evaluate({ script: touchTargetCheckScript }) — 44x44px
  6. browser_console_messages({ level: "error" })
  7. browser_network_requests({ static: false })
  8. browser_resize({ width: 375 }) → browser_take_screenshot()
  9. browser_resize({ width: 768 }) → browser_take_screenshot()
  10. browser_resize({ width: 1440 }) → browser_take_screenshot()

  Collect: screenshots[], accessibility_issues[], console_errors[], failed_requests[]
```

**Contrast check script:**
```javascript
() => {
  const violations = [];
  document.querySelectorAll('*').forEach(el => {
    const cs = getComputedStyle(el);
    const bg = cs.backgroundColor;
    const fg = cs.color;
    if (fg && bg && fg !== 'rgba(0, 0, 0, 0)') {
      // Simplified contrast check — flag low-contrast text
      const small = parseFloat(cs.fontSize) < 18;
      // Full WCAG check needs luminance calculation
    }
  });
  return JSON.stringify({ violationCount: violations.length, violations: violations.slice(0, 20) });
}
```

**Touch target check script:**
```javascript
() => {
  const small = [];
  document.querySelectorAll('button, a, [role="button"], input, select, textarea').forEach(el => {
    const r = el.getBoundingClientRect();
    if (r.width < 44 || r.height < 44) small.push({ tag: el.tagName, w: r.width, h: r.height, text: el.textContent?.slice(0,30) });
  });
  return JSON.stringify(small);
}
```

#### Code Audit Execution

```
1. Scan for hardcoded colors:
   Grep: #[0-9a-fA-F]{3,8}(?![0-9a-fA-F]) — exclude token definition files
   Grep: rgb\(| hsl\( — exclude token definition files

2. Scan for hardcoded spacing:
   Grep: \d+px(?![^(]*\)) in .tsx/.vue/.css/.scss — exclude 0px, border, transform
   Grep: gap-\[|p-\[|m-\[|w-\[|h-\[ — Tailwind arbitrary values

3. Check token consistency:
   For each token file, verify:
   - All colors in palette are actually used
   - No duplicate semantic meaning (e.g., --primary and --brand pointing to same hex)
   - Spacing scale is complete (xs, sm, md, lg, xl minimum)

4. Accessibility scan:
   Grep: <img[^>]*(?!alt=) — images without alt
   Grep: <(button|a)[^>]*(?!aria-label)(?!>) — interactive without aria-label
   Grep: onClick(?!\s*=\s*\{[^}]*role) — click handlers without role

5. Collect findings per file with severity levels
```

#### Compare Mode Execution

```
1. browser_navigate({ url: REFERENCE_URL })
2. browser_take_screenshot() → reference_screenshot
3. browser_evaluate({ cssExtractionScript }) → reference_tokens
4. browser_navigate({ url: LOCAL_URL })
5. browser_take_screenshot() → local_screenshot
6. browser_evaluate({ cssExtractionScript }) → local_tokens
7. Compare token sets:
   - colors: match %, missing colors, extra colors
   - typography: font match, scale match
   - spacing: scale match
   - shadows: match
8. Output similarity scores + gap analysis
```

**GATE 2**: Review data collected. Must have findings for each enabled check category.

### Phase 3: Score & Report

Score each finding across 5 categories:

| Category | Weight | Scoring |
|----------|--------|---------|
| **Visual Rendering** | 20% | Pages render correctly, no layout breaks |
| **Accessibility** | 25% | WCAG AA contrast, touch targets, aria, keyboard nav |
| **Token Discipline** | 20% | No hardcoded values, consistent token usage |
| **Responsive** | 15% | All breakpoints render without overflow |
| **Console/Network Health** | 10% | Zero errors, zero failed requests |

**Severity levels:**
- **CRITICAL** — Broken layout, accessibility violations, hardcoded secrets
- **WARN** — Hardcoded colors/spacing, missing aria, small touch targets
- **INFO** — Dead tokens, unused imports, minor inconsistencies
- **PASS** — No issues found

**Report format:**
```
## Frontend Review Report

### Project: {name}
### Mode: {browser | code | full | compare}
### Date: {timestamp}

### Overall Score: {X}/100

| Category | Score | Status |
|----------|-------|--------|
| Visual Rendering | {score}/100 | ✅ PASS / ⚠️ WARN / ❌ FAIL |
| Accessibility | {score}/100 | ✅ PASS / ⚠️ WARN / ❌ FAIL |
| Token Discipline | {score}/100 | ✅ PASS / ⚠️ WARN / ❌ FAIL |
| Responsive | {score}/100 | ✅ PASS / ⚠️ WARN / ❌ FAIL |
| Console/Network | {score}/100 | ✅ PASS / ⚠️ WARN / ❌ FAIL |

### Critical Issues ({count})
- [{file}:{line}] {description}

### Warnings ({count})
- [{file}:{line}] {description}

### Info ({count})
- [{file}:{line}] {description}

### Pages Tested: {tested}/{total}
| Page | Route | Rendering | Console Errors | Notes |
|------|-------|-----------|----------------|-------|

### Token Audit
- Tokens defined: {count}
- Tokens used: {count}
- Dead tokens: {list}
- Hardcoded values found: {count} across {files} files

### Responsive Summary
| Breakpoint | Pages OK | Pages Broken |
|------------|----------|-------------|
| Mobile (375px) | {n} | {n} |
| Tablet (768px) | {n} | {n} |
| Desktop (1440px) | {n} | {n} |
```

**GATE 3**: Report generated with scores for all categories.

### Phase 4: Dispatch Fixes (if needed)

If CRITICAL or WARN findings exist:

```
IF token discipline issues found (hardcoded values):
  → Dispatch to masday-visual-frontend skill
  → Pass: token files + hardcoded values list + expected tokens
  → masday-visual-frontend regenerates/fixes tokens

IF visual rendering broken:
  → Dispatch to masday-visual-frontend skill (Component mode)
  → Pass: broken component files + expected design from reference
  → masday-visual-frontend rebuilds components

IF accessibility issues found:
  → List specific fixes needed (aria-label additions, contrast fixes)
  → Ask user: auto-fix or manual review
```

**GATE 4**: Fix dispatches completed or user confirmed no fixes needed.

### Phase 5: Store & Complete

```
1. memory_store({ type: "artifact", summary: "Frontend review for {project}", content: report })
2. If in workflow context: review_submit + workflow_completeTask + local_sync
```

## Error Handling

| Error | Action |
|-------|--------|
| Dev server not running | Suggest start command, wait |
| No page routes found | Ask user for routes manually |
| No token files found | Skip token audit, report as INFO |
| Browser crash | Restart browser, retry page |
| CORS on CSS extraction | Use computed styles fallback |

## Rules

- ALWAYS score findings — never just list issues without severity
- ALWAYS include file path + line number for code findings
- ALWAYS screenshot before and after any fixes
- NEVER auto-fix CRITICAL issues without user confirmation
- NEVER skip accessibility checks — they are mandatory
- Store reports via `memory_store` for cross-session tracking

## Mandatory Review Pipeline

When this skill completes work on a workflow task:

```
STEP 1: Save progress
  workflow_saveProgress({ workflow_id, task_id, agent_name: "masday-frontend-review", progress_note, evidence })

STEP 2: Submit review
  review_submit({ workflow_id, task_id, reviewer_agent: "masday-reviewer", decision, notes, gaps })

STEP 3: If REWORK_REQUIRED — fix and loop (max 2)

STEP 4: If APPROVED — validate and complete
  policy_validate_completion({ workflow_id, task_id })
  workflow_completeTask({ workflow_id, task_id })
  local_sync({ cwd, workflow_id })
```
