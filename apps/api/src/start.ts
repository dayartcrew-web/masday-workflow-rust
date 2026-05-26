#!/usr/bin/env node
import { createRequire } from 'module';
import { resolve } from 'path';
import { fileURLToPath } from 'url';

const require = createRequire(import.meta.url);
const __dirname = fileURLToPath(new URL('.', import.meta.url));

// Load .env BEFORE any other module evaluation
const dotenv = require('dotenv');
const envPath = resolve(__dirname, '..', '..', '..', '.env');
const result = dotenv.config({ path: envPath });

if (result.error) {
  console.error(`[bootstrap] Warning: could not load .env: ${result.error.message}`);
} else {
  console.error(`[bootstrap] Loaded ${Object.keys(result.parsed || {}).length} env vars from ${envPath}`);
  console.error(`[bootstrap] DATABASE_URL set: ${!!process.env.DATABASE_URL}`);
}

// Catch unhandled rejections (postgres internal errors, etc.)
process.on('unhandledRejection', (reason) => {
  console.error('[bootstrap] Unhandled rejection:', reason);
});

// Now dynamically import main.ts (all its static imports will see DATABASE_URL)
import('./main.js').catch((err) => {
  console.error('[bootstrap] Failed to start:', err);
  process.exit(1);
});
