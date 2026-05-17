import type { RunResult } from './types.js';
import { createLogger } from '@mcp-rebuild/core';

const logger = createLogger('PrismaBackend');

/**
 * Minimal structural type for PrismaClient methods we need.
 * Avoids importing @prisma/client at compile time so the package
 * is optional (only required when actually using PostgreSQL).
 */
interface PrismaClientLike {
  $connect(): Promise<void>;
  $disconnect(): Promise<void>;
  $executeRawUnsafe(query: string, ...args: unknown[]): Promise<number>;
  $queryRawUnsafe(query: string, ...args: unknown[]): Promise<unknown[]>;
}

/**
 * PrismaBackend provides an async-capable PostgreSQL storage backend.
 *
 * Unlike SqliteBackend and JsonBackend which implement the synchronous
 * StorageBackend interface, this class implements AsyncStorageBackend
 * because Prisma operations are inherently asynchronous.
 *
 * The StorageAdapterFactory can create either sync or async backends
 * based on configuration.
 */
export class PrismaBackend {
  private client: PrismaClientLike | null = null;
  private connectionString: string;
  private connected = false;

  constructor(connectionString: string) {
    this.connectionString = connectionString;
  }

  /**
   * Initialize the Prisma client and connect to PostgreSQL.
   * Must be called before any other method.
   */
  async initialize(): Promise<void> {
    if (this.connected) return;

    try {
      // Dynamic import via variable to avoid TS module resolution error
      // @prisma/client is an optional peer dependency - not required at compile time
      const moduleName = '@prisma/client';
      // eslint-disable-next-line @typescript-eslint/no-unsafe-member-access
      const prismaModule = await import(moduleName) as unknown as { PrismaClient: new (opts: unknown) => PrismaClientLike };
      this.client = new prismaModule.PrismaClient({
        datasources: {
          db: { url: this.connectionString },
        },
      });
      await this.client.$connect();
      this.connected = true;
      logger.info('Prisma backend initialized');
    } catch (error: unknown) {
      const message = error instanceof Error ? error.message : String(error);
      throw new Error(`Failed to initialize Prisma backend: ${message}`);
    }
  }

  /**
   * Disconnect from PostgreSQL and release resources.
   */
  async close(): Promise<void> {
    if (this.client) {
      await this.client.$disconnect();
      this.client = null;
      this.connected = false;
      logger.info('Prisma backend closed');
    }
  }

  /**
   * Check if the backend is connected.
   */
  isConnected(): boolean {
    return this.connected;
  }

  /**
   * Execute a raw SQL statement (INSERT, UPDATE, DELETE).
   */
  async run(sql: string, params?: unknown[]): Promise<RunResult> {
    if (!this.client) throw new Error('Prisma backend not initialized');
    const changes = await this.client.$executeRawUnsafe(sql, ...(params ?? []));
    return { changes, lastInsertRowid: 0 };
  }

  /**
   * Execute a raw SQL query and return all matching rows.
   */
  async query<T = Record<string, unknown>>(sql: string, params?: unknown[]): Promise<T[]> {
    if (!this.client) throw new Error('Prisma backend not initialized');
    return this.client.$queryRawUnsafe(sql, ...(params ?? [])) as Promise<T[]>;
  }

  /**
   * Execute a raw SQL query and return the first matching row, or undefined.
   */
  async queryOne<T = Record<string, unknown>>(sql: string, params?: unknown[]): Promise<T | undefined> {
    const rows = await this.query<T>(sql, params);
    return rows[0];
  }
}
