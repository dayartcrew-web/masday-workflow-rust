# =============================================================================
# Multi-target Dockerfile for masday-workflow-rebuild monorepo
# Targets: base, deps, build, api, dashboard, mcp, production
# =============================================================================

# ---------------------------------------------------------------------------
# base: Node 20 slim + pnpm + workspace scaffolding
# ---------------------------------------------------------------------------
FROM node:20-slim AS base
RUN corepack enable && corepack prepare pnpm@9 --activate
WORKDIR /app

# ---------------------------------------------------------------------------
# deps: Install all dependencies (cached unless lockfile changes)
# ---------------------------------------------------------------------------
FROM base AS deps
COPY package.json pnpm-lock.yaml turbo.json ./

# Write a workspace config that excludes apps/desktop (Tauri needs native toolchain)
RUN printf 'packages:\n  - "apps/api"\n  - "apps/dashboard"\n  - "apps/agent-runner"\n  - "packages/*"\n' > pnpm-workspace.yaml

COPY packages/capability/package.json     ./packages/capability/
COPY packages/code-skills/package.json    ./packages/code-skills/
COPY packages/core/package.json           ./packages/core/
COPY packages/db/package.json             ./packages/db/
COPY packages/intelligence/package.json   ./packages/intelligence/
COPY packages/llm/package.json            ./packages/llm/
COPY packages/memory/package.json         ./packages/memory/
COPY packages/policy/package.json         ./packages/policy/
COPY packages/project-rules/package.json  ./packages/project-rules/
COPY packages/shared-utils/package.json   ./packages/shared-utils/
COPY packages/store/package.json          ./packages/store/
COPY packages/workflow-engine/package.json ./packages/workflow-engine/
COPY apps/api/package.json                ./apps/api/
COPY apps/dashboard/package.json          ./apps/dashboard/
COPY apps/agent-runner/package.json       ./apps/agent-runner/

RUN pnpm install --frozen-lockfile

# ---------------------------------------------------------------------------
# build: Full Turbo build (produces dist/ and .next/ outputs)
# Inherits node_modules from deps stage, then overlays source code
# ---------------------------------------------------------------------------
FROM deps AS build
COPY . .
RUN printf 'packages:\n  - "apps/api"\n  - "apps/dashboard"\n  - "apps/agent-runner"\n  - "packages/*"\n' > pnpm-workspace.yaml
RUN pnpm build && pnpm build:apps

# ---------------------------------------------------------------------------
# api: Hono HTTP + WebSocket server (defaults: HTTP 3000, WS 3001)
# ---------------------------------------------------------------------------
FROM node:20-slim AS api
RUN corepack enable && corepack prepare pnpm@9 --activate
WORKDIR /app

COPY --from=build /app/package.json        ./
COPY --from=build /app/pnpm-workspace.yaml ./
COPY --from=build /app/node_modules        ./node_modules
COPY --from=build /app/packages/*/dist     ./packages/*/dist
COPY --from=build /app/packages/*/package.json    ./packages/*/
COPY --from=build /app/packages/*/node_modules    ./packages/*/node_modules
COPY --from=build /app/apps/api/dist       ./apps/api/dist
COPY --from=build /app/apps/api/package.json      ./apps/api/
COPY --from=build /app/apps/api/node_modules      ./apps/api/node_modules

ENV NODE_ENV=production
ENV API_PORT=3000
ENV API_WS_PORT=3001
EXPOSE 3000 3001

HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 \
  CMD node -e "const http = require('http'); const req = http.get('http://localhost:3000/health', (r) => { process.exit(r.statusCode === 200 ? 0 : 1); }); req.on('error', () => process.exit(1)); req.setTimeout(3000, () => { req.destroy(); process.exit(1); });"

CMD ["node", "apps/api/dist/main.js"]

# ---------------------------------------------------------------------------
# dashboard: Next.js standalone (port 3000)
# ---------------------------------------------------------------------------
FROM node:20-slim AS dashboard
RUN corepack enable && corepack prepare pnpm@9 --activate
WORKDIR /app

COPY --from=build /app/apps/dashboard/.next/standalone ./apps/dashboard/.next/standalone
COPY --from=build /app/apps/dashboard/.next/static     ./apps/dashboard/.next/static
RUN mkdir -p ./apps/dashboard/public

ENV NODE_ENV=production
ENV PORT=3000
EXPOSE 3000

HEALTHCHECK --interval=30s --timeout=5s --start-period=15s --retries=3 \
  CMD node -e "const http = require('http'); const req = http.get('http://localhost:3000/', (r) => { process.exit(r.statusCode < 400 ? 0 : 1); }); req.on('error', () => process.exit(1)); req.setTimeout(3000, () => { req.destroy(); process.exit(1); });"

CMD ["node", "apps/dashboard/.next/standalone/apps/dashboard/server.js"]

# ---------------------------------------------------------------------------
# mcp: MCP stdio server (agent-runner, no HTTP port)
# ---------------------------------------------------------------------------
FROM node:20-slim AS mcp
RUN corepack enable && corepack prepare pnpm@9 --activate
WORKDIR /app

COPY --from=build /app/package.json        ./
COPY --from=build /app/pnpm-workspace.yaml ./
COPY --from=build /app/node_modules        ./node_modules
COPY --from=build /app/packages/*/dist     ./packages/*/dist
COPY --from=build /app/packages/*/package.json    ./packages/*/
COPY --from=build /app/packages/*/node_modules    ./packages/*/node_modules
COPY --from=build /app/apps/agent-runner/dist     ./apps/agent-runner/dist
COPY --from=build /app/apps/agent-runner/package.json  ./apps/agent-runner/
COPY --from=build /app/apps/agent-runner/node_modules  ./apps/agent-runner/node_modules

ENV NODE_ENV=production

CMD ["node", "apps/agent-runner/dist/runtime/mcp.js"]

# ---------------------------------------------------------------------------
# production: Smallest image — defaults to MCP stdio (backward-compatible)
# ---------------------------------------------------------------------------
FROM mcp AS production
