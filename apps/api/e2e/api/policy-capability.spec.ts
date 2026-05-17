import { test, expect } from '../fixtures';

test.describe('Policy endpoints', () => {
  test('GET /api/policy/session/:key returns readiness result', async ({ authedRequest }) => {
    const { status, body } = await authedRequest({
      method: 'GET',
      path: '/api/policy/session/test-session',
    });
    expect(status).toBe(200);
    expect(body).toBeDefined();
  });

  test('POST /api/policy/validate/execution returns validation result', async ({ authedRequest }) => {
    const { status, body } = await authedRequest({
      method: 'POST',
      path: '/api/policy/validate/execution',
      body: { workflowId: 'wf-1', taskId: 't-1', sessionKey: 'sk-1' },
    });
    expect(status).toBe(200);
    expect(body).toMatchObject({ ok: false });
  });

  test('POST /api/policy/validate/completion returns result', async ({ authedRequest }) => {
    const { status, body } = await authedRequest({
      method: 'POST',
      path: '/api/policy/validate/completion',
      body: { workflowId: 'wf-1', taskId: 't-1', acceptanceCriteria: ['test'], evidence: [] },
    });
    expect(status).toBe(200);
    expect(body).toBeDefined();
  });

  test('POST /api/policy/validate/parallel returns ok', async ({ authedRequest }) => {
    const { status, body } = await authedRequest({
      method: 'POST',
      path: '/api/policy/validate/parallel',
      body: { workflowId: 'wf-1', branchResults: [{ branchKey: 'b1', status: 'done' }] },
    });
    expect(status).toBe(200);
    expect(body).toMatchObject({ ok: true });
  });

  test('POST /api/policy/drift returns drift result', async ({ authedRequest }) => {
    const { status, body } = await authedRequest({
      method: 'POST',
      path: '/api/policy/drift',
      body: { workflowId: 'wf-1', originalScope: 'build feature', currentInput: 'build feature' },
    });
    expect(status).toBe(200);
    expect(body).toBeDefined();
  });

  test('POST /api/policy/fingerprint returns fingerprint', async ({ authedRequest }) => {
    const { status, body } = await authedRequest({
      method: 'POST',
      path: '/api/policy/fingerprint',
      body: { workflowId: 'wf-1', planId: 'p-1', taskId: 't-1' },
    });
    expect(status).toBe(200);
    expect(body).toMatchObject({ currentFingerprint: 'wf-1-p-1-t-1' });
  });

  test('GET /api/policy/audit/:workflowId returns audit result', async ({ authedRequest }) => {
    const { status, body } = await authedRequest({
      method: 'GET',
      path: '/api/policy/audit/wf-nonexistent',
    });
    expect(status).toBe(200);
    expect(body).toBeDefined();
  });
});

test.describe('Capability endpoints', () => {
  test('POST /api/capability/agent creates agent', async ({ authedRequest }) => {
    const { status, body } = await authedRequest({
      method: 'POST',
      path: '/api/capability/agent',
      body: { name: 'test-agent', role: 'planner', projectRoot: '/tmp' },
    });
    expect(status).toBe(201);
    expect(body).toBeDefined();
  });

  test('POST /api/capability/skill creates skill', async ({ authedRequest }) => {
    const { status, body } = await authedRequest({
      method: 'POST',
      path: '/api/capability/skill',
      body: { name: 'test-skill', projectRoot: '/tmp' },
    });
    expect(status).toBe(201);
    expect(body).toBeDefined();
  });

  test('GET /api/capability/agents lists agents', async ({ authedRequest }) => {
    const { status, body } = await authedRequest({
      method: 'GET',
      path: '/api/capability/agents?projectRoot=/tmp',
    });
    expect(status).toBe(200);
    expect(body).toBeDefined();
  });

  test('POST /api/capability/match matches agent', async ({ authedRequest }) => {
    const { status, body } = await authedRequest({
      method: 'POST',
      path: '/api/capability/match',
      body: { taskType: 'code', projectRoot: '/tmp' },
    });
    expect(status).toBe(200);
    expect(body).toBeDefined();
  });

  test('GET /api/capability/skills lists skills', async ({ authedRequest }) => {
    const { status, body } = await authedRequest({
      method: 'GET',
      path: '/api/capability/skills?projectRoot=/tmp',
    });
    expect(status).toBe(200);
    expect(body).toBeDefined();
  });

  test('GET /api/capability/templates lists templates', async ({ authedRequest }) => {
    const { status, body } = await authedRequest({
      method: 'GET',
      path: '/api/capability/templates',
    });
    expect(status).toBe(200);
    expect(body).toBeDefined();
  });

  test('GET /api/capability/readiness returns readiness', async ({ authedRequest }) => {
    const { status, body } = await authedRequest({
      method: 'GET',
      path: '/api/capability/readiness?projectRoot=/tmp',
    });
    expect(status).toBe(200);
    expect(body).toBeDefined();
  });
});
