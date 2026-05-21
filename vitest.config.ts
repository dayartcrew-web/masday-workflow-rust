import { defineConfig } from 'vitest/config';
import path from 'path';

export default defineConfig({
  resolve: {
    alias: {
      '@mcp-rebuild/shared-utils': path.resolve(__dirname, 'packages/shared-utils/src/index.ts'),
      '@mcp-rebuild/core': path.resolve(__dirname, 'packages/core/src/index.ts'),
    },
  },
  test: {
    exclude: [
      '**/node_modules/**',
      '**/dist/**',
      'apps/dashboard/**',
      '**/e2e/**',
    ],
  },
});
