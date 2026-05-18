import { readFile } from 'node:fs/promises';
import { join, dirname } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';

const __dirname = dirname(fileURLToPath(import.meta.url));

const hookModules = {
  'pre-tool-use': './pre-tool-use.js',
  'tdd-guard': './tdd-guard.js',
  'tool-name-guard': './tool-name-guard.js',
  'pre-build-skill': './pre-build-skill.js',
  'workflow-lock': './workflow-lock.js',
  'post-tool-use': './post-tool-use.js',
  'on-notification': './on-notification.js',
  'on-stop': './on-stop.js',
  'agentic-mem-context': './agentic-mem-context.js',
};

async function readStdin() {
  const chunks = [];
  for await (const chunk of process.stdin) {
    chunks.push(chunk);
  }
  return Buffer.concat(chunks).toString('utf8');
}

async function main() {
  const hookName = process.argv[2];
  if (!hookName) {
    console.error('Usage: run-hook.mjs <hook-name>');
    process.exit(1);
  }

  const modulePath = hookModules[hookName];
  if (!modulePath) {
    console.error(`Unknown hook: ${hookName}`);
    process.exit(1);
  }

  const input = await readStdin();
  let context;
  try {
    context = JSON.parse(input);
  } catch {
    context = {};
  }

  try {
    const mod = await import(pathToFileURL(join(__dirname, modulePath)).href);
    const handler = mod.default || mod;
    const result = await handler(context);

    if (result) {
      console.log(JSON.stringify(result));
    }
    process.exit(0);
  } catch (err) {
    if (err.block) {
      console.error(err.message || 'Operation blocked by hook');
      process.exit(2);
    }
    console.error(`Hook ${hookName} error: ${err.message}`);
    process.exit(0);
  }
}

main();
