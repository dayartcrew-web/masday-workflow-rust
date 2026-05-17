import { test, expect } from '../fixtures';

test.describe.serial('Health / Monitoring API', () => {
  test('GET /api/health returns 200 with status field', async ({ request }) => {
    const res = await request.get('/api/health');
    const body = await res.json();

    expect(res.status()).toBe(200);
    expect(body).toHaveProperty('status');
  });

  test('GET /api/metrics returns 200', async ({ authedRequest }) => {
    const { status, body } = await authedRequest({
      method: 'GET',
      path: '/api/metrics',
    });

    expect(status).toBe(200);
  });

  test('GET /api/stats returns 200 with engine and api fields', async ({ authedRequest }) => {
    const { status, body } = await authedRequest({
      method: 'GET',
      path: '/api/stats',
    });

    expect(status).toBe(200);
    expect(body).toHaveProperty('engine');
    expect(body).toHaveProperty('api');
  });
});
