---
name: masday-e2e
description: Test wired frontend and backend end-to-end using Playwright MCP browser tools — auto-detects project config, navigates, snapshots, asserts
disable-model-invocation: false
allowed-tools: Bash Read Write Edit Grep Glob Agent mcp__plugin_playwright_playwright__browser_navigate mcp__plugin_playwright_playwright__browser_snapshot mcp__plugin_playwright_playwright__browser_click mcp__plugin_playwright_playwright__browser_type mcp__plugin_playwright_playwright__browser_take_screenshot mcp__plugin_playwright_playwright__browser_evaluate mcp__plugin_playwright_playwright__browser_fill_form mcp__plugin_playwright_playwright__browser_wait_for mcp__plugin_playwright_playwright__browser_press_key mcp__plugin_playwright_playwright__browser_network_requests mcp__plugin_playwright_playwright__browser_console_messages mcp__plugin_playwright_playwright__browser_tabs mcp__plugin_playwright_playwright__browser_resize memory.store workflow.saveProgress tests.run
context: inline
---

# Masday E2E Testing with Playwright MCP

Test wired frontend-backend flows using Playwright MCP browser tools. Auto-detects service URLs and page routes from project config.

## Step 0: Auto-Detect Project Config

Before any testing, discover the project's service URLs and pages:

**1. Find frontend and backend URLs:**

```
Search order (use first match):
1. playwright.config.ts / playwright.config.js — read baseURL values from each project
2. package.json scripts — look for "dev", "start" scripts with port hints
3. .env / .env.local — look for PORT, NEXT_PUBLIC_API_URL, API_URL, etc.
4. vite.config.ts / next.config.js — look for port config
5. If none found: ask user for frontend and backend URLs
```

**2. Discover page routes:**

```
For Next.js: Glob **/app/**/page.tsx (or page.jsx)
For React Router: Grep <Route path= in src/
For Nuxt: Glob **/pages/**/*.vue
For SvelteKit: Glob src/routes/**/+page.svelte
```

Extract route paths from file structure. Group by feature area.

**3. Store detected config for the session:**

```
FRONTEND_URL = detected frontend base URL (e.g. http://localhost:3000)
BACKEND_URL = detected backend base URL (e.g. http://localhost:3001)
PAGES = list of discovered routes with descriptions
```

**4. Verify services are running:**

```bash
curl -sf "$FRONTEND_URL" && echo "Frontend: UP" || echo "Frontend: DOWN"
curl -sf "$BACKEND_URL/health" 2>/dev/null || curl -sf "$BACKEND_URL/api/health" 2>/dev/null || echo "Backend health: check manually"
```

If services are down, report which are down and suggest start commands from package.json scripts. Do NOT proceed until services are up.

## Testing Flow

### Phase 1: Smoke Test

1. `browser_navigate({ url: FRONTEND_URL })`
2. `browser_snapshot()` — verify page loads, no crash/error state
3. `browser_navigate({ url: BACKEND_URL + "/health" })` (or `/api/health`)
4. Verify API responds with valid JSON

### Phase 2: Page-by-Page Wire Testing

For each discovered page, test that:
- Page renders without JS errors
- API calls succeed (network requests return 2xx)
- Data flows from backend to UI elements

**Pattern for each page:**

```
1. browser_navigate({ url: FRONTEND_URL + "/<route>" })
2. browser_snapshot() — capture page state
3. browser_console_messages({ level: "error" }) — check for JS errors
4. browser_network_requests({ static: false }) — verify API calls
5. If API calls failed: investigate with browser_network_request({ index: N })
```

**Test order by priority (auto-sorted):**

1. Auth/login pages — gate for everything else
2. List/index pages — core data-fetching pages
3. Create/new pages — test POST wiring
4. Detail/[id] pages — test parameterized routes
5. Settings/config pages — test CRUD operations
6. Dashboard/analytics pages — test aggregated data

### Phase 3: Interactive Flow Testing

Test complete user flows that cross frontend-backend boundary.

**Generic form submission flow:**
```
1. Navigate to any form page (create/new routes)
2. browser_fill_form with test data
3. Submit (click button or press Enter)
4. browser_wait_for success indicator (redirect, toast, status change)
5. Verify result appears (list page, detail page, or API response)
```

**Login/auth flow (if auth page exists):**
```
1. Navigate to login route
2. Fill credentials
3. Submit
4. Wait for redirect to home/dashboard
5. Verify authenticated state (token in storage, user menu visible)
```

**CRUD flow (for any entity with list + create pages):**
```
1. Navigate to list page — verify items load
2. Navigate to create page
3. Fill form — submit
4. Verify redirect to detail or list with new item
5. Verify via network request that backend stored the data
```

### Phase 4: Responsive Testing

```
1. browser_resize({ width: 375, height: 667 })  — mobile
2. Test primary flow
3. browser_resize({ width: 768, height: 1024 })  — tablet
4. Test primary flow
5. browser_resize({ width: 1440, height: 900 })  — desktop
6. Test primary flow
```

### Phase 5: Console & Network Health Check

After all pages tested:

```
1. browser_console_messages({ level: "error", all: true })
   — Report any errors found across all pages

2. Check for failed network requests across pages
   — Report any 4xx/5xx responses
```

## Assertion Patterns

Use `browser_evaluate` to assert DOM state. Adapt selectors to the project's patterns:

```javascript
// Generic checks — adapt data-testid to project conventions
() => document.querySelector('[data-testid="list"], [role="list"], table, ul') !== null
() => document.querySelector('h1') !== null
() => !document.querySelector('[data-testid="error"], .error-boundary, [role="alert"]')
() => document.querySelectorAll('a[href]').length > 0  // navigation links exist
```

First, discover the project's test ID convention:
```
Grep for: data-testid, data-cy, data-test, test-id, aria-label
Use the project's convention in assertions.
```

## Error Handling

| Error | Action |
|-------|--------|
| Config not found | Ask user for frontend/backend URLs |
| Service not running | Report which is down, suggest start command from package.json |
| Console errors | Capture full error, identify source file |
| Network 404 | Check if API route exists, compare with backend routes |
| Network 500 | Check backend logs, may be DB/connection issue |
| Timeout | Increase wait, check if page has loading state |
| Auth required | Test login flow first, then retry |

## Report Format

After testing, output:

```
## E2E Test Report

### Project Config (auto-detected)
- Frontend: {FRONTEND_URL}
- Backend: {BACKEND_URL}
- Pages discovered: {count}
- Test ID convention: {data-testid / data-cy / other}

### Services
- Frontend: UP/DOWN
- Backend: UP/DOWN

### Pages Tested: X/Y
| Page | Route | Status | Errors | Notes |
|------|-------|--------|--------|-------|

### Flows Tested
- Auth: PASS/FAIL — details
- CRUD: PASS/FAIL — details
- Navigation: PASS/FAIL — details

### Console Errors: N
### Failed Network Requests: N

### Recommendations
- fix suggestions
```

## Rules

- ALWAYS run Step 0 (auto-detect) first — never hardcode URLs or routes
- Take snapshots before and after interactions for debugging
- Check console errors after every page navigation
- Report all failures — don't skip pages that error out
- Use `browser_wait_for` for dynamic content instead of fixed sleeps
- Capture screenshots of failures for visual debugging
- Adapt assertion selectors to the project's convention (data-testid, data-cy, etc.)
- Store E2E results via `memory.store` (type: "artifact") for cross-session recall
- Save progress via `workflow.saveProgress` after each testing phase completes
