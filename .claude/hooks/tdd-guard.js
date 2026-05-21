/**
 * tdd-guard — Enforces TDD for tasks with requiresTdd=true.
 *
 * Three enforcement levels:
 * 1. NO ACTIVE TASK: soft reminder (no workflow context)
 * 2. ACTIVE TASK, requiresTdd=false: soft reminder
 * 3. ACTIVE TASK, requiresTdd=true: HARD BLOCK if no test file exists
 *
 * Reads task state from .masday/ local cache (synced from PostgreSQL).
 */

import { access, readFile, readdir } from 'node:fs/promises';
import { join, dirname, basename, extname } from 'node:path';

const SRC_EXTENSIONS = new Set(['.ts', '.tsx', '.js', '.jsx', '.mjs', '.cjs']);

function guessTestPath(srcPath) {
  const dir = dirname(srcPath);
  const base = basename(srcPath, extname(srcPath));
  const ext = extname(srcPath);

  return [
    join(dir, `${base}.test${ext}`),
    join(dir, `${base}.spec${ext}`),
    join(dir, '__tests__', `${base}.test${ext}`),
    join(dir, 'tests', `${base}.test${ext}`),
    join(dir, 'test', `${base}.test${ext}`),
  ];
}

async function fileExists(path) {
  try { await access(path); return true; } catch { return false; }
}

async function loadActiveTask(cwd) {
  // Try .masday/state.json (local state dir)
  const statePath = join(cwd, '.masday', 'state.json');
  try {
    const raw = await readFile(statePath, 'utf8');
    const state = JSON.parse(raw);
    return state?.activeTask || null;
  } catch {
    // no local state
  }

  // Fallback: try .masday/cache/tasks/*.json
  try {
    const tasksDir = join(cwd, '.masday', 'cache', 'tasks');
    const files = await readdir(tasksDir).catch(() => []);
    for (const f of files) {
      if (!f.endsWith('.json')) continue;
      try {
        const raw = await readFile(join(tasksDir, f), 'utf8');
        const task = JSON.parse(raw);
        if (task.status === 'RUNNING' || task.status === 'running') return task;
      } catch { /* skip */ }
    }
  } catch {
    // no tasks dir
  }

  return null;
}

export default async function tddGuard(context) {
  const filePath = context.tool_input?.file_path || '';
  if (!filePath) return;

  const ext = extname(filePath).toLowerCase();
  if (!SRC_EXTENSIONS.has(ext)) return;
  if (filePath.includes('.test.') || filePath.includes('.spec.')) return;
  if (filePath.includes('node_modules')) return;
  if (filePath.includes('.claude/hooks/')) return;

  // Check if a test file already exists
  const testPaths = guessTestPath(filePath);
  let hasTest = false;
  for (const testPath of testPaths) {
    if (await fileExists(testPath)) {
      hasTest = true;
      break;
    }
  }

  // Load active task from .masday/ local state
  const cwd = context.tool_input?.cwd || process.cwd();
  const task = await loadActiveTask(cwd);

  const fileName = basename(filePath);

  // No active task — soft reminder only
  if (!task) {
    if (hasTest) return;
    return {
      systemMessage: `[tdd-guard] No test file found for ${fileName}. Consider creating a test file first (TDD workflow).`,
    };
  }

  const requiresTdd = task.requiresTdd === true || task.requires_tdd === true;

  // Task does NOT require TDD — soft reminder
  if (!requiresTdd) {
    if (hasTest) return;
    return {
      systemMessage: `[tdd-guard] No test file found for ${fileName}. Task "${task.title || task.name}" does not require TDD, but tests are recommended.`,
    };
  }

  // Task REQUIRES TDD — hard block if no test file
  if (!hasTest) {
    const taskTitle = task.title || task.name || task.id;
    return {
      decision: 'block',
      reason: `[tdd-guard] BLOCKED: Task "${taskTitle}" requires TDD but no test file found for ${fileName}.\n\n` +
        `This task has requiresTdd=true. You MUST:\n` +
        `1. Create a test file FIRST (RED phase)\n` +
        `2. Verify the test fails\n` +
        `3. Then implement the source file (GREEN phase)\n\n` +
        `Expected test paths:\n` +
        testPaths.map(p => `  - ${basename(p)}`).join('\n'),
    };
  }

  // Test file exists — remind to run tests
  return {
    systemMessage: `[tdd-guard] Test file found for ${fileName}. Task requires TDD — verify tests pass with testEvidence before completing.`,
  };
}
