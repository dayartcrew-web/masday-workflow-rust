/**
 * on-stop — Enforces masday wrap-up before session ends.
 *
 * Checks:
 * 1. Active workflow/task exists → remind to saveProgress + memory.store
 * 2. Uncommitted changes → remind to commit or stash
 * 3. Incomplete task state → warn about leaving tasks RUNNING
 */

import { readFile, readdir } from 'node:fs/promises';
import { join } from 'node:path';
import { execSync } from 'node:child_process';

async function loadActiveWorkflow(cwd) {
  const statePath = join(cwd, '.masday', 'state.json');
  try {
    const raw = await readFile(statePath, 'utf8');
    const state = JSON.parse(raw);
    return state?.activeWorkflow || null;
  } catch { return null; }
}

async function loadActiveTask(cwd) {
  const statePath = join(cwd, '.masday', 'state.json');
  try {
    const raw = await readFile(statePath, 'utf8');
    const state = JSON.parse(raw);
    return state?.activeTask || null;
  } catch {
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
    } catch { /* no tasks dir */ }
  }
  return null;
}

function hasUncommittedChanges(cwd) {
  try {
    const status = execSync('git status --porcelain', { cwd, encoding: 'utf8', timeout: 5000 });
    return status.trim().length > 0;
  } catch { return false; }
}

export default async function onStop(context) {
  const cwd = context?.cwd || process.cwd();
  const warnings = [];

  // 1. Check active workflow
  const workflow = await loadActiveWorkflow(cwd);
  if (workflow) {
    const task = await loadActiveTask(cwd);
    if (task) {
      const taskName = task.title || task.name || task.id;
      warnings.push(
        `[on-stop] ACTIVE TASK "${taskName}" still RUNNING. Before ending:\n` +
        `  1. workflow.saveProgress — persist current work\n` +
        `  2. review.submit — quality gate\n` +
        `  3. workflow.completeTask — close the task\n` +
        `  4. memory.store — save findings for future sessions`
      );
    } else {
      warnings.push(
        `[on-stop] Active workflow "${workflow.name || workflow.id}" found. Run workflow.saveProgress and memory.store before ending.`
      );
    }
  }

  // 2. Check uncommitted changes
  if (hasUncommittedChanges(cwd)) {
    warnings.push(
      `[on-stop] Uncommitted changes detected. Commit or stash before ending.`
    );
  }

  if (warnings.length === 0) return;

  return {
    systemMessage: warnings.join('\n\n'),
  };
}
