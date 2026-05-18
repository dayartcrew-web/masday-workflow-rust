Browse style library, review UI, or implement design patterns.

## Usage

```
/msd-ui-ux                    # Browse available styles and select one
/msd-ui-ux review             # Review current project UI against a style
/msd-ui-ux implement          # Implement selected style in current project
/msd-ui-ux [style-name]       # Load a specific style (e.g., /msd-ui-ux wise)
```

## Modes

### Browse (default, no arguments)

List all styles in the library and help the user select one:

1. Use Glob to discover styles: `.claude/skills/msd-ui-ux-expert/library/*/design.md`
2. For each style, read design.md and summarize its visual identity
3. If screenshots exist (light.jpeg, dark.jpeg), read and describe them
4. Present options to the user with descriptions
5. Let user select a style or request more details

### Review (`review` argument)

Audit existing UI against a library style:

1. If no style specified, ask which style to review against
2. Load the msd-ui-ux-expert skill
3. Follow the UI Review Process from the skill:
   - Load design tokens from selected style
   - Scan existing project code (Glob for UI files)
   - Compare colors, typography, spacing, components against tokens
   - Generate audit report with scored findings
4. Present the audit report with prioritized recommendations

### Implement (`implement` argument)

Apply a library style to the current project:

1. If no style specified, ask which style to implement
2. Load the msd-ui-ux-expert skill
3. Follow the Implementation Workflow:
   - Detect project framework (React, Vue, Svelte, Angular)
   - Apply design tokens to the project's theming system
   - Create/update components matching the style
   - Verify responsive breakpoints (640, 768, 1024, 1280px)
   - Verify accessibility (WCAG 2.1 AA)
4. Save progress notes for continuity

### Style Load (`[style-name]` argument)

Load and display a specific style from the library:

1. Read `.claude/skills/msd-ui-ux-expert/library/{style-name}/design.md`
2. Read screenshots if available (light.jpeg, dark.jpeg)
3. Display the full design token reference
4. Suggest next steps: review or implement with this style

## Argument Parsing

```
$ARGUMENTS = ""           → Browse mode (default)
$ARGUMENTS = "review"     → Review mode
$ARGUMENTS = "implement"  → Implement mode
$ARGUMENTS = "wise"       → Load style "wise"
$ARGUMENTS = "wise review"     → Review against style "wise"
$ARGUMENTS = "wise implement"  → Implement style "wise"
```

## Pre-conditions

- [ ] Style library exists at `.claude/skills/msd-ui-ux-expert/library/`
- [ ] At least one style has a design.md file

If library is empty → report "No styles found. Add a style folder to .claude/skills/msd-ui-ux-expert/library/{name}/ with design.md"

## Integration

This command invokes the `msd-ui-ux-expert` skill and uses the `msd-ui-ux-expert` agent for complex review/implementation tasks.
