// ============================================================
// Tests for APIServer integration — HTTP endpoints
// ============================================================

import { describe, it, expect, beforeAll, afterAll, vi } from 'vitest';
import { createUser, signToken } from '../auth/jwt';
import { APIServer } from '../index';
import { EventBus } from '@mcp-rebuild/core';
import http from 'http';

/** Make an HTTP request and return response */
function makeRequest(
  port: number,
  options: http.RequestOptions,
  body?: string,
): Promise<{ status: number; body: Record<string, unknown> }> {
  return new Promise((resolve, reject) => {
    const req = http.request({ ...options, port }, (res) => {
      const chunks: Buffer[] = [];
      res.on('data', (chunk: Buffer) => chunks.push(chunk));
      res.on('end', () => {
        const responseBody = Buffer.concat(chunks).toString();
        let parsed: Record<string, unknown>;
        try {
          parsed = JSON.parse(responseBody);
        } catch {
          parsed = { raw: responseBody };
        }
        resolve({ status: res.statusCode || 0, body: parsed });
      });
    });
    req.on('error', reject);
    if (body) req.write(body);
    req.end();
  });
}

describe('APIServer HTTP Integration', () => {
  const TEST_PORT = 3098;
  let server: APIServer;
  let authToken: string;
  let eventBus: EventBus;

  beforeAll(async () => {
    eventBus = new EventBus();

    const user = createUser({ email: 'server-test@example.com', name: 'Server Test' });
    authToken = signToken({ userId: user.id, email: user.email, role: user.role });

    const mockProvider = {
      store: vi.fn(async () => ({ id: 'mem_1' })),
      storeResearch: vi.fn(async () => ({ id: 'res_1' })),
      recallDocuments: vi.fn(async () => []),
      recallRecent: vi.fn(async () => []),
      recallByType: vi.fn(async () => []),
      recallByTask: vi.fn(async () => []),
      update: vi.fn(async () => ({ updated: true })),
      delete: vi.fn(async () => ({ deleted: true })),
      hybridContextPack: vi.fn(async () => ({ pack: {} })),
      contextFingerprint: vi.fn(async () => ({ fingerprint: 'abc123' })),
      codeSearch: vi.fn(async () => ({ results: [] })),
      checkReadiness: vi.fn(async () => ({ ready: true })),
      validateExecution: vi.fn(async () => ({ valid: true })),
      validateCompletion: vi.fn(async () => ({ valid: true })),
      validateParallel: vi.fn(async () => ({ valid: true })),
      detectDrift: vi.fn(async () => ({ driftDetected: false })),
      requireContextRefresh: vi.fn(async () => ({ refreshRequired: false })),
      auditWorkflow: vi.fn(async () => ({ issues: [] })),
      createAgent: vi.fn(async () => ({ created: true })),
      createSkill: vi.fn(async () => ({ created: true })),
      listAgents: vi.fn(async () => ({ agents: [] })),
      matchAgent: vi.fn(async () => ({ matched: null })),
      listSkills: vi.fn(async () => ({ skills: [] })),
      listTemplates: vi.fn(async () => ({ templates: [] })),
      checkSystemReadiness: vi.fn(async () => ({ ready: true })),
      workflowAudit: vi.fn(async () => ({ issues: [] })),
      complete: vi.fn(async () => ({ response: 'ok' })),
      react: vi.fn(async () => ({ result: 'done' })),
      listProviders: vi.fn(async () => ({ providers: [] })),
      testProvider: vi.fn(async () => ({ success: true })),
      getHealth: vi.fn(async () => ({ status: 'ok' })),
      getMetrics: vi.fn(() => ({ requests: 0 })),
      getStats: vi.fn(() => ({ uptime: 0 })),
    };

    const mockEngine = {
      listWorkflows: vi.fn(() => []),
      createWorkflow: vi.fn(() => ({ id: 'wf_new', name: 'Test', state: 'INIT', tasks: [], description: '', metadata: {}, traceId: 't1', createdAt: new Date(), updatedAt: new Date() })),
      getWorkflow: vi.fn(() => null),
      getStatus: vi.fn(() => null),
      executeWorkflow: vi.fn(async () => ({ id: 'wf_new', state: 'DONE', tasks: [] })),
      addTask: vi.fn(() => ({ id: 'task_1', name: 'Test', state: 'pending' })),
    } as any;

    server = new APIServer(
      {
        eventBus,
        engine: mockEngine,
        memoryProvider: mockProvider as any,
        searchProvider: mockProvider as any,
        policyProvider: mockProvider as any,
        capabilityProvider: mockProvider as any,
        chatProvider: mockProvider as any,
        providerService: mockProvider as any,
        monitoringProvider: mockProvider as any,
      },
      {
        httpPort: TEST_PORT,
        wsPort: 3097,
        rateLimitWindow: 60000,
        rateLimitMax: 1000,
      },
    );

    await server.start();
  });

  afterAll(async () => {
    await server.stop();
  });

  // --- CORS ---

  it('should handle CORS preflight', async () => {
    const res = await makeRequest(TEST_PORT, {
      method: 'OPTIONS',
      path: '/api/health',
      headers: { 'Origin': 'http://localhost:3000' },
    });
    expect(res.status).toBe(204);
  });

  // --- Health (no auth required) ---

  it('GET /api/health should return health status', async () => {
    const res = await makeRequest(TEST_PORT, {
      method: 'GET',
      path: '/api/health',
    });
    expect(res.status).toBe(200);
    expect(res.body.status).toBe('ok');
  });

  // --- 404 ---

  it('should return 404 for unknown routes', async () => {
    const res = await makeRequest(TEST_PORT, {
      method: 'GET',
      path: '/api/nonexistent',
    });
    expect(res.status).toBe(404);
    expect(res.body.code).toBe('NOT_FOUND');
  });

  // --- Auth routes ---

  it('POST /api/auth/login should create user and return token', async () => {
    const res = await makeRequest(TEST_PORT, {
      method: 'POST',
      path: '/api/auth/login',
      headers: { 'Content-Type': 'application/json' },
    }, JSON.stringify({ email: 'login-test@example.com', name: 'Login Test' }));

    expect(res.status).toBe(200);
    expect(res.body.token).toBeTruthy();
    expect(res.body.user).toBeTruthy();
    expect((res.body.user as Record<string, unknown>).email).toBe('login-test@example.com');
  });

  it('POST /api/auth/login should return existing user', async () => {
    // Create user first
    const res1 = await makeRequest(TEST_PORT, {
      method: 'POST',
      path: '/api/auth/login',
      headers: { 'Content-Type': 'application/json' },
    }, JSON.stringify({ email: 'returning@example.com', name: 'Returning' }));

    const res2 = await makeRequest(TEST_PORT, {
      method: 'POST',
      path: '/api/auth/login',
      headers: { 'Content-Type': 'application/json' },
    }, JSON.stringify({ email: 'returning@example.com', name: 'Returning' }));

    expect(res2.status).toBe(200);
    expect((res1.body.user as Record<string, unknown>).id).toBe(
      (res2.body.user as Record<string, unknown>).id,
    );
  });

  it('POST /api/auth/token should verify a token', async () => {
    const res = await makeRequest(TEST_PORT, {
      method: 'POST',
      path: '/api/auth/token',
      headers: { 'Content-Type': 'application/json' },
    }, JSON.stringify({ token: authToken }));

    expect(res.status).toBe(200);
    expect(res.body.valid).toBe(true);
  });

  it('POST /api/auth/token should reject invalid token', async () => {
    const res = await makeRequest(TEST_PORT, {
      method: 'POST',
      path: '/api/auth/token',
      headers: { 'Content-Type': 'application/json' },
    }, JSON.stringify({ token: 'invalid' }));

    expect(res.status).toBe(200);
    expect(res.body.valid).toBe(false);
  });

  it('GET /api/auth/me should return user with valid token', async () => {
    const res = await makeRequest(TEST_PORT, {
      method: 'GET',
      path: '/api/auth/me',
      headers: { 'Authorization': `Bearer ${authToken}` },
    });

    expect(res.status).toBe(200);
    expect(res.body.user).toBeTruthy();
  });

  it('GET /api/auth/me should reject without token', async () => {
    const res = await makeRequest(TEST_PORT, {
      method: 'GET',
      path: '/api/auth/me',
    });

    expect(res.status).toBe(401);
  });

  // --- Protected routes ---

  it('GET /api/workflows should require auth', async () => {
    const res = await makeRequest(TEST_PORT, {
      method: 'GET',
      path: '/api/workflows',
    });
    expect(res.status).toBe(401);
  });

  it('GET /api/workflows should return workflows with auth', async () => {
    const res = await makeRequest(TEST_PORT, {
      method: 'GET',
      path: '/api/workflows',
      headers: { 'Authorization': `Bearer ${authToken}` },
    });
    expect(res.status).toBe(200);
    expect(res.body.workflows).toBeTruthy();
  });

  it('POST /api/workflows should create a workflow', async () => {
    const res = await makeRequest(TEST_PORT, {
      method: 'POST',
      path: '/api/workflows',
      headers: {
        'Authorization': `Bearer ${authToken}`,
        'Content-Type': 'application/json',
      },
    }, JSON.stringify({ name: 'Test Workflow', description: 'A test' }));

    expect(res.status).toBe(201);
    expect(res.body.workflow).toBeTruthy();
  });

  it('GET /api/workflows/active should return active workflow', async () => {
    const res = await makeRequest(TEST_PORT, {
      method: 'GET',
      path: '/api/workflows/active',
      headers: { 'Authorization': `Bearer ${authToken}` },
    });
    expect(res.status).toBe(200);
  });

  it('GET /api/workflows/:id should return 404 for missing workflow', async () => {
    const res = await makeRequest(TEST_PORT, {
      method: 'GET',
      path: '/api/workflows/wf_missing',
      headers: { 'Authorization': `Bearer ${authToken}` },
    });
    expect(res.status).toBe(404);
  });

  // --- Memory routes ---

  it('POST /api/memory should store a memory', async () => {
    const res = await makeRequest(TEST_PORT, {
      method: 'POST',
      path: '/api/memory',
      headers: {
        'Authorization': `Bearer ${authToken}`,
        'Content-Type': 'application/json',
      },
    }, JSON.stringify({
      memoryType: 'learning',
      summary: 'Test memory',
      content: 'Test content',
    }));

    expect(res.status).toBe(201);
    expect(res.body.id).toBeTruthy();
  });

  it('GET /api/memory/:workflowId should recall documents', async () => {
    const res = await makeRequest(TEST_PORT, {
      method: 'GET',
      path: '/api/memory/wf_test',
      headers: { 'Authorization': `Bearer ${authToken}` },
    });
    expect(res.status).toBe(200);
  });

  // --- Search routes ---

  it('POST /api/search/hybrid should require auth', async () => {
    const res = await makeRequest(TEST_PORT, {
      method: 'POST',
      path: '/api/search/hybrid',
      headers: { 'Content-Type': 'application/json' },
    }, JSON.stringify({ workflowId: 'w1', planId: 'p1', taskId: 't1' }));

    expect(res.status).toBe(401);
  });

  // --- Monitoring routes ---

  it('GET /api/metrics should require auth', async () => {
    const res = await makeRequest(TEST_PORT, {
      method: 'GET',
      path: '/api/metrics',
    });
    expect(res.status).toBe(401);
  });

  it('GET /api/stats should require auth', async () => {
    const res = await makeRequest(TEST_PORT, {
      method: 'GET',
      path: '/api/stats',
    });
    expect(res.status).toBe(401);
  });

  // --- Server stats ---

  it('should track server stats', () => {
    const stats = server.getStats();
    expect(stats.requestsTotal).toBeGreaterThan(0);
    expect(stats.routes).toBeGreaterThanOrEqual(40);
  });
});
