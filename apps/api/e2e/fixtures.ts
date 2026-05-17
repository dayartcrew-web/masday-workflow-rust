import { test as base, expect } from '@playwright/test';
import { APIServer } from '../src/index';
import { EventBus, HealthChecker, getMetrics } from '@mcp-rebuild/core';
import type { MemoryType } from '@mcp-rebuild/core';
import { OrchestratingEngine } from '@mcp-rebuild/workflow-engine';
import type { ISkillRegistry } from '@mcp-rebuild/workflow-engine';
import { JsonBackend, WorkflowStore, TaskResultStore, PersistenceListener } from '@mcp-rebuild/store';
import { MemoryStore } from '@mcp-rebuild/memory';
import * as policyTools from '@mcp-rebuild/policy';
import * as capabilityTools from '@mcp-rebuild/capability';
import type {
  MemoryServiceProvider, SearchServiceProvider, PolicyServiceProvider,
  CapabilityServiceProvider, ChatServiceProvider, ProviderServiceProvider,
  MonitoringServiceProvider,
} from '../src/routes';
import os from 'os';
import path from 'path';
import fs from 'fs';

const TEST_PORT = parseInt(process.env.API_TEST_PORT || '3099', 10);

function isMemoryType(value: string): value is MemoryType {
  return ['fact', 'preference', 'skill', 'experience', 'strategy', 'decision', 'artifact', 'learning', 'blocker'].includes(value);
}

type ApiFixtures = {
  authToken: string;
  authedRequest: (options: { method: string; path: string; body?: unknown }) => Promise<{ status: number; body: unknown }>;
};

export const test = base.extend<ApiFixtures>({
  authToken: async ({ request }, use) => {
    const res = await request.post('/api/auth/login', {
      data: { email: 'e2e@test.com', name: 'E2E Tester' },
    });
    const body = await res.json();
    await use(body.token);
  },

  authedRequest: async ({ request, authToken }, use) => {
    const makeRequest = async ({ method, path, body }: { method: string; path: string; body?: unknown }) => {
      const opts: Record<string, unknown> = {
        headers: { Authorization: `Bearer ${authToken}` },
      };
      if (body !== undefined) opts.data = body;
      const res = await request.fetch(path, { method, ...opts });
      const resBody = await res.json().catch(() => null);
      return { status: res.status(), body: resBody };
    };
    await use(makeRequest);
  },
});

let serverInstance: APIServer | null = null;

export async function startTestServer(): Promise<void> {
  const eventBus = new EventBus();
  const skillRegistry: ISkillRegistry = {
    async execute(skill: string, input: unknown): Promise<unknown> { throw new Error("Not available: " + skill); },
    has(skill: string): boolean { return false; },
    getAll(): Array<{ name: string; description: string }> { return []; },
  };

  const tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), 'e2e-api-'));
  const backend = new JsonBackend(path.join(tmpDir, 'store.json'));
  backend.initialize();

  const workflowStore = new WorkflowStore(backend);
  const taskResultStore = new TaskResultStore(backend);
  new PersistenceListener(eventBus, workflowStore, taskResultStore).start();

  const engine = new OrchestratingEngine(skillRegistry, eventBus, {
    coordinator: false, enableSkillRouting: false, store: workflowStore,
  });

  const memoryStore = new MemoryStore({ filePath: path.join(tmpDir, 'memories.json') });
  await memoryStore.init();

  const memoryProvider: MemoryServiceProvider = {
    store: async (entry) => {
      const type: MemoryType = isMemoryType(entry.memoryType) ? entry.memoryType : 'fact';
      const tags = [...(entry.workflowId ? [entry.workflowId] : []), ...(entry.taskId ? [entry.taskId] : [])];
      const record = await memoryStore.add(entry.content, { type, importance: entry.importance, tags, source: 'e2e' });
      await memoryStore.save();
      return { id: record.id };
    },
    storeResearch: async (entry) => {
      const record = await memoryStore.add(entry.findings, { type: 'fact', importance: 0.6, tags: ['research', entry.workflowId], source: 'e2e' });
      await memoryStore.save();
      return { id: record.id };
    },
    recallDocuments: async (workflowId, limit) => (await memoryStore.search(workflowId, { limit: limit ?? 10, threshold: 0.0 })).map(r => r.memory),
    recallRecent: async (workflowId, limit) => memoryStore.getAll().filter(m => m.tags.includes(workflowId)).sort((a, b) => b.createdAt - a.createdAt).slice(0, limit ?? 10),
    recallByType: async (workflowId, type, limit) => memoryStore.getAll().filter(m => m.tags.includes(workflowId) && m.type === type).sort((a, b) => b.createdAt - a.createdAt).slice(0, limit ?? 10),
    recallByTask: async (taskId, limit) => memoryStore.getAll().filter(m => m.tags.includes(taskId)).sort((a, b) => b.createdAt - a.createdAt).slice(0, limit ?? 10),
    update: async (id, updates) => {
      const updated = memoryStore.update(id, { content: typeof updates.content === 'string' ? updates.content : undefined });
      if (!updated) return { updated: false };
      await memoryStore.save();
      return { updated: true };
    },
    delete: async (id) => { const d = memoryStore.delete(id); if (d) await memoryStore.save(); return { deleted: d }; },
  };

  const searchProvider: SearchServiceProvider = {
    hybridContextPack: async (input) => ({ workflowId: input.workflowId, planId: input.planId, taskId: input.taskId, fingerprint: 'test', contextSufficient: true, semanticMemories: [], semanticDocs: [] }),
    contextFingerprint: async (input) => ({ fingerprint: `${input.workflowId}-${input.planId}-${input.taskId}`, contextSufficient: true }),
    codeSearch: async (input) => ({ query: input.query, results: [], limit: input.limit ?? 10 }),
  };

  const policyProvider: PolicyServiceProvider = {
    checkReadiness: (sessionKey) => policyTools.checkSessionReadiness(backend, { sessionKey }),
    validateExecution: (input) => policyTools.validateExecution(backend, input),
    validateCompletion: async (input) => policyTools.validateCompletion(backend, { workflowId: input.workflowId, taskId: input.taskId, acceptanceCriteria: input.acceptanceCriteria, outputText: input.evidence?.join('\n') }),
    validateParallel: async (input) => ({ ok: true, workflowId: input.workflowId, branchCount: input.branchResults.length }),
    detectDrift: async (input) => policyTools.detectScopeDriftTool(backend, { taskTitle: input.originalScope, acceptanceCriteria: [], requiredContext: [], outputText: input.currentInput }),
    requireContextRefresh: async (input) => ({ refreshRequired: false, reason: 'test', currentFingerprint: `${input.workflowId}-${input.planId}-${input.taskId}` }),
    auditWorkflow: async (workflowId) => new policyTools.WorkflowAuditor(backend).audit(workflowId),
  };

  const capabilityProvider: CapabilityServiceProvider = {
    createAgent: async (input) => capabilityTools.createAgentTool(backend, { projectRoot: input.projectRoot, name: input.name, role: input.role, description: input.description ?? '', instructions: '' }),
    createSkill: async (input) => capabilityTools.createSkillTool(backend, { projectRoot: input.projectRoot, name: input.name, description: input.description ?? '', trigger: 'manual', steps: [] }),
    listAgents: async (projectRoot) => capabilityTools.listAgentsTool(backend, { projectRoot }),
    matchAgent: async (input) => capabilityTools.matchAgentTool(backend, { projectRoot: input.projectRoot, taskDescription: input.taskType }),
    listSkills: async (projectRoot) => capabilityTools.listSkillsTool(backend, { projectRoot }),
    listTemplates: async () => capabilityTools.listTemplatesTool(backend),
    checkReadiness: async (projectRoot) => capabilityTools.systemReadinessTool(backend, projectRoot),
    auditWorkflow: async (workflowId) => capabilityTools.workflowAuditTool(backend, { workflowId: workflowId || undefined }),
  };

  const chatProvider: ChatServiceProvider = {
    complete: async () => ({ error: 'LLM not configured for E2E', ok: false }),
    react: async () => ({ ok: false, error: 'ReAct not available in E2E' }),
  };

  const providerService: ProviderServiceProvider = {
    listProviders: async () => ({ providers: ['anthropic', 'openai', 'custom'], env: { LLM_PROVIDER: null } }),
    testProvider: async (name) => ({ ok: false, provider: name, message: 'Not available in E2E' }),
  };

  const healthChecker = new HealthChecker(eventBus, { checkInterval: 60000 });
  healthChecker.start();

  let apiServer: APIServer | null = null;
  const monitoringProvider: MonitoringServiceProvider = {
    getHealth: async () => healthChecker.check(),
    getMetrics: () => ({ points: getMetrics().getPoints() }),
    getStats: () => ({ engine: { workflows: engine.listWorkflows().length }, api: apiServer ? apiServer.getStats() : { started: false } }),
  };

  apiServer = new APIServer(
    { eventBus, engine, memoryProvider, searchProvider, policyProvider, capabilityProvider, chatProvider, providerService, monitoringProvider },
    { httpPort: TEST_PORT, wsPort: TEST_PORT + 1 },
  );

  await apiServer.start();
  serverInstance = apiServer;
}

export async function stopTestServer(): Promise<void> {
  if (serverInstance) {
    await serverInstance.stop();
    serverInstance = null;
  }
}

export { expect };
