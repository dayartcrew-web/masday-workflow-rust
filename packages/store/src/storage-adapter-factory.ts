import { SqliteBackend } from './sqlite-backend.js';
import { JsonBackend } from './json-backend.js';
import { PrismaBackend } from './prisma-backend.js';
import type { StorageBackend } from './types.js';
import { createLogger } from '@mcp-rebuild/core';

const logger = createLogger('StorageAdapterFactory');

/**
 * Configuration for creating a storage backend.
 * The `type` field determines which backend is instantiated:
 * - 'sqlite' — local SQLite database (default, for development)
 * - 'json'   — JSON file-based storage (lightweight, for testing)
 * - 'prisma' — PostgreSQL via Prisma (production-grade, async)
 */
export interface StorageConfig {
  type: 'sqlite' | 'json' | 'prisma';
  /** Path for SQLite database file or JSON file */
  path?: string;
  /** PostgreSQL connection string (prisma only) */
  connectionString?: string;
}

/**
 * Factory for creating storage backends based on configuration.
 * This enables the pluggable backend pattern (inspired by voltagent):
 * - Development: SqliteBackend (zero external dependencies)
 * - Testing: JsonBackend (in-memory, no file I/O required)
 * - Production: PrismaBackend (PostgreSQL + pgvector, async)
 *
 * Usage:
 *   const backend = StorageAdapterFactory.create({ type: 'sqlite', path: './data.db' });
 *   // For Prisma (async):
 *   const backend = await StorageAdapterFactory.createAsync({ type: 'prisma', connectionString: 'postgresql://...' });
 */
export const StorageAdapterFactory = {
  /**
   * Create a synchronous storage backend (sqlite or json).
   * For Prisma (async), use createAsync() instead.
   */
  create(config: StorageConfig): StorageBackend {
    switch (config.type) {
      case 'sqlite': {
        const path = config.path ?? ':memory:';
        logger.info(`Creating SQLite backend at ${path}`);
        return new SqliteBackend(path);
      }
      case 'json': {
        const path = config.path ?? ':memory:';
        logger.info(`Creating JSON backend at ${path}`);
        return new JsonBackend(path);
      }
      case 'prisma':
        throw new Error(
          'Prisma backend requires async initialization. Use StorageAdapterFactory.createAsync() instead.'
        );
      default:
        throw new Error(`Unknown storage backend type: ${String(config.type)}`);
    }
  },

  /**
   * Create and initialize a storage backend asynchronously.
   * Required for PrismaBackend which needs async connection setup.
   * For sqlite/json, this still works (initialize is synchronous).
   */
  async createAsync(config: StorageConfig): Promise<StorageBackend> {
    if (config.type === 'prisma') {
      const connStr = config.connectionString;
      if (!connStr) {
        throw new Error('Prisma backend requires a connectionString');
      }
      logger.info('Creating Prisma backend');
      const backend = new PrismaBackend(connStr);
      await backend.initialize();
      // Wrap PrismaBackend in a SyncAdapterBridge for StorageBackend interface compatibility
      return new PrismaSyncBridge(backend);
    }

    // For sync backends, create and initialize synchronously
    const backend = this.create(config);
    backend.initialize();
    return backend;
  },

  /**
   * Create and initialize the default backend (SQLite in-memory).
   * Useful for tests and quick prototyping.
   */
  createDefault(): StorageBackend {
    const backend = new SqliteBackend(':memory:');
    backend.initialize();
    return backend;
  },
} as const;

/**
 * Bridge that wraps an async PrismaBackend behind the synchronous StorageBackend interface.
 * Since Prisma operations are async but the existing codebase uses sync StorageBackend,
 * this bridge throws descriptive errors for sync methods and provides async alternatives.
 *
 * For code paths that need async storage, use the underlying PrismaBackend directly
 * via `PrismaSyncBridge.getAsyncBackend()`.
 */
export class PrismaSyncBridge implements StorageBackend {
  private prismaBackend: PrismaBackend;

  constructor(prismaBackend: PrismaBackend) {
    this.prismaBackend = prismaBackend;
  }

  /**
   * Get the underlying PrismaBackend for async operations.
   */
  getAsyncBackend(): PrismaBackend {
    return this.prismaBackend;
  }

  initialize(): void {
    // PrismaBackend is already initialized by the factory
    if (!this.prismaBackend.isConnected()) {
      throw new Error('PrismaBackend must be initialized before wrapping in PrismaSyncBridge');
    }
  }

  close(): void {
    // Fire-and-forget close. Use getAsyncBackend().close() for proper cleanup.
    void this.prismaBackend.close();
  }

  run(_sql: string, _params?: unknown[]): never {
    throw new Error(
      'Synchronous run() not supported with Prisma backend. ' +
      'Use getAsyncBackend().run() for async SQL execution, or use sqlite/json backend for sync operations.'
    );
  }

  query(_sql: string, _params?: unknown[]): never {
    throw new Error(
      'Synchronous query() not supported with Prisma backend. ' +
      'Use getAsyncBackend().query() for async SQL execution, or use sqlite/json backend for sync operations.'
    );
  }

  queryOne(_sql: string, _params?: unknown[]): never {
    throw new Error(
      'Synchronous queryOne() not supported with Prisma backend. ' +
      'Use getAsyncBackend().queryOne() for async SQL execution, or use sqlite/json backend for sync operations.'
    );
  }
}
