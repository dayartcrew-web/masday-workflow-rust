import { stopTestServer } from './fixtures';

async function globalTeardown() {
  await stopTestServer();
}

export default globalTeardown;
