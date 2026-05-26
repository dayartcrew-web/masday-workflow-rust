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
import { existsSync, readdirSync, readFileSync } from 'fs';
import { config } from 'dotenv';
// Load .env before any module that reads DATABASE_URL
config({ path: path.resolve(process.cwd(), '.env') });
import { EventBus, createLogger, getMetrics, HealthChecker, getRouteTokenBreakdown, trackLLMTokens, setPrismaClient } from '@mcp-rebuild/core';
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
  setDualWritePrisma,
  setDualWriteMemoryPrisma,
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
import { eq, desc, and, sql, count } from 'drizzle-orm';

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

  // Wire Drizzle db for PostgreSQL persistence, token usage, and dual-write sync
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  let drizzleDb: any = null;
  let dbReady = false;
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  let schemaTables: any = null;

  try {
    const dbModule = await import('@mcp-rebuild/db');
    schemaTables = dbModule;
    const healthy = await dbModule.healthCheck(15000);
    if (healthy) {
      drizzleDb = dbModule.db;
      dbReady = true;
      setPrismaClient(drizzleDb);
      setDualWritePrisma(drizzleDb);
      setDualWriteMemoryPrisma(drizzleDb);
      logger.info('Drizzle PostgreSQL connected — all API routes will use real DB queries');
    } else {
      logger.warn('PostgreSQL health check failed — falling back to in-memory engine');
    }
  } catch (err) {
    logger.warn({ err: String(err) }, 'Drizzle init failed — falling back to in-memory engine');
  }

  /** Read agent/skill .md files and build a registry that executes tasks via LLM */
  const agentsDir = path.resolve(process.cwd(), '.claude/agents');
  const skillsDir = path.resolve(process.cwd(), '.claude/skills');
  const agentDefs = new Map<string, { description: string; instructions: string }>();

  function loadAgentFiles(): void {
    for (const dir of [agentsDir, skillsDir]) {
      if (!existsSync(dir)) continue;
      const entries = readdirSync(dir, { withFileTypes: true });
      for (const entry of entries) {
        try {
          const name = entry.name.replace(/\.md$/, '');
          const filePath = path.join(dir, entry.isDirectory() ? `${entry.name}/${entry.name}.md` : entry.name);
          if (!existsSync(filePath)) continue;
          const content = readFileSync(filePath, 'utf-8');
          const frontmatterMatch = content.match(/^---\n([\s\S]*?)\n---\n([\s\S]*)$/);
          if (frontmatterMatch) {
            const descMatch = frontmatterMatch[1].match(/description:\s*>?\s*([\s\S]*?)(?=\n[a-z]|\n---)/);
            const description = descMatch ? descMatch[1].trim().replace(/\n\s+/g, ' ') : name;
            agentDefs.set(name, { description, instructions: frontmatterMatch[2].trim() });
          }
        } catch { /* skip unreadable files */ }
      }
    }
  }
  loadAgentFiles();
  logger.info({ agents: Array.from(agentDefs.keys()) }, 'Loaded agent/skill definitions');

  const skillRegistry: ISkillRegistry = {
    async execute(skill: string, input: unknown): Promise<unknown> {
      const def = agentDefs.get(skill);
      const { createLLM } = await import('@mcp-rebuild/llm');
      const llmConfig = await resolveLLMConfig();
      const llm = createLLM(llmConfig);
      const inputStr = typeof input === 'string' ? input : JSON.stringify(input, null, 2);
      const systemPrompt = def
        ? `You are "${skill}". ${def.description}\n\nInstructions:\n${def.instructions}`
        : `You are a task executor. Execute the task and return the result.`;
      const userPrompt = `Execute this task:\n${inputStr}\n\nReturn ONLY a JSON object with your result.`;
      const result = await llm.chat([
        { role: 'system', content: systemPrompt },
        { role: 'user', content: userPrompt },
      ]);
      try {
        const text = result.text || '';
        const jsonMatch = text.match(/\{[\s\S]*\}/);
        return jsonMatch ? JSON.parse(jsonMatch[0]) : { result: text };
      } catch {
        return { result: result.text || '' };
      }
    },
    has(skill: string): boolean { return agentDefs.has(skill); },
    getAll(): Array<{ name: string; description: string }> {
      return Array.from(agentDefs.entries()).map(([name, def]) => ({ name, description: def.description }));
    },
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

      // Drizzle-first: write directly to PostgreSQL
      if (dbReady && drizzleDb && schemaTables) {
        try {
          const memId = `mem_${Date.now()}_${Math.random().toString(36).slice(2, 8)}`;
          await drizzleDb.insert(schemaTables.memories).values({
            id: memId,
            memoryType: type,
            summary: entry.summary ?? entry.content.slice(0, 200),
            content: entry.content,
            importanceScore: entry.importance ?? 0.5,
            createdByAgent: 'api',
            tags,
            workflowId: entry.workflowId ?? null,
            taskId: entry.taskId ?? null,
          });
          return { id: memId };
        } catch (err) {
          logger.warn({ err: String(err) }, 'Drizzle memory store failed, falling back to JSON');
        }
      }

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
        summary: entry.summary ?? entry.content.slice(0, 200),
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
      // Drizzle-first: write to PostgreSQL Memory + ContextDocument
      if (dbReady && drizzleDb && schemaTables) {
        try {
          const recId = `mem_${Date.now()}_${Math.random().toString(36).slice(2, 8)}`;
          await drizzleDb.insert(schemaTables.memories).values({
            id: recId,
            memoryType: 'research',
            summary: entry.query,
            content: entry.findings,
            importanceScore: 0.6,
            createdByAgent: 'api',
            tags: ['research', entry.workflowId, ...entry.sources],
            workflowId: entry.workflowId,
          });
          await drizzleDb.insert(schemaTables.contextDocuments).values({
            id: `ctx_${Date.now()}_${Math.random().toString(36).slice(2, 8)}`,
            workflowId: entry.workflowId,
            sourceType: 'research',
            title: entry.query,
            content: entry.findings,
            metadata: { sources: entry.sources, agent: 'api' } as never,
          });
          return { id: recId };
        } catch (err) {
          logger.warn({ err: String(err) }, 'Drizzle research store failed, falling back to JSON');
        }
      }

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
      if (dbReady && drizzleDb && schemaTables) {
        try {
          const rows = await drizzleDb.select().from(schemaTables.memories)
            .where(eq(schemaTables.memories.workflowId, workflowId))
            .orderBy(desc(schemaTables.memories.createdAt))
            .limit(limit ?? 10);
          return rows;
        } catch {
          // fall through to JSON
        }
      }
      const results = await memoryStore.search(workflowId, { limit: limit ?? 10, threshold: 0.0 });
      return results.map(r => r.memory);
    },
    recallRecent: async (workflowId, limit) => {
      if (dbReady && drizzleDb && schemaTables) {
        try {
          const rows = await drizzleDb.select().from(schemaTables.memories)
            .where(eq(schemaTables.memories.workflowId, workflowId))
            .orderBy(desc(schemaTables.memories.createdAt))
            .limit(limit ?? 10);
          return rows;
        } catch {
          // fall through to JSON
        }
      }
      const all = memoryStore.getAll().filter(m => m.tags.includes(workflowId));
      all.sort((a, b) => b.createdAt - a.createdAt);
      return all.slice(0, limit ?? 10);
    },
    recallByType: async (workflowId, type, limit) => {
      if (dbReady && drizzleDb && schemaTables) {
        try {
          const rows = await drizzleDb.select().from(schemaTables.memories)
            .where(and(
              eq(schemaTables.memories.workflowId, workflowId),
              eq(schemaTables.memories.memoryType, type),
            ))
            .orderBy(desc(schemaTables.memories.createdAt))
            .limit(limit ?? 10);
          return rows;
        } catch {
          // fall through to JSON
        }
      }
      const all = memoryStore.getAll().filter(m => m.tags.includes(workflowId) && m.type === type);
      all.sort((a, b) => b.createdAt - a.createdAt);
      return all.slice(0, limit ?? 10);
    },
    recallByTask: async (taskId, limit) => {
      if (dbReady && drizzleDb && schemaTables) {
        try {
          const rows = await drizzleDb.select().from(schemaTables.memories)
            .where(eq(schemaTables.memories.taskId, taskId))
            .orderBy(desc(schemaTables.memories.createdAt))
            .limit(limit ?? 10);
          return rows;
        } catch {
          // fall through to JSON
        }
      }
      const all = memoryStore.getAll().filter(m => m.tags.includes(taskId));
      all.sort((a, b) => b.createdAt - a.createdAt);
      return all.slice(0, limit ?? 10);
    },
    update: async (id, updates) => {
      if (dbReady && drizzleDb && schemaTables) {
        try {
          const setValues: Record<string, unknown> = {};
          if (typeof updates.content === 'string') setValues.content = updates.content;
          if (typeof updates.type === 'string') setValues.memoryType = updates.type;
          if (typeof updates.importance === 'number') setValues.importanceScore = updates.importance;
          if (Array.isArray(updates.tags)) setValues.tags = updates.tags;
          if (Object.keys(setValues).length > 0) {
            await drizzleDb.update(schemaTables.memories).set(setValues).where(eq(schemaTables.memories.id, id));
          }
          return { updated: true };
        } catch {
          // fall through to JSON
        }
      }
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
      if (dbReady && drizzleDb && schemaTables) {
        try {
          await drizzleDb.delete(schemaTables.memories).where(eq(schemaTables.memories.id, id));
          return { deleted: true };
        } catch {
          // fall through to JSON
        }
      }
      const deleted = memoryStore.delete(id);
      if (deleted) await memoryStore.save();
      replicateMemoryDelete(id);
      return { deleted };
    },
  };

  const searchProvider: SearchServiceProvider = {
    hybridContextPack: async (input) => {
      // Drizzle-first: query real context from PostgreSQL
      if (dbReady && drizzleDb && schemaTables) {
        try {
          const [memRows, docRows] = await Promise.all([
            drizzleDb.select().from(schemaTables.memories)
              .where(eq(schemaTables.memories.workflowId, input.workflowId))
              .orderBy(desc(schemaTables.memories.importanceScore))
              .limit(10),
            drizzleDb.select().from(schemaTables.contextDocuments)
              .where(eq(schemaTables.contextDocuments.workflowId, input.workflowId))
              .orderBy(desc(schemaTables.contextDocuments.createdAt))
              .limit(10),
          ]);

          // Log retrieval
          await drizzleDb.insert(schemaTables.retrievalLogs).values({
            workflowId: input.workflowId,
            taskId: input.taskId,
            agentName: 'api',
            query: `hybrid-context-pack:${input.planId}`,
            source: 'api-search',
            results: { memoryCount: memRows.length, docCount: docRows.length },
          });

          return {
            workflowId: input.workflowId,
            planId: input.planId,
            taskId: input.taskId,
            fingerprint: `${input.workflowId}-${input.planId}-${input.taskId}`,
            contextSufficient: memRows.length > 0 || docRows.length > 0,
            semanticMemories: memRows,
            semanticDocs: docRows,
          };
        } catch {
          // fall through to stub
        }
      }
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
      // Drizzle-first: compute fingerprint from actual task context
      if (dbReady && drizzleDb && schemaTables) {
        try {
          const [task] = await drizzleDb.select({ contextFingerprint: schemaTables.tasks.contextFingerprint })
            .from(schemaTables.tasks)
            .where(eq(schemaTables.tasks.id, input.taskId))
            .limit(1);
          if (task?.contextFingerprint) {
            return { fingerprint: task.contextFingerprint, contextSufficient: true };
          }
        } catch {
          // fall through to stub
        }
      }
      return {
        fingerprint: `${input.workflowId}-${input.planId}-${input.taskId}`,
        contextSufficient: true,
      };
    },
    codeSearch: async (input) => {
      // Log the search query
      if (dbReady && drizzleDb && schemaTables) {
        try {
          await drizzleDb.insert(schemaTables.retrievalLogs).values({
            agentName: 'api',
            query: input.query,
            source: 'code_search',
            results: { glob: input.glob, type: input.type, limit: input.limit ?? 10 },
          });
        } catch {
          // non-critical
        }
      }
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
    checkReadiness: (sessionKey) => {
      // Drizzle-first: check SessionState in PostgreSQL
      if (dbReady && drizzleDb && schemaTables) {
        return (async () => {
          try {
            const [state] = await drizzleDb.select().from(schemaTables.sessionStates)
              .where(eq(schemaTables.sessionStates.sessionKey, sessionKey)).limit(1);
            const ready = state ? (state.workflowLoaded && state.planLoaded && state.taskLoaded) : false;
            return {
              sessionKey,
              ready,
              workflowLoaded: state?.workflowLoaded ?? false,
              planLoaded: state?.planLoaded ?? false,
              taskLoaded: state?.taskLoaded ?? false,
            };
          } catch {
            return policyTools.checkSessionReadiness(backend, { sessionKey });
          }
        })();
      }
      return policyTools.checkSessionReadiness(backend, { sessionKey });
    },
    validateExecution: (input) => {
      if (dbReady && drizzleDb && schemaTables) {
        return (async () => {
          try {
            const [wf] = await drizzleDb.select().from(schemaTables.workflows)
              .where(eq(schemaTables.workflows.id, input.workflowId)).limit(1);
            if (!wf) return { valid: false, reason: 'Workflow not found', ...input };
            if (wf.status === 'DONE' || wf.status === 'FAILED') return { valid: false, reason: 'Workflow is ' + wf.status, ...input };
            const [task] = await drizzleDb.select().from(schemaTables.tasks)
              .where(eq(schemaTables.tasks.id, input.taskId)).limit(1);
            if (!task) return { valid: false, reason: 'Task not found', ...input };
            if (task.status === 'DONE') return { valid: false, reason: 'Task already completed', ...input };
            return { valid: true, ...input };
          } catch {
            return policyTools.validateExecution(backend, input);
          }
        })();
      }
      return policyTools.validateExecution(backend, input);
    },
    validateCompletion: async (input) => {
      if (dbReady && drizzleDb && schemaTables) {
        try {
          const [review] = await drizzleDb.select().from(schemaTables.reviewDecisions)
            .where(and(
              eq(schemaTables.reviewDecisions.workflowId, input.workflowId),
              eq(schemaTables.reviewDecisions.taskId, input.taskId),
            ))
            .orderBy(desc(schemaTables.reviewDecisions.createdAt)).limit(1);
          const approved = review?.decision === 'APPROVED';
          return { valid: approved, reviewDecision: review?.decision ?? 'none', ...input };
        } catch {
          // fall through to policy tools
        }
      }
      return policyTools.validateCompletion(backend, {
        workflowId: input.workflowId,
        taskId: input.taskId,
        acceptanceCriteria: input.acceptanceCriteria,
        outputText: input.evidence?.join('\n'),
      });
    },
    validateParallel: async (input) => {
      if (dbReady && drizzleDb && schemaTables) {
        try {
          const branches = await drizzleDb.select().from(schemaTables.parallelBranches)
            .where(eq(schemaTables.parallelBranches.workflowId, input.workflowId));
          const allDone = branches.length > 0 && branches.every((b: Record<string, unknown>) => b.status === 'COMPLETED');
          return { ok: allDone, workflowId: input.workflowId, branchCount: input.branchResults.length, dbBranchCount: branches.length };
        } catch {
          // fall through
        }
      }
      return { ok: true, workflowId: input.workflowId, branchCount: input.branchResults.length };
    },
    detectDrift: async (input) => {
      if (dbReady && drizzleDb && schemaTables) {
        try {
          const suspiciousKeywords = ['unrelated', 'off-topic', 'completely different'];
          const driftDetected = suspiciousKeywords.some(k => input.currentInput.toLowerCase().includes(k));
          return { driftDetected, outputLength: input.currentInput.length, workflowId: input.workflowId };
        } catch {
          // fall through
        }
      }
      return policyTools.detectScopeDriftTool(backend, {
        taskTitle: input.originalScope,
        acceptanceCriteria: [],
        requiredContext: [],
        outputText: input.currentInput,
      });
    },
    requireContextRefresh: async (input) => {
      if (dbReady && drizzleDb && schemaTables) {
        try {
          const [wf] = await drizzleDb.select({ id: schemaTables.workflows.id, updatedAt: schemaTables.workflows.updatedAt })
            .from(schemaTables.workflows)
            .where(eq(schemaTables.workflows.id, input.workflowId)).limit(1);
          const currentFingerprint = wf ? `${wf.id}-${wf.updatedAt.getTime()}` : null;
          return {
            refreshRequired: false,
            reason: 'Fingerprint computed from DB',
            currentFingerprint,
          };
        } catch {
          // fall through
        }
      }
      return {
        refreshRequired: false,
        reason: 'API endpoint does not supply fingerprint inputs; skipping',
        currentFingerprint: `${input.workflowId}-${input.planId}-${input.taskId}`,
      };
    },
    auditWorkflow: async (workflowId) => {
      if (dbReady && drizzleDb && schemaTables) {
        try {
          const [wf] = await drizzleDb.select().from(schemaTables.workflows)
            .where(eq(schemaTables.workflows.id, workflowId)).limit(1);
          if (!wf) return { workflowId, stuckTasks: [], missingReviews: ['Workflow not found'], incompleteProgress: [], totalIssues: 1 };

          const taskRows = await drizzleDb.select().from(schemaTables.tasks)
            .where(eq(schemaTables.tasks.workflowId, workflowId));
          const progressRows = await drizzleDb.select().from(schemaTables.taskProgressLogs)
            .where(eq(schemaTables.taskProgressLogs.workflowId, workflowId));

          // Check for reviews
          const reviewRows = await drizzleDb.select().from(schemaTables.reviewDecisions)
            .where(eq(schemaTables.reviewDecisions.workflowId, workflowId));
          const reviewedTaskIds = new Set(reviewRows.map((r: Record<string, unknown>) => r.taskId as string));

          // Stuck tasks: RUNNING with no recent progress
          const stuckTasks = taskRows
            .filter((t: Record<string, unknown>) => t.status === 'RUNNING'
              && progressRows.filter((p: Record<string, unknown>) => p.taskId === t.id).length === 0)
            .map((t: Record<string, unknown>) => ({
              id: t.id, name: t.title, state: 'running', agent: t.ownerAgent ?? '', skill: '', dependencies: [],
            }));

          // Missing reviews: DONE tasks without a review
          const missingReviews = taskRows
            .filter((t: Record<string, unknown>) => t.status === 'DONE' && !reviewedTaskIds.has(t.id as string))
            .map((t: Record<string, unknown>) => `${t.title} (no review)`);

          // Incomplete progress: tasks with progress < 100 but not PENDING
          const incompleteProgress = taskRows
            .filter((t: Record<string, unknown>) => t.status !== 'PENDING' && t.status !== 'DONE' && (t.progressPercent as number ?? 0) < 100)
            .map((t: Record<string, unknown>) => `${t.title}: ${(t.progressPercent as number ?? 0)}%`);

          return {
            workflowId,
            stuckTasks,
            missingReviews,
            incompleteProgress,
            totalIssues: stuckTasks.length + missingReviews.length + incompleteProgress.length,
            taskCount: taskRows.length,
          };
        } catch {
          // fall through to policy tools
        }
      }
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
    checkReadiness: async (projectRoot) => {
      const base = await capabilityTools.systemReadinessTool(backend, projectRoot);
      return { ...base, postgresql: dbReady };
    },
    auditWorkflow: async (workflowId) => {
      if (dbReady && drizzleDb && schemaTables && workflowId) {
        try {
          const [wf] = await drizzleDb.select().from(schemaTables.workflows)
            .where(eq(schemaTables.workflows.id, workflowId)).limit(1);
          if (!wf) return { audited: 0, issues: ['Workflow not found'] };
          const taskRows = await drizzleDb.select().from(schemaTables.tasks)
            .where(eq(schemaTables.tasks.workflowId, workflowId));
          const progressRows = await drizzleDb.select().from(schemaTables.taskProgressLogs)
            .where(eq(schemaTables.taskProgressLogs.workflowId, workflowId));
          const issues: string[] = [];
          const running = taskRows.filter((t: Record<string, unknown>) => t.status === 'RUNNING');
          if (running.length > 1) issues.push(`Multiple running tasks: ${running.map((t: Record<string, unknown>) => (t.id as string).slice(0, 8)).join(', ')}`);
          const noProgress = taskRows.filter((t: Record<string, unknown>) => t.status === 'RUNNING' && progressRows.filter((p: Record<string, unknown>) => p.taskId === t.id).length === 0);
          if (noProgress.length > 0) issues.push(`Running tasks with no progress: ${noProgress.map((t: Record<string, unknown>) => (t.id as string).slice(0, 8)).join(', ')}`);
          return { audited: 1, issues, taskCount: taskRows.length };
        } catch {
          // fall through to capability tools
        }
      }
      return capabilityTools.workflowAuditTool(backend, { workflowId: workflowId || undefined });
    },
  };

  /** Resolve LLM config from DB (match by model → default → first), fallback undefined → ENV */
  const resolveLLMConfig = async (model?: string): Promise<Parameters<typeof import('@mcp-rebuild/llm').createLLM>[0] | undefined> => {
    if (!dbReady || !drizzleDb || !schemaTables) return undefined;
    try {
      const allProviders = await drizzleDb.select().from(schemaTables.llmProviderConfigs);
      let match: typeof allProviders[0] | undefined = allProviders.find((p: Record<string, unknown>) =>
        Array.isArray(p.models) && model && (p.models as string[]).includes(model)
      );
      if (!match) {
        match = allProviders.find((p: Record<string, unknown>) => p.isDefault) || allProviders[0];
      }
      if (match) {
        const raw = match.apiKeyEnvVar || '';
        const isEnvVarName = /^[A-Z_][A-Z0-9_]+$/.test(raw);
        const apiKey = isEnvVarName ? (process.env[raw] || '') : raw;
        if (apiKey) {
          return {
            provider: (match.providerName?.toLowerCase()?.includes('anthropic') ? 'anthropic'
              : match.providerName?.toLowerCase()?.includes('openai') ? 'openai' : 'custom') as 'anthropic' | 'openai' | 'custom',
            apiKey,
            baseUrl: match.baseUrl ?? undefined,
            defaultModel: (Array.isArray(match.models) && match.models.length > 0) ? match.models[0] as string : undefined,
          };
        }
      }
    } catch { /* fall through */ }
    return undefined;
  };

  const chatProvider: ChatServiceProvider = {
    complete: async (input) => {
      try {
        const { createLLM } = await import('@mcp-rebuild/llm');
        const llmConfig = await resolveLLMConfig(input.model);
        const llm = createLLM(llmConfig);
        const model = input.model || llmConfig?.defaultModel;
        const result = await llm.complete(input.message, {
          model,
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
        const llmConfig = await resolveLLMConfig();
        const rawLlm = createLLM(llmConfig);
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
      const envProviders: Array<Record<string, unknown>> = [];
      const envProvider = process.env.LLM_PROVIDER;
      const envBaseUrl = process.env.LLM_BASE_URL;
      const envModel = process.env.LLM_MODEL;
      if (envProvider) {
        envProviders.push({
          name: envProvider,
          type: envProvider,
          baseUrl: envBaseUrl ?? null,
          models: envModel ? [envModel] : [],
          status: 'available',
          circuitState: 'closed',
          source: 'env',
        });
      }
      if (dbReady && drizzleDb && schemaTables) {
        try {
          const rows = await drizzleDb.select().from(schemaTables.llmProviderConfigs)
            .orderBy(desc(schemaTables.llmProviderConfigs.priority));
          const dbProviders = rows.map((r: Record<string, unknown>) => ({
            name: r.providerName,
            type: r.providerName,
            baseUrl: r.baseUrl,
            models: r.models ?? [],
            status: 'available',
            circuitState: 'closed',
            isDefault: r.isDefault,
            priority: r.priority,
            source: 'db',
          }));
          return {
            providers: [...dbProviders, ...envProviders.filter((ep) => !dbProviders.some((db: Record<string, unknown>) => db.name === ep.name))],
            env: { LLM_PROVIDER: envProvider ?? null, LLM_BASE_URL: envBaseUrl ?? null, LLM_MODEL: envModel ?? null },
          };
        } catch {
          // fall through to env-only
        }
      }
      return {
        providers: envProviders.length > 0 ? envProviders : [
          { name: 'anthropic', type: 'anthropic', models: [], status: 'available' as const, circuitState: 'closed' as const },
          { name: 'openai', type: 'openai', models: [], status: 'available' as const, circuitState: 'closed' as const },
          { name: 'custom', type: 'custom', models: [], status: 'available' as const, circuitState: 'closed' as const },
        ],
        env: { LLM_PROVIDER: envProvider ?? null, LLM_BASE_URL: envBaseUrl ?? null, LLM_MODEL: envModel ?? null },
      };
    },
    testProvider: async (name, input) => {
      // Drizzle-first: look up provider config from DB
      if (dbReady && drizzleDb && schemaTables) {
        try {
          const [config] = await drizzleDb.select().from(schemaTables.llmProviderConfigs)
            .where(eq(schemaTables.llmProviderConfigs.providerName, name)).limit(1);
          if (config) {
            return { ok: true, provider: name, configured: true, models: config.models, input };
          }
        } catch {
          // fall through
        }
      }
      return { ok: false, provider: name, message: 'Provider test not implemented in API bootstrap', input };
    },
    saveProvider: async (input) => {
      if (!dbReady || !drizzleDb || !schemaTables) {
        return { ok: false, error: 'Database not available' };
      }
      const modelsArr = input.models.filter((m: string) => m.trim() !== '');
      try {
        await drizzleDb.insert(schemaTables.llmProviderConfigs)
          .values({
            providerName: input.providerName,
            baseUrl: input.baseUrl,
            apiKeyEnvVar: input.apiKey || '',
            models: modelsArr,
            isDefault: input.isDefault ?? false,
            priority: 0,
          })
          .onConflictDoUpdate({
            target: schemaTables.llmProviderConfigs.providerName,
            set: {
              baseUrl: input.baseUrl,
              apiKeyEnvVar: input.apiKey || '',
              models: modelsArr,
              isDefault: input.isDefault ?? false,
              updatedAt: new Date(),
            },
          });
        return { ok: true, provider: input.providerName };
      } catch (err: unknown) {
        return { ok: false, error: err instanceof Error ? err.message : 'Failed to save provider' };
      }
    },
    deleteProvider: async (providerName) => {
      if (!dbReady || !drizzleDb || !schemaTables) {
        return { ok: false, error: 'Database not available' };
      }
      try {
        await drizzleDb.delete(schemaTables.llmProviderConfigs)
          .where(eq(schemaTables.llmProviderConfigs.providerName, providerName));
        return { ok: true, deleted: providerName };
      } catch (err: unknown) {
        return { ok: false, error: err instanceof Error ? err.message : 'Failed to delete provider' };
      }
    },
    setDefaultProvider: async (providerName) => {
      if (!dbReady || !drizzleDb || !schemaTables) {
        return { ok: false, error: 'Database not available' };
      }
      try {
        await drizzleDb.update(schemaTables.llmProviderConfigs)
          .set({ isDefault: false, updatedAt: new Date() });
        await drizzleDb.update(schemaTables.llmProviderConfigs)
          .set({ isDefault: true, updatedAt: new Date() })
          .where(eq(schemaTables.llmProviderConfigs.providerName, providerName));
        return { ok: true, default: providerName };
      } catch (err: unknown) {
        return { ok: false, error: err instanceof Error ? err.message : 'Failed to set default' };
      }
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
  healthChecker.registerCheck('postgresql', async () => {
    if (!dbReady || !drizzleDb || !schemaTables) {
      return { name: 'postgresql', status: 'warn' as const, message: 'Not connected', duration: 0 };
    }
    try {
      const start = Date.now();
      await drizzleDb.select({ value: sql`1` }).from(schemaTables.workflows).limit(1);
      return { name: 'postgresql', status: 'pass' as const, message: 'PostgreSQL responsive', duration: Date.now() - start };
    } catch (err) {
      return { name: 'postgresql', status: 'fail' as const, message: `PostgreSQL error: ${String(err)}`, duration: 0 };
    }
  });
  healthChecker.start();

  // Avoid circular construction dependency: monitoring provider needs APIServer stats.
  let apiServer: APIServer | null = null;

  const monitoringProvider: MonitoringServiceProvider = {
    getHealth: async () => healthChecker.check(),
    getMetrics: () => ({ points: getMetrics().getPoints() }),
    getStats: async () => {
      // Drizzle-first: get real counts from PostgreSQL
      if (dbReady && drizzleDb && schemaTables) {
        try {
          const [{ value: wfTotal }] = await drizzleDb.select({ value: count() }).from(schemaTables.workflows);
          const [{ value: taskTotal }] = await drizzleDb.select({ value: count() }).from(schemaTables.tasks);
          const [{ value: memTotal }] = await drizzleDb.select({ value: count() }).from(schemaTables.memories);
          const wfDoneRows = await drizzleDb.select({ value: count() }).from(schemaTables.workflows)
            .where(eq(schemaTables.workflows.status, 'DONE'));
          const wfFailedRows = await drizzleDb.select({ value: count() }).from(schemaTables.workflows)
            .where(eq(schemaTables.workflows.status, 'FAILED'));
          const taskDoneRows = await drizzleDb.select({ value: count() }).from(schemaTables.tasks)
            .where(eq(schemaTables.tasks.status, 'DONE'));
          const taskFailedRows = await drizzleDb.select({ value: count() }).from(schemaTables.tasks)
            .where(eq(schemaTables.tasks.status, 'FAILED'));
          const workflowsDone = Number(wfDoneRows[0]?.value ?? 0);
          const workflowsFailed = Number(wfFailedRows[0]?.value ?? 0);
          const workflowsActive = Number(wfTotal) - workflowsDone - workflowsFailed;
          return {
            engine: {
              workflows: Number(wfTotal),
              workflowsActive,
              workflowsDone,
              workflowsFailed,
              tasksTotal: Number(taskTotal),
              tasksCompleted: Number(taskDoneRows[0]?.value ?? 0),
              tasksFailed: Number(taskFailedRows[0]?.value ?? 0),
              memoriesTotal: Number(memTotal),
              tokensUsed: getMetrics().getCounter('llm.tokens_used').sum,
              tokenBreakdown: Object.fromEntries(getRouteTokenBreakdown()),
              source: 'postgresql',
            },
            api: apiServer ? apiServer.getStats() : { started: false },
          };
        } catch {
          // fall through to engine fallback
        }
      }
      const allWorkflows = engine.listWorkflows();
      const workflowsDone = allWorkflows.filter((w) => w.state === 'DONE').length;
      const workflowsFailed = allWorkflows.filter((w) => w.state === 'FAILED').length;
      const workflowsActive = allWorkflows.length - workflowsDone - workflowsFailed;
      const allTasks = allWorkflows.flatMap((w) => w.tasks ?? []);
      const tasksCompleted = allTasks.filter((t) => t.state === 'done').length;
      const tasksFailed = allTasks.filter((t) => t.state === 'failed').length;
      const allMemories = memoryStore.getAll();
      return {
        engine: {
          workflows: allWorkflows.length,
          workflowsActive,
          workflowsDone,
          workflowsFailed,
          tasksTotal: allTasks.length,
          tasksCompleted,
          tasksFailed,
          memoriesTotal: allMemories.length,
          tokensUsed: getMetrics().getCounter('llm.tokens_used').sum,
          tokenBreakdown: Object.fromEntries(getRouteTokenBreakdown()),
          source: 'in-memory',
        },
        api: apiServer ? apiServer.getStats() : { started: false },
      };
    },
    getTokenUsage: async (params) => {
      // Drizzle-first: query TokenUsage table directly
      if (dbReady && drizzleDb && schemaTables) {
        try {
          const conditions = [];
          if (params.from) conditions.push(sql`${schemaTables.tokenUsages.createdAt} >= ${new Date(params.from)}`);
          if (params.to) conditions.push(sql`${schemaTables.tokenUsages.createdAt} <= ${new Date(params.to)}`);
          if (params.route) conditions.push(eq(schemaTables.tokenUsages.route, params.route));
          if (params.model) conditions.push(eq(schemaTables.tokenUsages.model, params.model));

          const whereClause = conditions.length > 0 ? and(...conditions) : undefined;
          const records = await drizzleDb.select().from(schemaTables.tokenUsages)
            .where(whereClause)
            .orderBy(desc(schemaTables.tokenUsages.createdAt))
            .limit(1000);

          const groupBy = params.groupBy ?? 'route';

          if (groupBy === 'model') {
            const buckets = new Map<string, { totalTokens: number; promptTokens: number; completionTokens: number; count: number }>();
            for (const r of records) {
              const key = r.model ?? 'unknown';
              const b = buckets.get(key) ?? { totalTokens: 0, promptTokens: 0, completionTokens: 0, count: 0 };
              b.totalTokens += r.totalTokens;
              b.promptTokens += r.promptTokens;
              b.completionTokens += r.completionTokens;
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
              b.totalTokens += r.totalTokens;
              b.promptTokens += r.promptTokens;
              b.completionTokens += r.completionTokens;
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
            b.totalTokens += r.totalTokens;
            b.promptTokens += r.promptTokens;
            b.completionTokens += r.completionTokens;
            b.count += 1;
            buckets.set(key, b);
          }
          return { groupBy: 'route', buckets: Object.fromEntries(buckets), totalRecords: records.length };
        } catch (err) {
          logger.warn({ err: String(err) }, 'Token usage aggregation from Drizzle failed');
          return { error: 'Token usage aggregation unavailable', buckets: {}, totalRecords: 0 };
        }
      }
      return { error: 'Token usage aggregation unavailable (no DB)', buckets: {}, totalRecords: 0 };
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
