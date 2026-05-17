import { test, expect } from '../fixtures';

test.describe.serial('Memory API', () => {
  let memoryId: string;

  test('POST /api/memory — stores a memory', async ({ authedRequest }) => {
    const { status, body } = await authedRequest({
      method: 'POST',
      path: '/api/memory',
      body: {
        memoryType: 'decision',
        summary: 'Test',
        content: 'E2E test content',
        importance: 0.8,
        workflowId: 'wf-e2e',
      },
    });

    expect(status).toBe(201);
    expect(body).toHaveProperty('id');
    memoryId = (body as { id: string }).id;
  });

  test('POST /api/memory/research — stores research', async ({ authedRequest }) => {
    const { status, body } = await authedRequest({
      method: 'POST',
      path: '/api/memory/research',
      body: {
        workflowId: 'wf-e2e',
        query: 'test query',
        findings: 'test findings',
        sources: ['src1'],
      },
    });

    expect(status).toBe(201);
    expect(body).toHaveProperty('id');
  });

  test('GET /api/memory/:workflowId — recalls documents', async ({ authedRequest }) => {
    const { status, body } = await authedRequest({
      method: 'GET',
      path: '/api/memory/wf-e2e',
    });

    expect(status).toBe(200);
    expect(body).toHaveProperty('documents');
    expect(Array.isArray((body as { documents: unknown[] }).documents)).toBe(true);
  });

  test('GET /api/memory/:workflowId/recent — returns recent memories', async ({ authedRequest }) => {
    const { status, body } = await authedRequest({
      method: 'GET',
      path: '/api/memory/wf-e2e/recent?limit=5',
    });

    expect(status).toBe(200);
    expect(Array.isArray((body as { memories: unknown[] }).memories)).toBe(true);
  });

  test('GET /api/memory/:workflowId/by-type/decision — returns filtered memories', async ({ authedRequest }) => {
    const { status, body } = await authedRequest({
      method: 'GET',
      path: '/api/memory/wf-e2e/by-type/decision',
    });

    expect(status).toBe(200);
    expect(Array.isArray((body as { memories: unknown[] }).memories)).toBe(true);
  });

  test('PUT /api/memory/:id — updates a memory', async ({ authedRequest }) => {
    const { status, body } = await authedRequest({
      method: 'PUT',
      path: `/api/memory/${memoryId}`,
      body: { content: 'updated content' },
    });

    expect(status).toBe(200);
    expect(body).toEqual({ updated: true });
  });

  test('DELETE /api/memory/:id — deletes a memory', async ({ authedRequest }) => {
    const { status, body } = await authedRequest({
      method: 'DELETE',
      path: `/api/memory/${memoryId}`,
    });

    expect(status).toBe(200);
    expect(body).toEqual({ deleted: true });
  });
});
