import { config } from 'dotenv';
import path from 'path';

const envPath = path.resolve(process.cwd(), '.env');
const result = config({ path: envPath });

if (result.error) {
  console.error(`[preload-env] Warning: could not load ${envPath}: ${result.error.message}`);
} else {
  console.error(`[preload-env] Loaded ${Object.keys(result.parsed || {}).length} env vars from ${envPath}`);
}

export { result };
