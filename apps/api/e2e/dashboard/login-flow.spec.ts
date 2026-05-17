import { test, expect } from '@playwright/test';

test.describe('Dashboard Integration', () => {
  test('login page renders with email and name fields', async ({ page }) => {
    await page.goto('/login');

    await expect(page.locator('#email')).toBeVisible();
    await expect(page.locator('#name')).toBeVisible();
    await expect(page.getByRole('button', { name: /sign in/i })).toBeVisible();
  });

  test('full dashboard E2E flow', async ({ page }) => {
    test.setTimeout(120000);

    await page.goto('/login');
    await page.locator('#email').fill('e2e@test.com');
    await page.locator('#name').fill('E2E Tester');
    await page.getByRole('button', { name: /sign in/i }).click();
    await page.waitForURL('/', { timeout: 30000 });

    const token = await page.evaluate(() => localStorage.getItem('auth_token'));
    expect(token).toBeTruthy();

    // Each page mounts its own AppShell which calls checkAuth() → GET /api/auth/me.
    // Intercept to prevent transient API failures from logging us out mid-test.
    await page.route('**/api/auth/me', async (route) => {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({ user: { email: 'e2e@test.com', name: 'E2E Tester' } }),
      });
    });

    await test.step('dashboard page loads', async () => {
      await expect(page.getByText(/active workflows/i)).toBeVisible({ timeout: 10000 });
    });

    await test.step('workflows page renders', async () => {
      await page.getByRole('link', { name: /workflows/i }).click();
      await expect(page.getByText(/new workflow/i)).toBeVisible({ timeout: 10000 });
    });

    await test.step('graph page renders', async () => {
      await page.getByRole('link', { name: /knowledge graph/i }).click();
      await page.waitForURL('/graph', { timeout: 10000 });
      await expect(page.getByRole('main').getByRole('heading', { name: /knowledge graph/i })).toBeVisible({ timeout: 10000 });
    });

    await test.step('providers page renders', async () => {
      await page.goto('/providers');
      await expect(page.getByText(/llm providers/i)).toBeVisible({ timeout: 10000 });
    });

    await test.step('settings page shows user info', async () => {
      await page.goto('/settings');
      await expect(page.getByRole('main').getByRole('heading', { name: /settings/i })).toBeVisible({ timeout: 10000 });
      await expect(page.getByRole('main').getByText(/e2e@test\.com/i)).toBeVisible({ timeout: 10000 });
    });
  });
});
