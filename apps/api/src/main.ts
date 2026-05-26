#!/usr/bin/env node

/**
 * apps/api executable entrypoint.
 *
 * `src/index.ts` intentionally exports the APIServer class for embedding.
 * This file is the runtime bootstrap used by `pnpm -C apps/api start`.
 */

import fs from 'fs';
import net from 'net';
import path from 'path';
import { EventBus, createLogger, getMetrics, HealthChecker, getRouteTokenBreakdown, trackLLMTokens, setTokenDb } from '@mcp-rebuild/core';
import type { MemoryType } from '@mcp-rebuild/core';
import { OrchestratingEngine } from '@mcp-rebuild/workflow-engine';
import type { ISkillRegistry } from '@mcp-rebuild/workflow-engine';
import {
  JsonBackend,
  WorkflowStore,
  TaskResultStore,
  PersistenceListener,
  DualWriteWorkflowStore,
  DualWriteTaskResultStore,
  setDualWriteDb,
  setDualWriteMemoryDb,
  replicateMemory,
  replicateMemoryDelete,
} from '@mcp-rebuild/store';
import { MemoryStore } from '@mcp-rebuild/memory';
import * as policyTools from '@mcp-rebuild/policy';
import * as capabilityTools from '@mcp-rebuild/capability';
import { APIServer } from './index';
import type {
  MemoryServiceProvider,
  SearchServiceProvider,
  PolicyServiceProvider,
  CapabilityServiceProvider,
  ChatServiceProvider,
  ProviderServiceProvider,
  MonitoringServiceProvider,
} from './routes';

const logger = createLogger('api:main');

function isMemoryType(value: string): value is MemoryType {
  return (
    value === 'fact' ||
    value === 'preference' ||
    value === 'skill' ||
    value === 'experience' ||
    value === 'strategy' ||
    value === 'decision' ||
    value === 'artifact' ||
    value === 'learning' ||
    value === 'blocker'
  );
}

function envNumber(name: string, fallback: number): number {
  const raw = process.env[name];
  if (!raw) return fallback;
  const n = Number.parseInt(raw, 10);
  return Number.isFinite(n) ? n : fallback;
}

function resolveRepoRootFromDistDirname(): string | null {
  // When compiled, __dirname will typically be:
  //   <repo>/apps/api/dist
  // Walk up to monorepo root (contains pnpm-workspace.yaml).
  const candidate = path.resolve(__dirname, '../../../');
  const workspaceFile = path.join(candidate, 'pnpm-workspace.yaml');
  if (fs.existsSync(workspaceFile)) return candidate;
  return null;
}

async function isPortAvailable(port: number): Promise<boolean> {
  return new Promise((resolve) => {
    const server = net.createServer();
    server.unref();

    server.once('error', () => resolve(false));
    server.once('listening', () => {
      server.close(() => resolve(true));
    });

    server.listen(port, '0.0.0.0');
  });
}

async function findAvailablePort(preferred: number, maxAttempts: number = 50): Promise<number> {
  for (let p = preferred; p <= preferred + maxAttempts; p++) {
    // eslint-disable-next-line no-await-in-loop
    if (await isPortAvailable(p)) return p;
  }
  return preferred;
}

function resolveStorePath(): string {
  const envPath = process.env.MASDAY_STORE_PATH?.trim();
  const repoRoot = resolveRepoRootFromDistDirname();

  const defaultPath = repoRoot
    ? path.join(repoRoot, '.masday', 'state', 'masday.json')
    : path.resolve(process.cwd(), '.masday', 'state', 'masday.json');

  const rawPath = envPath && envPath.length > 0 ? envPath : defaultPath;
  const absPath = path.isAbsolute(rawPath) ? rawPath : path.resolve(process.cwd(), rawPath);
  fs.mkdirSync(path.dirname(absPath), { recursive: true });
  return absPath;
}

async function main(): Promise<void> {
  const requestedHttpPort = envNumber('API_PORT', envNumber('PORT', 3000));
  const requestedWsPort = envNumber('API_WS_PORT', envNumber('WS_PORT', 3001));
  const httpPort = await findAvailablePort(requestedHttpPort);
  let wsPort = await findAvailablePort(requestedWsPort);
  if (wsPort === httpPort) {
    wsPort = await findAvailablePort(wsPort + 1);
  }

  if (httpPort !== requestedHttpPort) {
    logger.warn({ requestedHttpPort, httpPort }, 'HTTP port in use; using next available port');
  }
  if (wsPort !== requestedWsPort) {
    logger.warn({ requestedWsPort, wsPort }, 'WS port in use; using next available port');
  }

  // --- Core runtime plumbing (EventBus + persistence + workflow engine) ---
  const eventBus = new EventBus();

  // Wire Drizzle client for token usage persistence and dual-write sync
  try {
    const { db, memories, tokenUsages, ...schema } = await import('@mcp-rebuild/db');
    setTokenDb(db, { tokenUsages });
    setDualWriteDb(db);
    setDualWriteMemoryDb(db, { memories });
    logger.info('Token usage persistence + dual-write sync: PostgreSQL connected');
  } catch (err) {
    logger.warn({ err: String(err) }, 'Token usage persistence: falling back to in-memory (DB unavailable)');
  }

  const skillRegistry: ISkillRegistry = {
    async execute(skill: string, input: unknown): Promise<unknown> { throw new Error("Not available: " + skill); },
    has(skill: string): boolean { return false; },
    getAll(): Array<{ name: string; description: string }> { return []; },
  };

  const storePath = resolveStorePath();
  const backend = new JsonBackend(storePath);
  backend.initialize();

  const workflowStore = new DualWriteWorkflowStore(new WorkflowStore(backend));
  const taskResultStore = new DualWriteTaskResultStore(new TaskResultStore(backend));
  const persistenceListener = new PersistenceListener(eventBus, workflowStore, taskResultStore);
  persistenceListener.start();

  const engine = new OrchestratingEngine(skillRegistry, eventBus, {
    // Keep bootstrap minimal: no agent coordinator, no skill routing.
    coordinator: false,
    enableSkillRouting: false,
    store: workflowStore,
  });

  // Restore persisted workflows if any exist.
  try {
    const persisted = workflowStore.loadAll();
    engine.restoreWorkflows(persisted);
  } catch (err) {
    logger.warn({ err }, 'Failed to restore persisted workflows (continuing)');
  }

  // --- Providers wired into HTTP API routes ---
  const repoRoot = resolveRepoRootFromDistDirname() ?? process.cwd();
  const memoryStore = new MemoryStore({
    filePath: path.join(repoRoot, '.msd', 'state', 'memories.json'),
  });
  await memoryStore.init();

  const memoryProvider: MemoryServiceProvider = {
    store: async (entry) => {
      const candidate = entry.memoryType;
      const type: MemoryType = isMemoryType(candidate) ? candidate : 'fact';

      const tags = [
        ...(entry.workflowId ? [entry.workflowId] : []),
        ...(entry.taskId ? [entry.taskId] : []),
      ];
      const record = await memoryStore.add(entry.content, {
        type,
        importance: entry.importance,
        tags,
        source: 'api',
      });
      await memoryStore.save();
      replicateMemory({
        id: record.id,
        memoryType: type,
        summary: entry.content.slice(0, 200),
        content: entry.content,
        importanceScore: entry.importance,
        createdByAgent: 'api',
        tags,
        workflowId: entry.workflowId,
        taskId: entry.taskId,
      });
      return { id: record.id };
    },
    storeResearch: async (entry) => {
      const record = await memoryStore.add(entry.findings, {
        type: 'fact',
        importance: 0.6,
        tags: ['research', entry.workflowId, entry.query, ...entry.sources],
        source: 'api',
      });
      await memoryStore.save();
      replicateMemory({
        id: record.id,
        memoryType: 'research',
        summary: entry.query,
        content: entry.findings,
        importanceScore: 0.6,
        createdByAgent: 'api',
        tags: ['research', entry.workflowId],
        workflowId: entry.workflowId,
      });
      return { id: record.id };
    },
    recallDocuments: async (workflowId, limit) => {
      const results = await memoryStore.search(workflowId, { limit: limit ?? 10, threshold: 0.0 });
      return results.map(r => r.memory);
    },
    recallRecent: async (workflowId, limit) => {
      const all = memoryStore.getAll().filter(m => m.tags.includes(workflowId));
      all.sort((a, b) => b.createdAt - a.createdAt);
      return all.slice(0, limit ?? 10);
    },
    recallByType: async (workflowId, type, limit) => {
      const all = memoryStore.getAll().filter(m => m.tags.includes(workflowId) && m.type === type);
      all.sort((a, b) => b.createdAt - a.createdAt);
      return all.slice(0, limit ?? 10);
    },
    recallByTask: async (taskId, limit) => {
      const all = memoryStore.getAll().filter(m => m.tags.includes(taskId));
      all.sort((a, b) => b.createdAt - a.createdAt);
      return all.slice(0, limit ?? 10);
    },
    update: async (id, updates) => {
      const updated = memoryStore.update(id, {
        content: typeof updates.content === 'string' ? updates.content : undefined,
        type: typeof updates.type === 'string' && isMemoryType(updates.type) ? updates.type : undefined,
        importance: typeof updates.importance === 'number' ? updates.importance : undefined,
        tags: Array.isArray(updates.tags) ? (updates.tags as string[]) : undefined,
      });
      if (!updated) return { updated: false };
      await memoryStore.save();
      if (typeof updates.content === 'string') {
        replicateMemory({
          id,
          memoryType: typeof updates.type === 'string' ? updates.type : 'fact',
          summary: updates.content.slice(0, 200),
          content: updates.content,
          importanceScore: typeof updates.importance === 'number' ? updates.importance : undefined,
          createdByAgent: 'api',
        });
      }
      return { updated: true };
    },
    delete: async (id) => {
      const deleted = memoryStore.delete(id);
      if (deleted) await memoryStore.save();
      replicateMemoryDelete(id);
      return { deleted };
    },
  };

  const searchProvider: SearchServiceProvider = {
    hybridContextPack: async (input) => {
      // The dashboard primarily needs an object-shaped response.
      // Full hybrid context packing is implemented in the MCP server runtime.
      return {
        workflowId: input.workflowId,
        planId: input.planId,
        taskId: input.taskId,
        fingerprint: `${input.workflowId}-${input.planId}-${input.taskId}`,
        contextSufficient: true,
        semanticMemories: [],
        semanticDocs: [],
      };
    },
    contextFingerprint: async (input) => {
      return {
        fingerprint: `${input.workflowId}-${input.planId}-${input.taskId}`,
        contextSufficient: true,
      };
    },
    codeSearch: async (input) => {
      // Minimal implementation: return the query echo; the dashboard can still render.
      return {
        query: input.query,
        glob: input.glob,
        type: input.type,
        limit: input.limit ?? 10,
        results: [],
      };
    },
  };

  const policyProvider: PolicyServiceProvider = {
    checkReadiness: (sessionKey) => policyTools.checkSessionReadiness(backend, { sessionKey }),
    validateExecution: (input) => policyTools.validateExecution(backend, input),
    validateCompletion: async (input) => {
      return policyTools.validateCompletion(backend, {
        workflowId: input.workflowId,
        taskId: input.taskId,
        // policy validator expects optional output/task context; evidence isn't part of the schema.
        acceptanceCriteria: input.acceptanceCriteria,
        outputText: input.evidence?.join('\n'),
      });
    },
    validateParallel: async (input) => {
      // This endpoint shape predates the current validator; return a best-effort result.
      return { ok: true, workflowId: input.workflowId, branchCount: input.branchResults.length };
    },
    detectDrift: async (input) => {
      // Defer to the orchestrator drift detector via policy tool wrapper.
      return policyTools.detectScopeDriftTool(backend, {
        taskTitle: input.originalScope,
        acceptanceCriteria: [],
        requiredContext: [],
        outputText: input.currentInput,
      });
    },
    requireContextRefresh: async (input) => {
      // API routes do not provide full fingerprint inputs; treat as always "refreshRequired: false".
      return {
        refreshRequired: false,
        reason: 'API endpoint does not supply fingerprint inputs; skipping',
        currentFingerprint: `${input.workflowId}-${input.planId}-${input.taskId}`,
      };
    },
    auditWorkflow: async (workflowId) => {
      const auditor = new (await import('@mcp-rebuild/policy')).WorkflowAuditor(backend);
      return auditor.audit(workflowId);
    },
  };

  const capabilityProvider: CapabilityServiceProvider = {
    createAgent: async (input) => {
      return capabilityTools.createAgentTool(backend, {
        projectRoot: input.projectRoot,
        name: input.name,
        role: input.role,
        description: input.description ?? '',
        instructions: '',
      });
    },
    createSkill: async (input) => {
      return capabilityTools.createSkillTool(backend, {
        projectRoot: input.projectRoot,
        name: input.name,
        description: input.description ?? '',
        trigger: 'manual',
        steps: [],
      });
    },
    listAgents: async (projectRoot) => capabilityTools.listAgentsTool(backend, { projectRoot }),
    matchAgent: async (input) => {
      return capabilityTools.matchAgentTool(backend, {
        projectRoot: input.projectRoot,
        taskDescription: input.taskType,
      });
    },
    listSkills: async (projectRoot) => capabilityTools.listSkillsTool(backend, { projectRoot }),
    listTemplates: async () => capabilityTools.listTemplatesTool(backend),
    checkReadiness: async (projectRoot) => capabilityTools.systemReadinessTool(backend, projectRoot),
    auditWorkflow: async (workflowId) => capabilityTools.workflowAuditTool(backend, { workflowId: workflowId || undefined }),
  };

  const chatProvider: ChatServiceProvider = {
    complete: async (input) => {
      try {
        const { createLLM } = await import('@mcp-rebuild/llm');
        const llm = createLLM();
        const result = await llm.complete(input.message, {
          model: input.model,
          temperature: input.temperature,
        });
        if (result.tokensUsed) {
          getMetrics().increment('llm.tokens_used', result.tokensUsed);
          trackLLMTokens({
            route: '/api/chat',
            model: result.model,
            promptTokens: result.promptTokens ?? 0,
            completionTokens: result.completionTokens ?? 0,
            totalTokens: result.tokensUsed,
            latencyMs: result.latencyMs,
          });
        }
        return result;
      } catch (err: unknown) {
        const message = err instanceof Error ? err.message : String(err);
        return { error: message, ok: false };
      }
    },
    react: async (input) => {
      try {
        const { createLLM } = await import('@mcp-rebuild/llm');
        const { ReActAgent } = await import('@mcp-rebuild/intelligence');
        const rawLlm = createLLM();
        // Wrap LLM to track token usage across all ReAct steps
        const llm = {
          complete: async (prompt: string, opts?: Record<string, unknown>) => {
            const result = await rawLlm.complete(prompt, opts as import('@mcp-rebuild/llm').LLMOptions);
            if (result.tokensUsed) {
              getMetrics().increment('llm.tokens_used', result.tokensUsed);
              trackLLMTokens({
                route: '/api/chat/react',
                model: result.model,
                promptTokens: result.promptTokens ?? 0,
                completionTokens: result.completionTokens ?? 0,
                totalTokens: result.tokensUsed,
                latencyMs: result.latencyMs,
              });
            }
            return result;
          },
          chat: async (messages: Array<{ role: 'system' | 'user' | 'assistant'; content: string }>, opts?: Record<string, unknown>) => {
            const result = await rawLlm.chat(messages, opts as import('@mcp-rebuild/llm').LLMOptions);
            if (result.tokensUsed) {
              getMetrics().increment('llm.tokens_used', result.tokensUsed);
              trackLLMTokens({
                route: '/api/chat/react',
                model: result.model,
                promptTokens: result.promptTokens ?? 0,
                completionTokens: result.completionTokens ?? 0,
                totalTokens: result.tokensUsed,
                latencyMs: result.latencyMs,
              });
            }
            return result;
          },
        };
        const agent = new ReActAgent({
          llm,
          memory: {
            search: async (query, opts) => memoryStore.search(query, { limit: opts?.limit ?? 5 }),
            add: async (content, opts) => memoryStore.add(content, {
              type: (opts?.type as MemoryType) ?? 'fact',
              importance: opts?.importance,
              source: opts?.source,
            }),
          },
          maxSteps: input.maxIterations ?? 5,
        });
        const result = await agent.run(input.goal);
        const steps = result.traces.map((t) => ({
          step: t.step,
          thought: t.thought,
          action: t.action,
          observation: t.observation,
          timestamp: t.timestamp,
        }));
        return { ok: true, steps, result: result.answer };
      } catch (err: unknown) {
        const message = err instanceof Error ? err.message : String(err);
        return { ok: false, error: message, goal: input.goal };
      }
    },
  };

  const providerService: ProviderServiceProvider = {
    listProviders: async () => {
      return {
        providers: ['anthropic', 'openai', 'custom'],
        env: {
          LLM_PROVIDER: process.env.LLM_PROVIDER ?? null,
          LLM_BASE_URL: process.env.LLM_BASE_URL ?? null,
          LLM_MODEL: process.env.LLM_MODEL ?? null,
        },
      };
    },
    testProvider: async (name, input) => {
      return { ok: false, provider: name, message: 'Provider test not implemented in API bootstrap', input };
    },
  };

  const healthChecker = new HealthChecker(eventBus, { checkInterval: 60000 });
  healthChecker.registerCheck('persistence', async () => {
    try {
      workflowStore.loadAll();
      return { name: 'persistence', status: 'pass' as const, message: 'Store responsive', duration: 0 };
    } catch (err) {
      return { name: 'persistence', status: 'fail' as const, message: `Store error: ${String(err)}`, duration: 0 };
    }
  });
  healthChecker.start();

  // Avoid circular construction dependency: monitoring provider needs APIServer stats.
  let apiServer: APIServer | null = null;

  const monitoringProvider: MonitoringServiceProvider = {
    getHealth: async () => healthChecker.check(),
    getMetrics: () => ({ points: getMetrics().getPoints() }),
    getStats: async () => {
      try {
        const { db, workflows, tasks, memories } = await import('@mcp-rebuild/db');
        const { count: cnt, eq: eqOp, desc: descOp, and: andOp, gte, lte } = await import('drizzle-orm');
        const [wc] = await db.select({ total: cnt() }).from(workflows);
        const [tc] = await db.select({ total: cnt() }).from(tasks);
        const [mc] = await db.select({ total: cnt() }).from(memories);
        const [doneCount] = await db.select({ total: cnt() }).from(workflows).where(eqOp(workflows.status, 'DONE'));
        const [failCount] = await db.select({ total: cnt() }).from(workflows).where(eqOp(workflows.status, 'FAILED'));
        const activeCount = wc.total - doneCount.total - failCount.total;
        const [tasksDone] = await db.select({ total: cnt() }).from(tasks).where(eqOp(tasks.status, 'DONE'));
        const [tasksFail] = await db.select({ total: cnt() }).from(tasks).where(eqOp(tasks.status, 'FAILED'));
        return {
          engine: {
            workflows: wc.total,
            workflowsActive: activeCount,
            workflowsDone: doneCount.total,
            workflowsFailed: failCount.total,
            tasksTotal: tc.total,
            tasksCompleted: tasksDone.total,
            tasksFailed: tasksFail.total,
            memoriesTotal: mc.total,
            tokensUsed: getMetrics().getCounter('llm.tokens_used').sum,
            tokenBreakdown: Object.fromEntries(getRouteTokenBreakdown()),
          },
          api: apiServer ? apiServer.getStats() : { started: false },
        };
      } catch {
        const allWorkflows = engine.listWorkflows();
        const workflowsDone = allWorkflows.filter((w) => w.state === 'DONE').length;
        const workflowsFailed = allWorkflows.filter((w) => w.state === 'FAILED').length;
        const workflowsActive = allWorkflows.length - workflowsDone - workflowsFailed;
        const allTasks = allWorkflows.flatMap((w) => w.tasks ?? []);
        const allMemories = memoryStore.getAll();
        return {
          engine: {
            workflows: allWorkflows.length,
            workflowsActive,
            workflowsDone,
            workflowsFailed,
            tasksTotal: allTasks.length,
            tasksCompleted: allTasks.filter((t) => t.state === 'done').length,
            tasksFailed: allTasks.filter((t) => t.state === 'failed').length,
            memoriesTotal: allMemories.length,
            tokensUsed: getMetrics().getCounter('llm.tokens_used').sum,
            tokenBreakdown: Object.fromEntries(getRouteTokenBreakdown()),
          },
          api: apiServer ? apiServer.getStats() : { started: false },
        };
      }
    },
    getTokenUsage: async (params) => {
      try {
        const { db, tokenUsages } = await import('@mcp-rebuild/db');
        const { eq: eqOp, desc: descOp, and: andOp, gte, lte } = await import('drizzle-orm');
        const conditions = [];
        if (params.from) conditions.push(gte(tokenUsages.createdAt, new Date(params.from)));
        if (params.to) conditions.push(lte(tokenUsages.createdAt, new Date(params.to)));
        if (params.route) conditions.push(eqOp(tokenUsages.route, params.route));
        if (params.model) conditions.push(eqOp(tokenUsages.model, params.model));

        const whereClause = conditions.length > 0 ? andOp(...conditions) : undefined;
        const records = whereClause
          ? await db.select().from(tokenUsages).where(whereClause).orderBy(descOp(tokenUsages.createdAt)).limit(1000)
          : await db.select().from(tokenUsages).orderBy(descOp(tokenUsages.createdAt)).limit(1000);

        const groupBy = params.groupBy ?? 'route';

        if (groupBy === 'model') {
          const buckets = new Map<string, { totalTokens: number; promptTokens: number; completionTokens: number; count: number }>();
          for (const r of records) {
            const key = r.model ?? 'unknown';
            const b = buckets.get(key) ?? { totalTokens: 0, promptTokens: 0, completionTokens: 0, count: 0 };
            b.totalTokens += r.totalTokens ?? 0;
            b.promptTokens += r.promptTokens ?? 0;
            b.completionTokens += r.completionTokens ?? 0;
            b.count += 1;
            buckets.set(key, b);
          }
          return { groupBy: 'model', buckets: Object.fromEntries(buckets), totalRecords: records.length };
        }

        if (groupBy === 'day') {
          const buckets = new Map<string, { totalTokens: number; promptTokens: number; completionTokens: number; count: number }>();
          for (const r of records) {
            const key = r.createdAt.toISOString().slice(0, 10);
            const b = buckets.get(key) ?? { totalTokens: 0, promptTokens: 0, completionTokens: 0, count: 0 };
            b.totalTokens += r.totalTokens ?? 0;
            b.promptTokens += r.promptTokens ?? 0;
            b.completionTokens += r.completionTokens ?? 0;
            b.count += 1;
            buckets.set(key, b);
          }
          return { groupBy: 'day', buckets: Object.fromEntries(buckets), totalRecords: records.length };
        }

        // Default: group by route
        const buckets = new Map<string, { totalTokens: number; promptTokens: number; completionTokens: number; count: number }>();
        for (const r of records) {
          const key = r.route;
          const b = buckets.get(key) ?? { totalTokens: 0, promptTokens: 0, completionTokens: 0, count: 0 };
          b.totalTokens += r.totalTokens ?? 0;
          b.promptTokens += r.promptTokens ?? 0;
          b.completionTokens += r.completionTokens ?? 0;
          b.count += 1;
          buckets.set(key, b);
        }
        return { groupBy: 'route', buckets: Object.fromEntries(buckets), totalRecords: records.length };
      } catch (err) {
        logger.warn({ err: String(err) }, 'Token usage aggregation failed');
        return { error: 'Token usage aggregation unavailable', buckets: {}, totalRecords: 0 };
      }
    },
  };

  const dbReader = {
    listWorkflows: async () => {
      const { db, workflows } = await import('@mcp-rebuild/db');
      const { desc } = await import('drizzle-orm');
      return db.select().from(workflows).orderBy(desc(workflows.createdAt));
    },
    getWorkflow: async (id: string) => {
      const { db, workflows } = await import('@mcp-rebuild/db');
      const { eq } = await import('drizzle-orm');
      const [row] = await db.select().from(workflows).where(eq(workflows.id, id));
      return row ?? null;
    },
    getWorkflowTasks: async (workflowId: string) => {
      const { db, tasks } = await import('@mcp-rebuild/db');
      const { eq, desc } = await import('drizzle-orm');
      return db.select().from(tasks).where(eq(tasks.workflowId, workflowId)).orderBy(desc(tasks.createdAt));
    },
  };

  apiServer = new APIServer(
    {
      eventBus,
      engine,
      memoryProvider,
      searchProvider,
      policyProvider,
      capabilityProvider,
      chatProvider,
      providerService,
      monitoringProvider,
      dbReader,
    },
    {
      httpPort,
      wsPort,
    },
  );

  await apiServer.start();
  logger.info({ httpPort, wsPort, storePath }, 'API listening');
  logger.info(`HTTP: http://localhost:${httpPort}`);
  logger.info(`WS:   ws://localhost:${wsPort}`);

  const shutdown = async (signal: string) => {
    logger.info({ signal }, 'Shutting down API server...');
    try {
      await apiServer.stop();
    } finally {
      healthChecker.stop();
      process.exit(0);
    }
  };

  process.on('SIGINT', () => void shutdown('SIGINT'));
  process.on('SIGTERM', () => void shutdown('SIGTERM'));
}

main().catch((error: unknown) => {
  logger.error({ err: error }, 'Fatal error');
  process.exit(1);
});
