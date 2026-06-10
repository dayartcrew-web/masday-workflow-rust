---
name: masday-visual-frontend
description: >
  Unified frontend development with browser agent, visual analysis, design token generation,
  and component building. Use when: reverse-engineering a live website's design, building
  pixel-accurate components from screenshots, extracting design tokens from URLs/images/docs,
  or visually verifying built components against design references.
disable-model-invocation: false
allowed-tools: >
  Read Write Edit Bash Glob Grep Agent
  mcp__plugin_playwright_playwright__browser_navigate
  mcp__plugin_playwright_playwright__browser_snapshot
  mcp__plugin_playwright_playwright__browser_take_screenshot
  mcp__plugin_playwright_playwright__browser_click
  mcp__plugin_playwright_playwright__browser_type
  mcp__plugin_playwright_playwright__browser_evaluate
  mcp__plugin_playwright_playwright__browser_fill_form
  mcp__plugin_playwright_playwright__browser_wait_for
  mcp__plugin_playwright_playwright__browser_press_key
  mcp__plugin_playwright_playwright__browser_network_requests
  mcp__plugin_playwright_playwright__browser_console_messages
  mcp__plugin_playwright_playwright__browser_tabs
  mcp__plugin_playwright_playwright__browser_resize
  mcp__4_5v_mcp__analyze_image
  mcp__web_reader__webReader
  mcp__masday__memory_store
  mcp__masday__memory_search
  mcp__masday__workflow_saveProgress
  mcp__masday__review_submit
  mcp__masday__policy_validate_completion
  mcp__masday__workflow_completeTask
  mcp__masday__local_sync
  mcp__masday__local_init
  mcp__masday__semantic-search_code_search
  filesystem_write filesystem_read filesystem_list
context: fork
---

# Masday Visual Frontend

Unified frontend development with **browser agent**, **visual analysis**, **design token generation**, and **component building** — all in one pipeline.

## Quick Start

> No input detected. Choose a mode:

| Usage | Example |
|-------|---------|
| **URL** | `/masday-visual-frontend https://stripe.com` |
| **Screenshot** | `/masday-visual-frontend dashboard-design.png` |
| **Design Doc** | `/masday-visual-frontend design.md` |
| **Component** | `/masday-visual-frontend "Build a pricing card with 3 tiers"` |

Paste a URL, screenshot path, design doc, or component description to get started.

## 4 Input Modes

| Mode | Input | What Happens |
|------|-------|-------------|
| **URL** | Live website URL | Navigate → Screenshot → Extract CSS vars → Generate tokens → Build components |
| **Screenshot** | PNG/JPG file | AI Vision analysis → Extract tokens → Build components |
| **Design Doc** | design.md file | Parse structured tokens → Build components |
| **Component** | Text description | Use existing project tokens → Build component |

**Auto-detection logic:**
- Input starts with `http://` or `https://` → **URL mode**
- Input ends with `.png`, `.jpg`, `.jpeg` → **Screenshot mode**
- Input ends with `.md` or is a directory → **Design Doc mode**
- Otherwise → **Component mode** (build from description)

## Pipeline (7 Phases)

This skill enforces **mandatory step completion**. Each step must be completed before proceeding. Do not skip steps.

### Phase 1: Input Detection & Loading

**Auto-detect the input mode** from the user's argument:

**URL Mode:**
```
1. browser_navigate({ url: INPUT_URL })
2. browser_take_screenshot() — capture full-page screenshot
3. browser_snapshot() — capture accessibility tree / DOM structure
4. browser_evaluate({ script: "document.title" }) — get page title for reference
→ Store: screenshot, snapshot, page title
```

**Screenshot Mode:**
```
1. Read the image file (Claude is multimodal — can analyze PNG/JPG directly)
2. Extract visual properties from the image:
   - Layout grid structure (columns, rows, sections)
   - Color palette (identify background, surface, primary, text, accent colors)
   - Typography (heading sizes, weights, body text size)
   - Component shapes (border radius, shadows, card styles)
   - Spacing patterns (padding, margins, gaps)
→ Store: visual analysis notes
```

**Design Doc Mode:**
```
1. If input is a directory → scan for .md and image files, load all
2. If input is a .md file → Read the full content
3. Identify format:
   - Comprehensive design doc → has sections for colors, typography, layout, components
   - Auto-extracted design system → has CSS Custom Properties section
   - If images found alongside .md → load them as supplementary references
→ Store: parsed design document
```

**Component Mode:**
```
1. Use the text description as the component specification
2. Search for existing design tokens in the project:
   - Grep for CSS custom properties (--color, --spacing, --font)
   - Grep for tailwind.config (theme.extend)
   - Grep for theme files (theme.ts, tokens.ts, design-tokens.*)
3. If tokens found → load them as the design baseline
4. If no tokens found → proceed with component description only
→ Store: component spec + existing tokens
```

**GATE 1**: Source loaded and validated. Must have either: screenshot image, parsed document, DOM snapshot, or component specification.

---

### Phase 2: Visual Analysis & Token Extraction

Generate a structured **DesignTokens** object from the loaded source.

#### URL Mode — Live Site CSS Extraction

Use `browser_evaluate` to extract design tokens directly from the live website:

```javascript
// Extract CSS custom properties from :root
() => {
  const cssVars = {};
  for (const sheet of document.styleSheets) {
    try {
      for (const rule of sheet.cssRules) {
        if (rule.selectorText === ':root' || rule.selectorText === 'html') {
          for (const prop of rule.style) {
            if (prop.startsWith('--')) {
              cssVars[prop] = rule.style.getPropertyValue(prop).trim();
            }
          }
        }
      }
    } catch(e) { /* cross-origin sheets */ }
  }
  return JSON.stringify(cssVars, null, 2);
}
```

```javascript
// Extract computed styles from key elements
() => {
  const elements = {
    body: document.body,
    h1: document.querySelector('h1'),
    h2: document.querySelector('h2'),
    button: document.querySelector('button, [role="button"], a.btn, a.button'),
    card: document.querySelector('[class*="card"], [class*="Card"]'),
    input: document.querySelector('input, textarea, select'),
    nav: document.querySelector('nav, [role="navigation"]'),
    link: document.querySelector('a')
  };
  const styles = {};
  for (const [name, el] of Object.entries(elements)) {
    if (!el) continue;
    const cs = getComputedStyle(el);
    styles[name] = {
      color: cs.color,
      backgroundColor: cs.backgroundColor,
      fontFamily: cs.fontFamily,
      fontSize: cs.fontSize,
      fontWeight: cs.fontWeight,
      lineHeight: cs.lineHeight,
      borderRadius: cs.borderRadius,
      boxShadow: cs.boxShadow,
      padding: cs.padding,
      margin: cs.margin
    };
  }
  return JSON.stringify(styles, null, 2);
}
```

```javascript
// Extract color palette from page
() => {
  const allElements = document.querySelectorAll('*');
  const colors = new Set();
  const bgColors = new Set();
  allElements.forEach(el => {
    const cs = getComputedStyle(el);
    if (cs.color && cs.color !== 'rgba(0, 0, 0, 0)') colors.add(cs.color);
    if (cs.backgroundColor && cs.backgroundColor !== 'rgba(0, 0, 0, 0)') bgColors.add(cs.backgroundColor);
  });
  return JSON.stringify({
    textColors: [...colors].slice(0, 20),
    backgroundColors: [...bgColors].slice(0, 20),
    totalTextColors: colors.size,
    totalBgColors: bgColors.size
  }, null, 2);
}
```

After extraction, map raw CSS values into the DesignTokens structure.

#### Screenshot Mode — AI Vision Analysis

Analyze the screenshot and extract these properties:

1. **Colors** — Identify hex values for:
   - Background (page bg)
   - Surface (cards, panels)
   - Primary (buttons, links, accents)
   - Text primary and secondary
   - Border/divider color
   - Success, warning, error (if present)

2. **Typography** — Identify:
   - Heading font family (from visual appearance)
   - Body font family
   - Heading scale (h1 through h4 sizes, in px → rem conversion)
   - Font weights used

3. **Spacing** — Estimate:
   - Base spacing unit (typically 4px or 8px)
   - Padding patterns for cards, sections, containers
   - Gap between elements

4. **Radius** — Identify:
   - Border radius on buttons, cards, inputs
   - Sharp vs rounded corners

5. **Shadows** — Identify:
   - Shadow levels (none, subtle, medium, prominent)
   - Usage context (cards, modals, dropdowns)

6. **Layout** — Identify:
   - Grid structure (columns, gutters)
   - Sidebar width (if present)
   - Content max-width
   - Responsive behavior hints

7. **Components** — Catalog visible:
   - Buttons (variants: primary, secondary, ghost)
   - Cards (content, stats, media)
   - Form inputs
   - Navigation (sidebar, topbar, tabs)
   - Tables, lists, grids

#### Design Doc Mode — Parse Markdown

Read the design.md and extract tokens from structured sections:
- **Color Palette section** → parse hex values and semantic roles
- **Typography section** → parse font families, sizes, weights
- **Spacing section** → parse spacing scale
- **Components section** → parse component specifications
- **CSS Custom Properties section** → directly use as token source (highest priority)

#### Component Mode — Use Existing Tokens

Search the project for existing design tokens:
```
1. Grep for tailwind.config.ts/js → read theme.extend section
2. Grep for *.css files with :root → extract CSS variables
3. Grep for tokens.ts/theme.ts → read exported token objects
4. If found → use as DesignTokens baseline
5. If not found → generate minimal tokens from component description
```

**Output — DesignTokens Structure:**

```
DesignTokens {
  // Source metadata
  source: {
    mode: "url" | "screenshot" | "design-doc" | "component",
    input: "<original input>",
    extractedAt: "<timestamp>"
  }

  // Core tokens
  colors: {
    background: string     // Page background
    surface: string        // Cards, panels
    surfaceElevated: string // Modals, popovers
    primary: string        // Main accent
    secondary: string      // Secondary accent
    textPrimary: string    // Main text
    textSecondary: string  // Subdued text
    border: string         // Borders, dividers
    success: string        // Positive states
    warning: string        // Warning states
    error: string          // Error states
    custom: Record<string, string>  // Additional colors
  }

  typography: {
    headingFont: string
    bodyFont: string
    headingWeight: number
    bodyWeight: number
    scale: Array<{
      token: string        // e.g., "heading-xl", "body-sm"
      size: string         // e.g., "2rem", "0.875rem"
      weight: number
      lineHeight: string
      usage: string        // e.g., "Page titles", "Body text"
    }>
  }

  spacing: {
    base: string           // e.g., "0.25rem" or "4px"
    scale: Array<{
      token: string        // e.g., "xs", "sm", "md", "lg", "xl"
      value: string        // e.g., "0.25rem", "0.5rem", "1rem"
    }>
  }

  radius: {
    scale: Array<{
      token: string        // e.g., "sm", "md", "lg", "full"
      value: string        // e.g., "0.25rem", "0.5rem", "9999px"
    }>
  }

  shadows: {
    levels: Array<{
      name: string         // e.g., "sm", "md", "lg"
      value: string        // CSS box-shadow value
      usage: string        // e.g., "Cards", "Modals"
    }>
  }

  components: {
    cards: string          // Card styling description
    buttons: string        // Button styling description
    inputs: string         // Input styling description
    navigation: string     // Nav styling description
  }

  layout: {
    grid: string           // Grid description
    sidebar: string        // Sidebar width/behavior
    breakpoints: Array<{
      name: string         // e.g., "sm", "md", "lg", "xl"
      width: string        // e.g., "640px", "768px", "1024px", "1280px"
      notes: string
    }>
  }

  responsive: {
    mobile: string         // Mobile layout description
    tablet: string         // Tablet layout description
    desktop: string        // Desktop layout description
  }
}
```

**GATE 2**: DesignTokens validated. Must have at minimum: `colors` (background + primary + textPrimary) and `typography` (headingFont + bodyFont + at least 2 scale entries).

---

### Phase 3: Token Storage

Persist extracted tokens for reuse across sessions.

```
1. Save to library:
   Path: .claude/skills/masday-visual-frontend/library/<source-slug>.md

   URL mode:    library/DESIGN-<domain>-<timestamp>.md
   Screenshot:  library/<category>/<design-name>_tokens.md
   Design doc:  library/<original-path>-tokens.md
   Component:   Skip library save (uses existing project tokens)

2. Store in masday memory:
   memory_store({
     memory_type: "artifact",
     summary: "Design tokens extracted from <source>",
     content: "<DesignTokens JSON>",
     tags: "design-tokens,<source-mode>,<framework>",
     created_by_agent: "masday-visual-frontend"
   })
```

**GATE 3**: Tokens persisted to at least one storage location.

---

### Phase 4: Requirement Clarification

**Auto-detect first** before asking the user:

```
1. Read package.json (root and any apps/ packages):
   - Detect: react, vue, svelte, angular, next, nuxt, solid
   - Detect: tailwindcss, styled-components, @emotion/react, css-modules
   - Detect: state management (zustand, redux, jotai, pinia)

2. Read existing project structure:
   - Component directory pattern (src/components, components/, lib/)
   - Styling pattern (*.module.css, *.styled.ts, tailwind classes)
   - Test file pattern (*.test.*, *.spec.*)
```

**Present detected stack and confirm with user:**

1. **Component scope** — Which component(s) to build?
2. **Framework** — React / Vue / Svelte / Angular / HTML? (auto-detected as default)
3. **Styling approach** — Tailwind / CSS Modules / Styled Components / Vanilla CSS? (auto-detected as default)
4. **State management** — If needed, which?
5. **Chart library** — If charts required, which?

If auto-detection found clear patterns, present them as defaults and ask for confirmation only.

**GATE 4**: Requirements confirmed — component scope, framework, styling approach locked in.

---

### Phase 5: Component Building

**Delegate to `masday-frontend` agent** for implementation:

```
Agent({
  subagent_type: "masday-frontend",
  prompt: "Build component(s) for workflow {workflowId}.

    DESIGN TOKENS (source of truth — use these, NEVER hardcode values):
    {DesignTokens JSON}

    REQUIREMENTS:
    - Component scope: {scope}
    - Framework: {framework}
    - Styling: {styling approach}
    - Working directory: {cwd}

    RULES:
    - Reference tokens for ALL colors, spacing, typography, shadows, radius
    - If using Tailwind: map tokens to tailwind.config.ts theme.extend
    - If using CSS vars: define in :root
    - TypeScript with proper types (no any)
    - WCAG 2.1 AA accessibility
    - Responsive breakpoints from tokens
    - Co-located test file
    - Under 200 lines per component

    Build order:
    1. Design token file (theme/constants)
    2. Layout shell
    3. Components (priority: nav → cards → charts → tables → widgets)
    4. Responsive breakpoints
    5. Interaction states"
})
```

**GATE 5**: Components built. Agent must report:
- Files created/modified
- Build status (pass/fail)
- Test results (pass/fail)

---

### Phase 6: Visual Verification

**Start dev server and verify with Playwright:**

```
1. Start dev server:
   Bash: cd {project} && npm run dev (or pnpm dev)
   Wait for "ready" or "compiled" in output

2. Navigate to component page:
   browser_navigate({ url: "http://localhost:<port>/<route>" })

3. Capture baseline screenshot:
   browser_take_screenshot() — full desktop view

4. Responsive verification at 3 breakpoints:
   browser_resize({ width: 375, height: 667 })   // Mobile
   browser_take_screenshot()

   browser_resize({ width: 768, height: 1024 })   // Tablet
   browser_take_screenshot()

   browser_resize({ width: 1440, height: 900 })   // Desktop
   browser_take_screenshot()

5. Console health check:
   browser_console_messages({ level: "error" })
   — Must be 0 errors. If errors found → report and flag.

6. Network health check:
   browser_network_requests({ static: false })
   — Must have 0 failed requests (4xx/5xx). If failures → report.

7. Fidelity assessment:
   Compare built screenshots against original design reference.
   Rate: EXACT | MINOR_DIFF | MODERATE_DIFF | MAJOR_DIFF

   Criteria:
   - Colors match tokens (hex comparison)
   - Typography matches scale
   - Spacing follows token system
   - Layout matches grid structure
   - Components render correctly at all breakpoints
   - No visual regressions
```

**If fidelity is MAJOR_DIFF:**
- List specific mismatches
- Suggest fixes
- Ask user whether to rework or accept

**GATE 6**: Visual fidelity verified. Rating must be EXACT or MINOR_DIFF to proceed.

---

### Phase 7: Review Pipeline

Complete the masday review pipeline:

```
STEP 7.1: Save progress
  workflow_saveProgress({
    workflow_id: "<workflowId>",
    task_id: "<taskId>",
    agent_name: "masday-visual-frontend",
    progress_note: "<summary: tokens extracted, components built, fidelity rating>",
    evidence: ["<files created>", "<screenshots captured>", "<test results>"]
  })

STEP 7.2: Submit for review
  review_submit({
    workflow_id: "<workflowId>",
    task_id: "<taskId>",
    reviewer_agent: "masday-reviewer",
    decision: "<APPROVED | REWORK_REQUIRED | BLOCKED>",
    notes: "<what was done, fidelity rating, key decisions>",
    gaps: ["<any gaps found>"]
  })

STEP 7.3: If REWORK_REQUIRED — fix and loop
  - Fix the gaps identified in the review
  - Re-verify visually (Phase 6)
  - Re-save progress (workflow_saveProgress)
  - Re-submit review (review_submit)
  - Max 2 rework attempts, then STOP

STEP 7.4: If APPROVED — validate completion
  policy_validate_completion({
    workflow_id: "<workflowId>",
    task_id: "<taskId>"
  })

STEP 7.5: Complete task
  workflow_completeTask({
    workflow_id: "<workflowId>",
    task_id: "<taskId>",
    result: "<completion summary>"
  })

STEP 7.6: Sync local state
  local_sync({ cwd: process.cwd(), workflow_id: "<workflowId>" })
```

---

## Output Format

```
## Visual Frontend Report

### Input
- Mode: <URL | Screenshot | Design Doc | Component>
- Source: <file path or URL>
- Page title: <if URL mode>

### Design Tokens Extracted
- Colors: <count> (background, surface, primary, secondary, text, border, success, warning, error)
- Typography: <count> sizes (headingFont: <name>, bodyFont: <name>)
- Spacing: <count> scale values (base: <value>)
- Radius: <count> levels
- Shadows: <count> levels
- Breakpoints: <count> (mobile, tablet, desktop)

### Components Built
- <component-name> — <description> (<lines> lines)
- <component-name> — <description> (<lines> lines)

### Files Created
- <file-path> — <purpose>

### Visual Verification
- Desktop (1440x900): <screenshot path>
- Tablet (768x1024): <screenshot path>
- Mobile (375x667): <screenshot path>
- Console errors: <count>
- Failed network requests: <count>
- Fidelity rating: <EXACT | MINOR_DIFF | MODERATE_DIFF | MAJOR_DIFF>

### Token Storage
- Library: <file path>
- Memory: <memory id>
```

## Error Handling

| Phase | Error | Action |
|-------|-------|--------|
| Phase 1 | URL unreachable | Ask for alternative URL or switch to Screenshot mode |
| Phase 1 | File not found | List available files in library/, ask user to select |
| Phase 1 | Image too small/blurry | Warn user, proceed with lower confidence |
| Phase 2 | CSS extraction blocked (CORS) | Fall back to screenshot analysis + computed styles |
| Phase 2 | Insufficient visual data | Ask user for supplementary screenshots or specs |
| Phase 2 | Token validation fails | Report missing tokens, ask user to supplement |
| Phase 4 | Framework detection ambiguous | Ask user to confirm |
| Phase 5 | Build fails | Parse error, suggest fix, re-dispatch agent |
| Phase 5 | Tests fail | Parse failure, fix component, re-run |
| Phase 6 | Dev server won't start | Check port conflicts, suggest manual start |
| Phase 6 | Console errors found | Report errors, check if blocking or cosmetic |
| Phase 6 | MAJOR_DIFF fidelity | List mismatches, ask user: rework or accept |
| Phase 7 | REWORK_REQUIRED | Fix gaps, re-verify, max 2 attempts |

## Rules

### Design Token Discipline
- **NEVER hardcode** colors, spacing, font values — always reference tokens or CSS custom properties
- If a token is missing, derive from closest pattern and flag it for review
- Hex values must be exact matches from extraction
- CSS variable names must follow project convention

### Browser Automation
- ALWAYS take a screenshot BEFORE and AFTER interactions for debugging
- Use `browser_wait_for` for dynamic content — never fixed `sleep()`
- Check console errors after every page navigation
- Close browser tabs when done to avoid resource leaks

### Component Quality
- TypeScript with proper types, no `any`
- Self-contained and reusable components
- Under 200 lines per component
- WCAG 2.1 AA accessibility (44x44px touch targets, contrast ratios, keyboard nav)

### Visual Fidelity
- Match the design reference as closely as possible
- Image (screenshot) wins over markdown on conflicts
- Ask rather than guess on ambiguity
- No invented UI patterns

## Library Path Convention

```
.claude/skills/masday-visual-frontend/library/
  DESIGN-<domain>-<timestamp>.md          # URL mode extractions
  <category>/
    <design-name>_tokens.md               # Screenshot mode extractions
    <design-name>_screenshot.png           # Reference screenshots
```

## Mandatory Review Pipeline

When this skill completes work on a workflow task, it MUST follow this pipeline:

`
STEP 1: Save progress to SQLite
  workflow_saveProgress({
    workflow_id: "<workflowId>",
    task_id: "<taskId>",
    agent_name: "masday-visual-frontend",
    progress_note: "<summary of work done>",
    evidence: ["<files modified>", "<screenshots captured>", "<test results>"]
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
- Never skip any step — complete each step before proceeding
- Never bypass a GATE marker without validating prior steps
- Never claim completion without executing all steps in order
- Never call workflow_completeTask without review_submit (APPROVED)
- Never skip policy_validate_completion before completion
- Never skip local_sync after completing a task
- Never claim done without saving progress to SQLite
- Never hardcode colors, spacing, or font values
- Never skip visual verification (Phase 6) after building components
