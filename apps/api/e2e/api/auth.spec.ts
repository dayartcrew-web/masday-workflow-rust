import { test, expect } from '../fixtures';

test.describe.serial('Auth API', () => {
  test('POST /api/auth/login with { email, name } returns 200 with token and user', async ({ request }) => {
    const res = await request.post('/api/auth/login', {
      data: { email: 'auth-test@example.com', name: 'Auth Tester' },
    });
    const body = await res.json();

    expect(res.status()).toBe(200);
    expect(body).toHaveProperty('token');
    expect(typeof body.token).toBe('string');
    expect(body).toHaveProperty('user');
    expect(body.user).toHaveProperty('id');
    expect(body.user.email).toBe('auth-test@example.com');
    expect(body.user.name).toBe('Auth Tester');
  });

  test('POST /api/auth/login with same email returns same user', async ({ request }) => {
    const email = `same-user-${Date.now()}@test.com`;
    const res1 = await request.post('/api/auth/login', {
      data: { email, name: 'First' },
    });
    const body1 = await res1.json();

    const res2 = await request.post('/api/auth/login', {
      data: { email, name: 'Second' },
    });
    const body2 = await res2.json();

    expect(res2.status()).toBe(200);
    expect(body2.user.id).toBe(body1.user.id);
  });

  test.skip('POST /api/auth/login with missing email returns 4xx', async ({ request }) => {
    const res = await request.post('/api/auth/login', {
      data: { name: 'No Email' },
    });

    expect(res.status()).toBeGreaterThanOrEqual(400);
    expect(res.status()).toBeLessThan(500);
  });

  test('POST /api/auth/token with valid token returns { valid: true }', async ({ authToken, authedRequest }) => {
    const { status, body } = await authedRequest({
      method: 'POST',
      path: '/api/auth/token',
      body: { token: authToken },
    });

    expect(status).toBe(200);
    expect((body as { valid: boolean }).valid).toBe(true);
  });

  test('POST /api/auth/token with invalid token returns { valid: false }', async ({ authedRequest }) => {
    const { status, body } = await authedRequest({
      method: 'POST',
      path: '/api/auth/token',
      body: { token: 'invalid-token-value' },
    });

    expect(status).toBe(200);
    expect((body as { valid: boolean }).valid).toBe(false);
  });

  test('GET /api/auth/me with Bearer token returns user object', async ({ authedRequest }) => {
    const { status, body } = await authedRequest({
      method: 'GET',
      path: '/api/auth/me',
    });

    expect(status).toBe(200);
    expect(body).toHaveProperty('user');
    const user = (body as { user: Record<string, unknown> }).user;
    expect(user).toHaveProperty('id');
    expect(user).toHaveProperty('email');
    expect(user).toHaveProperty('name');
    expect(user).toHaveProperty('role');
  });

  test('GET /api/auth/me without token returns 401', async ({ request }) => {
    const res = await request.get('/api/auth/me');

    expect(res.status()).toBe(401);
  });
});
