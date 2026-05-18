FROM node:20-slim AS base
RUN corepack enable && corepack prepare pnpm@9 --activate
WORKDIR /app

# Dependencies stage
FROM base AS deps
COPY package.json pnpm-workspace.yaml pnpm-lock.yaml turbo.json ./
COPY packages/*/package.json ./packages/*/
COPY apps/*/package.json ./apps/*/
RUN pnpm install --frozen-lockfile

# Generate Prisma client
FROM deps AS prisma
COPY packages/db/prisma ./packages/db/prisma
RUN pnpm db:generate

# Build stage
FROM base AS build
COPY --from=deps /app/node_modules ./node_modules
COPY --from=deps /app/packages/*/node_modules ./packages/*/node_modules
COPY --from=deps /app/apps/*/node_modules ./apps/*/node_modules
COPY --from=prisma /app/packages/db/node_modules/.prisma ./packages/db/node_modules/.prisma
COPY . .
RUN pnpm build

# Production stage
FROM node:20-slim AS production
RUN corepack enable && corepack prepare pnpm@9 --activate
WORKDIR /app

COPY --from=build /app/package.json ./
COPY --from=build /app/pnpm-workspace.yaml ./
COPY --from=build /app/node_modules ./node_modules
COPY --from=build /app/packages/*/dist ./packages/*/dist
COPY --from=build /app/packages/*/package.json ./packages/*/
COPY --from=build /app/packages/*/node_modules ./packages/*/node_modules
COPY --from=build /app/apps/*/dist ./apps/*/dist
COPY --from=build /app/apps/*/package.json ./apps/*/
COPY --from=build /app/apps/*/node_modules ./apps/*/node_modules
COPY --from=prisma /app/packages/db/node_modules/.prisma ./packages/db/node_modules/.prisma

ENV NODE_ENV=production
ENV DATABASE_URL="postgresql://postgres:postgres@postgres:5432/masday_workflow"

EXPOSE 3000

CMD ["npx", "tsx", "apps/agent-runner/src/runtime/mcp.ts"]
