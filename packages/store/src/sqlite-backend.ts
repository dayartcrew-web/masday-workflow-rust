import type { StorageBackend, RunResult } from './types.js';
import { createLogger } from '@mcp-rebuild/core';
import { createRequire } from 'node:module';
const esmRequire = createRequire(import.meta.url);

const logger = createLogger('SqliteBackend');

const SCHEMA = `
CREATE TABLE IF NOT EXISTS workflows (
  id          TEXT PRIMARY KEY,
  name        TEXT NOT NULL,
  description TEXT NOT NULL DEFAULT '',
  state       TEXT NOT NULL,
  metadata    TEXT NOT NULL DEFAULT '{}',
  created_at  TEXT NOT NULL,
  updated_at  TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS tasks (
  id           TEXT PRIMARY KEY,
  workflow_id  TEXT NOT NULL REFERENCES workflows(id) ON DELETE CASCADE,
  name         TEXT NOT NULL,
  agent        TEXT NOT NULL,
  skill        TEXT NOT NULL,
  dependencies TEXT NOT NULL DEFAULT '[]',
  state        TEXT NOT NULL,
  input        TEXT,
  output       TEXT,
  error        TEXT,
  created_at   TEXT NOT NULL,
  started_at   TEXT,
  completed_at TEXT
);

CREATE INDEX IF NOT EXISTS idx_tasks_workflow_id ON tasks(workflow_id);
CREATE INDEX IF NOT EXISTS idx_tasks_state ON tasks(state);

CREATE TABLE IF NOT EXISTS config (
  key        TEXT PRIMARY KEY,
  value      TEXT NOT NULL,
  updated_at TEXT NOT NULL
);
`;

export class SqliteBackend implements StorageBackend {
  // Avoid a hard runtime dependency on better-sqlite3 (native module).
  // If it's not installed, we throw a clear error when initializing.
  private db: any | null = null;
  private dbPath: string;

  constructor(dbPath: string) {
    this.dbPath = dbPath;
  }

  initialize(): void {
    let Database: any;
    try {
      // eslint-disable-next-line @typescript-eslint/no-require-imports
      Database = esmRequire('better-sqlite3');
    } catch (err) {
      throw new Error(
        `better-sqlite3 is not installed. Install it to use SqliteBackend. Under Node 24 on Windows this may require Visual Studio Build Tools. Original error: ${String(err)}`
      );
    }

    this.db = new Database(this.dbPath);
    this.db.pragma('journal_mode = WAL');
    this.db.pragma('foreign_keys = ON');
    this.db.exec(SCHEMA);
    logger.info(`SQLite backend initialized at ${this.dbPath}`);
  }

  close(): void {
    if (this.db) {
      this.db.close();
      this.db = null;
      logger.info('SQLite backend closed');
    }
  }

  run(sql: string, params?: unknown[]): RunResult {
    if (!this.db) throw new Error('Database not initialized');
    const stmt = this.db.prepare(sql);
    const result = params ? stmt.run(...params) : stmt.run();
    return { changes: result.changes, lastInsertRowid: result.lastInsertRowid };
  }

  query<T = Record<string, unknown>>(sql: string, params?: unknown[]): T[] {
    if (!this.db) throw new Error('Database not initialized');
    const stmt = this.db.prepare(sql);
    return params ? stmt.all(...params) as T[] : stmt.all() as T[];
  }

  queryOne<T = Record<string, unknown>>(sql: string, params?: unknown[]): T | undefined {
    if (!this.db) throw new Error('Database not initialized');
    const stmt = this.db.prepare(sql);
    return params ? stmt.get(...params) as T | undefined : stmt.get() as T | undefined;
  }
}
