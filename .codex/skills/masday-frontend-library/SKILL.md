---
name: masday-frontend-library
description: Read design references from visual screenshots (.png/.jpg) and markdown design.md files, extract design tokens, then build pixel-accurate frontend components matching the design system.
disable-model-invocation: false
allowed-tools: Read Write Edit Bash Glob Grep Agent memory_store filesystem_write
context: fork
---

# Masday Frontend Library

Build pixel-accurate frontend components from design references — both **visual screenshots** (PNG/JPG) and **markdown design specifications** (design.md).

## Supported Design Reference Formats

This skill handles two distinct design reference formats:

### Format 1: Comprehensive Design Document (.md)

A manually-authored design system document with structured sections covering:
- Overview & design philosophy
- Color system (palette table with hex values and roles)
- Typography system (font families, heading scale, weights)
- Layout structure (ASCII grid diagrams)
- Component specifications (sidebar, KPI cards, charts, graphs, tables)
- UI component language (card design, glow effects, input styles)
- Interaction system (hover states, motion language)
- Responsive design breakpoints
- Accessibility guidelines
- Tech stack recommendations
- Component architecture tree

defaults path: `.claude/skills/masday-frontend-library/library-design-md/*/*.md` (search for any design.md file in the library-design-md directory tree)
Example path: `.claude/skills/masday-frontend-library/library-design-md/dashboard/Agent-Memory-Llm-01/agent_memory_llm_dashboard_design_md.md`

### Format 2: Auto-Extracted Design System (.md)

A tool-extracted design system from a live website, with sections like:
- Visual Theme & Atmosphere
- Color Palette & Roles (with CSS variable names)
- Typography Rules (font families, type hierarchy table)
- Component Stylings (extracted CSS for buttons, cards, etc.)
- Layout Principles (spacing scale, border radius)
- Depth & Elevation (shadow levels)
- Do's and Don'ts
- Responsive Behavior
- Agent Prompt Guide
- CSS Custom Properties (extracted `:root` variables)

Explore the CSS Custom Properties section for a comprehensive list of design tokens that can be directly used in development.

defaults path: `.claude/skills/masday-frontend-library/library-design-md/*/DESIGN-*.md` (search for any design.md file in the library-design-md directory tree)
Example path: `.claude/skills/masday-frontend-library/library-design-md/DESIGN-e1s5h4v3dhn1-d-space-z-ai.md`

### Format 3: Visual Screenshot (.png/.jpg/.jpeg)

A design mockup screenshot that must be analyzed visually to extract:
- Layout structure and grid
- Color palette (background, text, accent colors)
- Typography (font sizes, weights, hierarchy)
- Component shapes (border radius, shadows)
- Spacing patterns
- Navigation structure

defaults path: `.claude/skills/masday-frontend-library/library-design-md/*/*.{png,jpg,jpeg}` (search for any image file in the library-design-md directory tree)
Example path: `.claude/skills/masday-frontend-library/library-design-md/dashboard/Agent-Memory-Llm-01/agent_memory_llm_dashboard_design.png`

## Steps

This skill enforces **mandatory step completion**. Each step must be completed before proceeding. Do not skip steps.


### Phase 1: Load Design Reference

1. **Identify the design reference** — the user provides a file path (or directory containing design files).

2. **Detect the format**:
   - `.md` file → read as markdown design spec
   - `.png`/`.jpg`/`.jpeg` file → analyze as visual screenshot
   - Directory → scan for both `.md` and image files, load all found

3. **Read the reference**:
   - For `.md` files: use the Read tool to load the full content
   - For image files: use the Read tool (Claude is multimodal and can analyze images)
   - For directories: load all `.md` and image files found

4. **If no path provided**, scan the `.claude/skills/masday-frontend-library/library-design-md/` directory tree and list available designs for the user to choose from.

### Phase 2: Extract Design Tokens

Parse the loaded reference and extract into a structured design token object:

```
DesignTokens {
  colors: {
    background: string
    surface: string
    surfaceElevated: string
    primary: string
    secondary: string
    textPrimary: string
    textSecondary: string
    border: string
    success: string
    warning: string
    error: string
    custom: Record<string, string>
  }
  typography: {
    headingFont: string
    bodyFont: string
    headingWeight: number
    scale: { token: string, size: string, weight: number, usage: string }[]
  }
  spacing: {
    base: string
    scale: { token: string, value: string }[]
  }
  radius: {
    scale: { token: string, value: string }[]
  }
  shadows: {
    levels: { name: string, value: string, usage: string }[]
  }
  components: {
    cards: string
    buttons: string
    inputs: string
    navigation: string
  }
  layout: {
    grid: string
    sidebar: string
    breakpoints: { name: string, width: string, notes: string }[]
  }
  responsive: {
    mobile: string
    tablet: string
    desktop: string
  }
}
```

For auto-extracted format (Format 2), prioritize the CSS Custom Properties section as the source of truth.

For visual screenshots (Format 3), extract tokens by careful visual analysis.

### Phase 3: Clarify Requirements

**Auto-detect first**: Read `package.json` (root and any `apps/` packages) to detect the framework and styling approach already in use. Check for `react`, `vue`, `svelte`, `next`, `nuxt`, `tailwindcss`, `styled-components`, `@emotion/react`, etc. Present detected stack as the recommended defaults before asking the user.

Before building, confirm with the user:

1. **Component scope** — Which component(s) to build?
2. **Framework** — React, Vue, Svelte, or plain HTML/CSS?
3. **Styling approach** — Tailwind CSS, CSS Modules, Styled Components, or vanilla CSS?
4. **State management** — If needed, which solution?
5. **Chart library** — If charts are required, which one?

If the design.md includes a "Suggested Tech Stack" section, present those as the recommended defaults.

### Phase 4: Build Components

**Implementation delegate**: If running inside a workflow, delegate component building to the `masday-frontend` agent. Pass extracted design tokens to the agent as context so it uses them as source of truth.


**GATE**: Verify steps 1-9 are complete before proceeding.

1. **Create design token file** — Extract tokens into theme/constants.
2. **Build layout shell** — Create the main layout matching the grid structure.
3. **Build components in priority order**: Navigation, KPI cards, Charts, Graphs, Tables, Widgets, Alerts.
4. **Apply responsive breakpoints** from the design spec.
5. **Apply interaction states** matching the design's motion language.

### Phase 5: Verify Design Fidelity

1. **Visual checklist** — Background, surface, radius, typography, spacing, shadows, colors, layout, breakpoints.
2. **Token consistency** — No hardcoded values.
3. **Start dev server** — Verify visually.

## Rules

### Design Token Discipline
- **NEVER hardcode** colors, spacing, or font values — always reference tokens or CSS custom properties.
- If a token is missing, derive from closest pattern and flag it.
- Hex values must be exact.

### Component Quality
- TypeScript with proper types, no `any`.
- Self-contained and reusable components.
- Under 200 lines per component.

### Visual Fidelity
- Match the design reference as closely as possible.
- Image wins over markdown on conflicts.
- Ask rather than guess on ambiguity.
- No invented UI patterns.

### CSS Approach
- Tailwind: use `tailwind.config.ts` extended theme.
- CSS custom properties: define in `:root`.
- CSS-in-JS: create theme object matching tokens.

### Accessibility
- 44x44px minimum touch targets.
- WCAG AA contrast ratios.
- Keyboard navigation support.

## Output Format

```
## Design Reference Loaded
- Source: <file path>
- Format: <comprehensive | auto-extracted | visual screenshot>
- Design Tokens Extracted: <count> colors, <count> typography sizes, <count> spacing values

## Components Built
- <component-name> — <description>

## Files Created
- <file-path> — <purpose>

## Design Fidelity Score
- Colors: <matched/total>
- Typography: <matched/total>
- Spacing: <matched/total>
- Layout: <matched/total>
```

## Library Path Convention

```
.claude/skills/masday-frontend-library/library-design-md/
  <category>/
    <Design-Name>/
      <design-name>_design.png
      <design-name>_design_md.md

  DESIGN-<site-id>.md
```

When the user says "build from design X", search both locations.

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
- Never skip any step — complete each step before proceeding
- Never bypass a GATE marker without validating prior steps
- Never claim completion without executing all steps in order
- Never call workflow_completeTask without review_submit (APPROVED)
- Never skip policy_validate_completion before completion
- Never skip local_sync after completing a task
- Never claim done without saving progress to PostgreSQL
