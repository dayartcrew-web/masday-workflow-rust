import type { RunResult } from './types.js';
import { createLogger } from '@mcp-rebuild/core';
import { sql } from 'drizzle-orm';

const logger = createLogger('DrizzleBackend');

interface DrizzleDbLike {
  execute(query: ReturnType<typeof sql>): Promise<unknown[]>;
}

/**
 * DrizzleBackend (formerly PrismaBackend) provides an async-capable
 * PostgreSQL storage backend using Drizzle ORM with postgres.js driver.
 */
export class PrismaBackend {
  private db: DrizzleDbLike | null = null;
  private connectionString: string;
  private connected = false;

  constructor(connectionString: string) {
    this.connectionString = connectionString;
  }

  async initialize(): Promise<void> {
    if (this.connected) return;

    try {
      const { drizzle } = await import("drizzle-orm/postgres-js");
      const postgres = (await import("postgres")).default;
      const client = postgres(this.connectionString);
      this.db = drizzle(client) as unknown as DrizzleDbLike;
      this.connected = true;
      logger.info('Drizzle backend initialized');
    } catch (error: unknown) {
      const message = error instanceof Error ? error.message : String(error);
      throw new Error(`Failed to initialize Drizzle backend: ${message}`);
    }
  }

  async close(): Promise<void> {
    this.db = null;
    this.connected = false;
    logger.info('Drizzle backend closed');
  }

  isConnected(): boolean {
    return this.connected;
  }

  async run(queryStr: string, _params?: unknown[]): Promise<RunResult> {
    if (!this.db) throw new Error('Drizzle backend not initialized');
    await this.db.execute(sql.raw(queryStr));
    return { changes: 1, lastInsertRowid: 0 };
  }

  async query<T = Record<string, unknown>>(queryStr: string, _params?: unknown[]): Promise<T[]> {
    if (!this.db) throw new Error('Drizzle backend not initialized');
    const result = await this.db.execute(sql.raw(queryStr));
    return result as T[];
  }

  async queryOne<T = Record<string, unknown>>(queryStr: string, _params?: unknown[]): Promise<T | undefined> {
    const rows = await this.query<T>(queryStr, _params);
    return rows[0];
  }
}
