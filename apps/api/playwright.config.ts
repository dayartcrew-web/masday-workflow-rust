import { defineConfig } from '@playwright/test';

export default defineConfig({
  testDir: './e2e',
  fullyParallel: true,
  forbidOnly: !!process.env.CI,
  retries: process.env.CI ? 2 : 0,
  workers: process.env.CI ? 1 : undefined,
  reporter: 'html',
  timeout: 30000,
  globalSetup: './e2e/global-setup.ts',
  globalTeardown: './e2e/global-teardown.ts',
  use: {
    baseURL: `http://localhost:${process.env.API_TEST_PORT || 3099}`,
  },
  webServer: {
    command: 'pnpm -C ../dashboard dev',
    port: parseInt(process.env.DASHBOARD_TEST_PORT || '3092', 10),
    env: {
      NEXT_PUBLIC_API_URL: `http://localhost:${process.env.API_TEST_PORT || 3099}`,
      PORT: process.env.DASHBOARD_TEST_PORT || '3092',
    },
    reuseExistingServer: true,
    timeout: 60000,
  },
  projects: [
    {
      name: 'api',
      testDir: './e2e/api',
    },
    {
      name: 'dashboard',
      testDir: './e2e/dashboard',
      use: { baseURL: `http://localhost:${process.env.DASHBOARD_TEST_PORT || 3092}` },
      dependencies: ['api'],
    },
  ],
});
