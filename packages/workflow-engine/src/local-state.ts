/**
 * Local State Sync (unified from msd-mcp and reborn)
 *
 * Manages the .masday/ directory for local-first state persistence
 * and provides sync with both Prisma (msd-mcp) and StorageBackend (reborn).
 */

import {
  existsSync,
  mkdirSync,
  readFileSync,
  readdirSync,
  writeFileSync,
  statSync,
} from "fs";
import { join, resolve } from "path";
import type { LocalState, ArtifactCategory } from "@mcp-rebuild/core";
import type { StorageBackend } from "@mcp-rebuild/store";
import { createLogger } from "@mcp-rebuild/core";

const logger = createLogger("LocalState");

const MSD_DIR = ".msd";

const CATEGORIES: ArtifactCategory[] = [
  "context/codebase",
  "context/research",
  "context/intel",
  "plans",
  "reports",
  "artifacts/diagrams",
];

// ── Init ──

export function initMsdDir(cwd: string): string {
  const msdPath = resolve(cwd, MSD_DIR);

  if (!existsSync(msdPath)) {
    mkdirSync(msdPath, { recursive: true });
  }

  for (const cat of CATEGORIES) {
    const dir = join(msdPath, cat);
    if (!existsSync(dir)) {
      mkdirSync(dir, { recursive: true });
    }
  }

  return msdPath;
}

// ── State ──

export function writeLocalState(cwd: string, state: LocalState): string {
  const msdPath = initMsdDir(cwd);
  const statePath = join(msdPath, "state.json");
  writeFileSync(statePath, JSON.stringify(state, null, 2), "utf-8");
  return statePath;
}

export function readLocalState(cwd: string): LocalState | null {
  const statePath = resolve(cwd, MSD_DIR, "state.json");
  if (!existsSync(statePath)) {
    return null;
  }
  try {
    const raw = readFileSync(statePath, "utf-8");
    return JSON.parse(raw) as LocalState;
  } catch {
    logger.warn({ statePath }, "Failed to read local state");
    return null;
  }
}

// ── Artifacts ──

export function writeArtifact(
  cwd: string,
  category: ArtifactCategory,
  filename: string,
  content: string,
): string {
  const msdPath = initMsdDir(cwd);
  const dir = join(msdPath, category);
  if (!existsSync(dir)) {
    mkdirSync(dir, { recursive: true });
  }
  const filePath = join(dir, filename);
  writeFileSync(filePath, content, "utf-8");
  return filePath;
}

export function listArtifacts(
  cwd: string,
  category?: ArtifactCategory,
): Array<{
  category: string;
  filename: string;
  size: number;
  modified: string;
}> {
  const msdPath = resolve(cwd, MSD_DIR);
  if (!existsSync(msdPath)) {
    return [];
  }

  const results: Array<{
    category: string;
    filename: string;
    size: number;
    modified: string;
  }> = [];
  const categories = category ? [category] : CATEGORIES;

  for (const cat of categories) {
    const dir = join(msdPath, cat);
    if (!existsSync(dir)) {
      continue;
    }
    for (const entry of readdirSync(dir)) {
      const fullPath = join(dir, entry);
      const stat = statSync(fullPath);
      if (stat.isFile()) {
        results.push({
          category: cat,
          filename: entry,
          size: stat.size,
          modified: stat.mtime.toISOString(),
        });
      }
    }
  }

  return results.sort((a, b) => b.modified.localeCompare(a.modified));
}

// ── DB Sync (StorageBackend variant from reborn) ──

export interface SyncResult {
  direction: "to_db";
  workflowId: string;
  tasksSynced: number;
  conflicts: number;
}

const STATUS_PRECEDENCE: Record<string, number> = {
  done: 3,
  reviewing: 2,
  in_progress: 1,
  todo: 0,
  blocked: -1,
};

function higherStatus(a: string, b: string): string {
  return (STATUS_PRECEDENCE[a] ?? 0) >= (STATUS_PRECEDENCE[b] ?? 0) ? a : b;
}

/**
 * Sync local state to a StorageBackend (SQLite/JSON).
 */
export async function syncToDb(
  cwd: string,
  storage: StorageBackend,
  workflowId: string,
): Promise<SyncResult> {
  const local = readLocalState(cwd);
  if (!local) {
    logger.warn("No local state to sync to DB");
    return { direction: "to_db", workflowId, tasksSynced: 0, conflicts: 0 };
  }

  let tasksSynced = 0;
  let conflicts = 0;

  storage.run(`
    CREATE TABLE IF NOT EXISTS local_task_sync (
      id          TEXT PRIMARY KEY,
      workflow_id TEXT NOT NULL,
      title       TEXT,
      status      TEXT NOT NULL,
      progress_percent INTEGER,
      synced_at   TEXT NOT NULL
    )
  `);

  for (const task of local.tasks) {
    const existing = storage.queryOne<{ status: string }>(
      `SELECT status FROM local_task_sync WHERE id = ? AND workflow_id = ?`,
      [task.id, workflowId],
    );

    if (existing) {
      const winner = higherStatus(task.status, existing.status);
      if (winner !== existing.status) {
        storage.run(
          `UPDATE local_task_sync SET status = ?, progress_percent = ?, synced_at = ? WHERE id = ? AND workflow_id = ?`,
          [
            task.status,
            task.progressPercent ?? null,
            new Date().toISOString(),
            task.id,
            workflowId,
          ],
        );
        tasksSynced++;
      } else if (winner !== task.status) {
        conflicts++;
      }
    } else {
      storage.run(
        `INSERT INTO local_task_sync (id, workflow_id, title, status, progress_percent, synced_at) VALUES (?, ?, ?, ?, ?, ?)`,
        [
          task.id,
          workflowId,
          task.title,
          task.status,
          task.progressPercent ?? null,
          new Date().toISOString(),
        ],
      );
      tasksSynced++;
    }
  }

  logger.info(
    { workflowId, tasksSynced, conflicts },
    "Synced local state to DB",
  );
  return { direction: "to_db", workflowId, tasksSynced, conflicts };
}

/**
 * Sync from a StorageBackend to local .masday/ directory.
 */
export async function syncFromDb(
  cwd: string,
  storage: StorageBackend,
  workflowId: string,
): Promise<LocalState> {
  storage.run(`
    CREATE TABLE IF NOT EXISTS local_task_sync (
      id          TEXT PRIMARY KEY,
      workflow_id TEXT NOT NULL,
      title       TEXT,
      status      TEXT NOT NULL,
      progress_percent INTEGER,
      synced_at   TEXT NOT NULL
    )
  `);

  const rows = storage.query<{
    id: string;
    workflow_id: string;
    title: string | null;
    status: string;
    progress_percent: number | null;
  }>(`SELECT * FROM local_task_sync WHERE workflow_id = ?`, [workflowId]);

  const local = readLocalState(cwd);
  const now = new Date().toISOString();

  const tasks: LocalState["tasks"] = rows.map((row) => {
    const localTask = local?.tasks.find((t) => t.id === row.id);
    const resolvedStatus = localTask
      ? higherStatus(row.status, localTask.status)
      : row.status;

    return {
      id: row.id,
      title: row.title ?? localTask?.title ?? "Untitled",
      status: resolvedStatus,
      progressPercent:
        row.progress_percent ?? localTask?.progressPercent ?? null,
    };
  });

  const currentTask =
    tasks.find((t) => t.status === "in_progress") ??
    tasks.find((t) => t.status !== "done") ??
    null;

  const newState: LocalState = {
    syncedAt: now,
    workflow: local?.workflow ?? {
      id: workflowId,
      name: "synced-workflow",
      status: "executing",
      createdAt: now,
      updatedAt: now,
    },
    plan: local?.plan ?? null,
    currentTask,
    tasks,
  };

  writeLocalState(cwd, newState);
  logger.info(
    { workflowId, taskCount: tasks.length },
    "Synced DB state to local",
  );
  return newState;
}
