import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { authApi, workflowApi, memoryApi, monitoringApi, providerApi, searchApi, policyApi, wsClient, WebSocketClient } from '@/lib/api-client';

// Mock fetch at the global level
const mockFetch = vi.fn();
vi.stubGlobal('fetch', mockFetch);

describe('authApi', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('login sends credentials', async () => {
    mockFetch.mockResolvedValue({
      ok: true,
      status: 200,
      headers: { get: () => 'application/json' },
      text: () => Promise.resolve(JSON.stringify({ token: 'abc123', user: { id: '1' } })),
    });

    const result = await authApi.login('user@test.com', 'Test User');

    expect(mockFetch).toHaveBeenCalledWith(
      expect.stringContaining('/api/auth/login'),
      expect.objectContaining({
        method: 'POST',
        body: JSON.stringify({ email: 'user@test.com', name: 'Test User' }),
      }),
    );
    expect(result).toEqual({ token: 'abc123', user: { id: '1' } });
  });

  it('getMe fetches current user', async () => {
    mockFetch.mockResolvedValue({
      ok: true,
      status: 200,
      headers: { get: () => 'application/json' },
      text: () => Promise.resolve(JSON.stringify({ user: { id: '1', email: 'user@test.com' } })),
    });

    await authApi.getMe();

    expect(mockFetch).toHaveBeenCalledWith(
      expect.stringContaining('/api/auth/me'),
      expect.any(Object),
    );
  });
});

describe('workflowApi', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('list fetches all workflows', async () => {
    mockFetch.mockResolvedValue({
      ok: true,
      status: 200,
      headers: { get: () => 'application/json' },
      text: () => Promise.resolve(JSON.stringify({ workflows: [{ id: '1' }, { id: '2' }] })),
    });

    const result = await workflowApi.list();

    expect(mockFetch).toHaveBeenCalledWith(
      expect.stringContaining('/api/workflows'),
      expect.objectContaining({ method: 'GET' }),
    );
    expect(result).toEqual({ workflows: [{ id: '1' }, { id: '2' }] });
  });

  it('get fetches single workflow', async () => {
    mockFetch.mockResolvedValue({
      ok: true,
      status: 200,
      headers: { get: () => 'application/json' },
      text: () => Promise.resolve(JSON.stringify({ workflow: { id: '1', name: 'Test' } })),
    });

    const result = await workflowApi.get('1');

    expect(result).toEqual({ workflow: { id: '1', name: 'Test' } });
  });

  it('create sends POST', async () => {
    mockFetch.mockResolvedValue({
      ok: true,
      status: 200,
      headers: { get: () => 'application/json' },
      text: () => Promise.resolve(JSON.stringify({ workflow: { id: 'new' } })),
    });

    await workflowApi.create('New Workflow', 'Description');

    expect(mockFetch).toHaveBeenCalledWith(
      expect.stringContaining('/api/workflows'),
      expect.objectContaining({
        method: 'POST',
        body: JSON.stringify({ name: 'New Workflow', description: 'Description', metadata: undefined }),
      }),
    );
  });

  it('execute triggers workflow execution', async () => {
    mockFetch.mockResolvedValue({
      ok: true,
      status: 200,
      headers: { get: () => 'application/json' },
      text: () => Promise.resolve(JSON.stringify({ workflow: { id: '1', status: 'running' } })),
    });

    await workflowApi.execute('1');

    expect(mockFetch).toHaveBeenCalledWith(
      expect.stringContaining('/api/workflows/1/execute'),
      expect.objectContaining({ method: 'POST' }),
    );
  });

  it('getTasks fetches tasks for a workflow', async () => {
    mockFetch.mockResolvedValue({
      ok: true,
      status: 200,
      headers: { get: () => 'application/json' },
      text: () => Promise.resolve(JSON.stringify({ tasks: [{ id: 't1' }] })),
    });

    const result = await workflowApi.getTasks('wf-1');

    expect(result).toEqual({ tasks: [{ id: 't1' }] });
  });
});

describe('memoryApi', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('store sends POST', async () => {
    mockFetch.mockResolvedValue({
      ok: true,
      status: 200,
      headers: { get: () => 'application/json' },
      text: () => Promise.resolve(JSON.stringify({ id: 'new-memory' })),
    });

    await memoryApi.store({ memoryType: 'note', summary: 'test', content: 'test content' });

    expect(mockFetch).toHaveBeenCalledWith(
      expect.stringContaining('/api/memory'),
      expect.objectContaining({
        method: 'POST',
        body: JSON.stringify({ memoryType: 'note', summary: 'test', content: 'test content' }),
      }),
    );
  });

  it('recallDocuments fetches documents', async () => {
    mockFetch.mockResolvedValue({
      ok: true,
      status: 200,
      headers: { get: () => 'application/json' },
      text: () => Promise.resolve(JSON.stringify({ documents: [{ id: '1' }] })),
    });

    const result = await memoryApi.recallDocuments('wf-1');

    expect(result).toEqual({ documents: [{ id: '1' }] });
  });

  it('delete sends DELETE', async () => {
    mockFetch.mockResolvedValue({
      ok: true,
      status: 200,
      headers: { get: () => 'application/json' },
      text: () => Promise.resolve(JSON.stringify({ deleted: true })),
    });

    await memoryApi.delete('1');

    expect(mockFetch).toHaveBeenCalledWith(
      expect.stringContaining('/api/memory/1'),
      expect.objectContaining({ method: 'DELETE' }),
    );
  });
});

describe('monitoringApi', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('getMetrics fetches metrics', async () => {
    mockFetch.mockResolvedValue({
      ok: true,
      status: 200,
      headers: { get: () => 'application/json' },
      text: () => Promise.resolve(JSON.stringify({ engine: { workflows: 10, workflowsActive: 5 } })),
    });

    const result = await monitoringApi.getMetrics();

    expect(result.workflowsTotal).toBe(10);
    expect(result.workflowsActive).toBe(5);
  });

  it('getStats fetches system stats', async () => {
    mockFetch.mockResolvedValue({
      ok: true,
      status: 200,
      headers: { get: () => 'application/json' },
      text: () => Promise.resolve(JSON.stringify({ api: { requestsTotal: 100, avgLatencyMs: 50 } })),
    });

    const result = await monitoringApi.getStats();

    expect(result.requestsTotal).toBe(100);
    expect(result.avgLatencyMs).toBe(50);
  });

  it('getHealth fetches health status', async () => {
    mockFetch.mockResolvedValue({
      ok: true,
      status: 200,
      headers: { get: () => 'application/json' },
      text: () => Promise.resolve(JSON.stringify({ status: 'healthy', uptimeMs: 1000, version: '1.0.0' })),
    });

    const result = await monitoringApi.getHealth();

    expect(result.status).toBe('healthy');
  });
});

describe('providerApi', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('list fetches providers', async () => {
    mockFetch.mockResolvedValue({
      ok: true,
      status: 200,
      headers: { get: () => 'application/json' },
      text: () => Promise.resolve(JSON.stringify({ providers: ['openai', 'anthropic'] })),
    });

    const result = await providerApi.list();

    expect(result.providers).toHaveLength(2);
  });

  it('test sends provider test request', async () => {
    mockFetch.mockResolvedValue({
      ok: true,
      status: 200,
      headers: { get: () => 'application/json' },
      text: () => Promise.resolve(JSON.stringify({ status: 'success' })),
    });

    await providerApi.test('openai', { model: 'gpt-4' });

    expect(mockFetch).toHaveBeenCalledWith(
      expect.stringContaining('/api/providers/openai/test'),
      expect.objectContaining({
        method: 'POST',
        body: JSON.stringify({ model: 'gpt-4' }),
      }),
    );
  });
});

describe('searchApi', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('codeSearch sends search query', async () => {
    mockFetch.mockResolvedValue({
      ok: true,
      status: 200,
      headers: { get: () => 'application/json' },
      text: () => Promise.resolve(JSON.stringify({ results: [{ id: '1' }] })),
    });

    await searchApi.codeSearch({ query: 'test query' });

    expect(mockFetch).toHaveBeenCalledWith(
      expect.stringContaining('/api/search/code'),
      expect.objectContaining({
        method: 'POST',
        body: JSON.stringify({ query: 'test query' }),
      }),
    );
  });
});

describe('WebSocketClient', () => {
  let mockWs: Partial<WebSocket>;

  beforeEach(() => {
    vi.clearAllMocks();
    mockWs = {
      send: vi.fn(),
      close: vi.fn(),
      readyState: 1,
    };
    const MockWebSocket = vi.fn(() => mockWs) as unknown as typeof WebSocket;
    vi.stubGlobal('WebSocket', MockWebSocket);
  });

  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it('connect creates WebSocket connection', () => {
    const client = new WebSocketClient();
    client.connect('test-token');

    expect(WebSocket).toHaveBeenCalledWith(expect.stringContaining('token=test-token'));
  });

  it('disconnect closes the connection', () => {
    const client = new WebSocketClient();
    client.connect('test-token');
    client.disconnect();

    expect(mockWs.close).toHaveBeenCalled();
    expect(client.connected).toBe(false);
  });

  it('onEvent registers handler and returns unsubscribe function', () => {
    const client = new WebSocketClient();
    const handler = vi.fn();
    const unsubscribe = client.onEvent(handler);

    expect(typeof unsubscribe).toBe('function');
    unsubscribe();
  });
});
