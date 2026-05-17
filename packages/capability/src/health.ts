/**
 * System Readiness Check
 *
 * Validates that all dependencies and subsystems are available
 * and properly configured for operation.
 */

import fs from 'fs';
import path from 'path';
import type { StorageBackend } from '@mcp-rebuild/store';
import { WorkflowStore } from '@mcp-rebuild/store';
import { createLogger } from '@mcp-rebuild/core';

const logger = createLogger('SystemHealth');

export interface ReadinessCheck {
  name: string;
  ready: boolean;
  message: string;
}

export interface SystemReadinessResult {
  ready: boolean;
  checks: ReadinessCheck[];
  checkedAt: string;
}

/**
 * Perform a comprehensive system readiness check.
 *
 * Validates:
 * 1. Storage backend is accessible
 * 2. Required tables exist (workflows, tasks)
 * 3. Session manager tables exist
 * 4. Review tables exist
 * 5. Parallel branch tables exist
 * 6. Registry directory (.claude/) exists
 */
export function checkSystemReadiness(
  storage: StorageBackend,
  projectRoot?: string,
): SystemReadinessResult {
  const checks: ReadinessCheck[] = [];

  // Check 1: Storage backend - try a simple query
  try {
    storage.query('SELECT 1 as test');
    checks.push({
      name: 'storage_backend',
      ready: true,
      message: 'Storage backend is accessible',
    });
  } catch (err) {
    checks.push({
      name: 'storage_backend',
      ready: false,
      message: `Storage backend error: ${err instanceof Error ? err.message : String(err)}`,
    });
  }

  // Check 2: Workflows table
  try {
    storage.query('SELECT count(*) as cnt FROM workflows LIMIT 1');
    checks.push({
      name: 'workflows_table',
      ready: true,
      message: 'Workflows table exists',
    });
  } catch {
    checks.push({
      name: 'workflows_table',
      ready: false,
      message: 'Workflows table not found - run initialization',
    });
  }

  // Check 3: Tasks table
  try {
    storage.query('SELECT count(*) as cnt FROM tasks LIMIT 1');
    checks.push({
      name: 'tasks_table',
      ready: true,
      message: 'Tasks table exists',
    });
  } catch {
    checks.push({
      name: 'tasks_table',
      ready: false,
      message: 'Tasks table not found - run initialization',
    });
  }

  // Check 4: Session readiness table
  try {
    storage.query('SELECT count(*) as cnt FROM session_readiness LIMIT 1');
    checks.push({
      name: 'session_table',
      ready: true,
      message: 'Session readiness table exists',
    });
  } catch {
    checks.push({
      name: 'session_table',
      ready: false,
      message: 'Session readiness table not found - initialize SessionManager',
    });
  }

  // Check 5: Review records table
  try {
    storage.query('SELECT count(*) as cnt FROM review_records LIMIT 1');
    checks.push({
      name: 'review_table',
      ready: true,
      message: 'Review records table exists',
    });
  } catch {
    checks.push({
      name: 'review_table',
      ready: false,
      message: 'Review records table not found - initialize ReviewManager',
    });
  }

  // Check 6: Parallel branches table
  try {
    storage.query('SELECT count(*) as cnt FROM parallel_branches LIMIT 1');
    checks.push({
      name: 'parallel_table',
      ready: true,
      message: 'Parallel branches table exists',
    });
  } catch {
    checks.push({
      name: 'parallel_table',
      ready: false,
      message: 'Parallel branches table not found - initialize ParallelExecutor',
    });
  }

  // Check 7: Registry directory (if project root provided)
  if (projectRoot) {
    const claudeDir = path.join(projectRoot, '.claude');
    const exists = fs.existsSync(claudeDir);
    checks.push({
      name: 'registry_directory',
      ready: exists,
      message: exists
        ? '.claude/ directory exists'
        : '.claude/ directory not found - run registry initialization',
    });

    // Check 8: Registry file
    const registryFile = path.join(claudeDir, 'registry.json');
    const registryExists = fs.existsSync(registryFile);
    checks.push({
      name: 'registry_file',
      ready: registryExists,
      message: registryExists
        ? 'Registry file exists'
        : 'Registry file not found - run registry initialization',
    });
  }

  const allReady = checks.every((c) => c.ready);

  const result: SystemReadinessResult = {
    ready: allReady,
    checks,
    checkedAt: new Date().toISOString(),
  };

  logger.info(
    { ready: allReady, checkCount: checks.length, failedChecks: checks.filter((c) => !c.ready).length },
    'System readiness check completed',
  );

  return result;
}

/**
 * Count active workflows in the system.
 */
export function getSystemStats(
  storage: StorageBackend,
): { totalWorkflows: number; activeWorkflows: number; totalTasks: number } {
  try {
    const workflowStore = new WorkflowStore(storage);
    const all = workflowStore.loadAll();
    const active = all.filter(
      (w) => w.state !== 'DONE' && w.state !== 'FAILED',
    );

    const totalTasks = all.reduce((sum, w) => sum + w.tasks.length, 0);

    return {
      totalWorkflows: all.length,
      activeWorkflows: active.length,
      totalTasks,
    };
  } catch {
    return { totalWorkflows: 0, activeWorkflows: 0, totalTasks: 0 };
  }
}
