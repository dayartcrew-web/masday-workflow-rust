import type { StorageBackend, IConfigStore } from './types.js';
import { createLogger } from '@mcp-rebuild/core';

const logger = createLogger('ConfigStore');

export class ConfigStore implements IConfigStore {
  private backend: StorageBackend;

  constructor(backend: StorageBackend) {
    this.backend = backend;
  }

  get(key: string): string | undefined {
    const row = this.backend.queryOne<{ value: string }>(
      'SELECT value FROM config WHERE key = ?',
      [key]
    );
    return row?.value;
  }

  set(key: string, value: string): void {
    const now = new Date().toISOString();
    const existing = this.get(key);
    if (existing !== undefined) {
      this.backend.run(
        'UPDATE config SET value = ?, updated_at = ? WHERE key = ?',
        [value, now, key]
      );
    } else {
      this.backend.run(
        'INSERT INTO config (key, value, updated_at) VALUES (?, ?, ?)',
        [key, value, now]
      );
    }
    logger.debug(`Config set: ${key}`);
  }

  delete(key: string): void {
    this.backend.run('DELETE FROM config WHERE key = ?', [key]);
    logger.debug(`Config deleted: ${key}`);
  }

  getAll(): Map<string, string> {
    const rows = this.backend.query<{ key: string; value: string }>('SELECT key, value FROM config');
    const map = new Map<string, string>();
    for (const row of rows) {
      map.set(row.key, row.value);
    }
    return map;
  }
}
