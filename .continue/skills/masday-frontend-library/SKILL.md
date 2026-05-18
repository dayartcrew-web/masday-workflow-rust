---
name: masday-frontend-library
description: Read design references from visual screenshots (.png/.jpg) and markdown design.md files, extract design tokens, then build pixel-accurate frontend components matching the design system.
disable-model-invocation: false
allowed-tools: Read Write Edit Bash Glob Grep Agent
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

defaults path: `~/library-design-md/*/*.md` (search for any design.md file in the library-design-md directory tree)
Example path: `~/library-design-md/dashboard/Agent-Memory-Llm-01/agent_memory_llm_dashboard_design_md.md`

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

defaults path: `~/library-design-md/*/DESIGN-*.md` (search for any design.md file in the library-design-md directory tree)
Example path: `~/library-design-md/DESIGN-e1s5h4v3dhn1-d-space-z-ai.md`

### Format 3: Visual Screenshot (.png/.jpg/.jpeg)

A design mockup screenshot that must be analyzed visually to extract:
- Layout structure and grid
- Color palette (background, text, accent colors)
- Typography (font sizes, weights, hierarchy)
- Component shapes (border radius, shadows)
- Spacing patterns
- Navigation structure

defaults path: `~/library-design-md/*/*.{png,jpg,jpeg}` (search for any image file in the library-design-md directory tree)
Example path: `~/library-design-md/dashboard/Agent-Memory-Llm-01/agent_memory_llm_dashboard_design.png`

## Steps

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

4. **If no path provided**, scan the `library-design-md/` directory tree and list available designs for the user to choose from.

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

Before building, confirm with the user:

1. **Component scope** — Which component(s) to build?
2. **Framework** — React, Vue, Svelte, or plain HTML/CSS?
3. **Styling approach** — Tailwind CSS, CSS Modules, Styled Components, or vanilla CSS?
4. **State management** — If needed, which solution?
5. **Chart library** — If charts are required, which one?

If the design.md includes a "Suggested Tech Stack" section, present those as the recommended defaults.

### Phase 4: Build Components

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
library-design-md/
  <category>/
    <Design-Name>/
      <design-name>_design.png
      <design-name>_design_md.md

DESIGN-<site-id>.md
```

When the user says "build from design X", search both locations.
