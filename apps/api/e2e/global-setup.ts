import { startTestServer } from './fixtures';

async function globalSetup() {
  await startTestServer();
}

export default globalSetup;
