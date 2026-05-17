import { test, expect } from '../fixtures';

test.describe.serial('Services API', () => {
  // --- Search endpoints ---

  test('POST /api/search/hybrid — returns hybrid context pack', async ({ authedRequest }) => {
    const { status, body } = await authedRequest({
      method: 'POST',
      path: '/api/search/hybrid',
      body: { workflowId: 'w1', planId: 'p1', taskId: 't1' },
    });

    expect(status).toBe(200);
    expect(body).toHaveProperty('workflowId');
    expect((body as { workflowId: string }).workflowId).toBe('w1');
  });

  test('POST /api/search/fingerprint — returns fingerprint', async ({ authedRequest }) => {
    const { status, body } = await authedRequest({
      method: 'POST',
      path: '/api/search/fingerprint',
      body: { workflowId: 'w1', planId: 'p1', taskId: 't1' },
    });

    expect(status).toBe(200);
    expect(body).toHaveProperty('fingerprint');
  });

  test('POST /api/search/code — returns results array', async ({ authedRequest }) => {
    const { status, body } = await authedRequest({
      method: 'POST',
      path: '/api/search/code',
      body: { query: 'test' },
    });

    expect(status).toBe(200);
    expect(body).toHaveProperty('results');
    expect(Array.isArray((body as { results: unknown[] }).results)).toBe(true);
  });

  // --- Chat endpoints ---

  test('POST /api/chat — returns response (error ok without LLM)', async ({ authedRequest }) => {
    const { status, body } = await authedRequest({
      method: 'POST',
      path: '/api/chat',
      body: { message: 'hello' },
    });

    expect(status).toBe(200);
    expect(body).toHaveProperty('error');
  });

  test('POST /api/chat/react — returns response', async ({ authedRequest }) => {
    const { status } = await authedRequest({
      method: 'POST',
      path: '/api/chat/react',
      body: { goal: 'test' },
    });

    expect(status).toBe(200);
  });

  // --- Providers ---

  test('GET /api/providers — returns providers array', async ({ authedRequest }) => {
    const { status, body } = await authedRequest({
      method: 'GET',
      path: '/api/providers',
    });

    expect(status).toBe(200);
    expect(body).toHaveProperty('providers');
    expect(Array.isArray((body as { providers: unknown[] }).providers)).toBe(true);
  });

  test('POST /api/providers/anthropic/test — tests provider connection', async ({ authedRequest }) => {
    const { status } = await authedRequest({
      method: 'POST',
      path: '/api/providers/anthropic/test',
    });

    expect(status).toBe(200);
  });
});
