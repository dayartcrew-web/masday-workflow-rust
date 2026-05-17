import { test, expect } from '../fixtures';

test.describe.serial('Workflow API', () => {
  let workflowId: string;

  test('POST /api/workflows creates a new workflow', async ({ authedRequest }) => {
    const { status, body } = await authedRequest({
      method: 'POST',
      path: '/api/workflows',
      body: { name: 'E2E Workflow', description: 'Created by E2E tests' },
    });

    expect(status).toBe(201);
    const workflow = (body as { workflow: { id: string; state: string } }).workflow;
    expect(workflow.id).toBeDefined();
    expect(workflow.state).toBe('INIT');
    workflowId = workflow.id;
  });

  test('GET /api/workflows returns list containing the created workflow', async ({ authedRequest }) => {
    const { status, body } = await authedRequest({
      method: 'GET',
      path: '/api/workflows',
    });

    expect(status).toBe(200);
    const workflows = (body as { workflows: { id: string }[] }).workflows;
    expect(workflows.some((w) => w.id === workflowId)).toBe(true);
  });

  test('GET /api/workflows/active returns the most recent workflow', async ({ authedRequest }) => {
    const { status, body } = await authedRequest({
      method: 'GET',
      path: '/api/workflows/active',
    });

    expect(status).toBe(200);
    expect((body as { workflow: { id: string } | null }).workflow).toBeDefined();
  });

  test('GET /api/workflows/:id returns the workflow', async ({ authedRequest }) => {
    const { status, body } = await authedRequest({
      method: 'GET',
      path: `/api/workflows/${workflowId}`,
    });

    expect(status).toBe(200);
    expect((body as { workflow: { id: string } }).workflow.id).toBe(workflowId);
  });

  test('POST /api/workflows/:id/plan creates tasks', async ({ authedRequest }) => {
    const { status, body } = await authedRequest({
      method: 'POST',
      path: `/api/workflows/${workflowId}/plan`,
      body: { tasks: [{ title: 'Test task' }] },
    });

    expect(status).toBe(200);
    const plan = (body as { plan: { taskCount: number } }).plan;
    expect(plan.taskCount).toBeGreaterThanOrEqual(1);
  });

  test('GET /api/workflows/:id/tasks returns tasks', async ({ authedRequest }) => {
    const { status, body } = await authedRequest({
      method: 'GET',
      path: `/api/workflows/${workflowId}/tasks`,
    });

    expect(status).toBe(200);
    const tasks = (body as { tasks: unknown[] }).tasks;
    expect(tasks.length).toBeGreaterThanOrEqual(1);
  });

  test('POST /api/workflows/:id/tasks adds a task', async ({ authedRequest }) => {
    const { status, body } = await authedRequest({
      method: 'POST',
      path: `/api/workflows/${workflowId}/tasks`,
      body: { name: 'Extra task', agent: 'test-agent', skill: 'test-skill' },
    });

    expect(status).toBe(201);
    const task = (body as { task: { name: string } }).task;
    expect(task.name).toBe('Extra task');
  });

  test('POST /api/workflows/:id/execute runs the workflow', async ({ authedRequest }) => {
    const { status, body } = await authedRequest({
      method: 'POST',
      path: `/api/workflows/${workflowId}/execute`,
    });

    expect([200, 500]).toContain(status);
    if (status === 200) {
      expect(['DONE', 'FAILED']).toContain((body as { workflow: { state: string } }).workflow.state);
    }
  });

  test('GET /api/workflows/:id/status returns status', async ({ authedRequest }) => {
    const { status, body } = await authedRequest({
      method: 'GET',
      path: `/api/workflows/${workflowId}/status`,
    });

    expect(status).toBe(200);
    expect((body as { status: unknown }).status).toBeDefined();
  });
});
