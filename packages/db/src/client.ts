import { drizzle, type PostgresJsDatabase } from "drizzle-orm/postgres-js";
import postgres from "postgres";
import * as schema from "./schema.js";

// Resolve .env BEFORE reading DATABASE_URL.
// ESM hoists ALL imports above consumer code, so dotenv loading in mcp.ts
// runs AFTER client.ts evaluates. Without this, DATABASE_URL is undefined
// when this module loads and the throw below kills the import chain.
if (!process.env.DATABASE_URL) {
  try {
    const { config } = await import("dotenv");
    const { fileURLToPath } = await import("url");
    const { dirname, join } = await import("path");
    const { existsSync } = await import("fs");
    const __dir = dirname(fileURLToPath(import.meta.url));
    // client.ts is at packages/db/src/client.ts → dist/ → 3 levels up to project root
    const root = join(__dir, "..", "..", "..");
    const envPath = join(root, ".env");
    if (existsSync(envPath)) config({ path: envPath });
    else config();
  } catch { /* dotenv not available — rely on environment */ }
}

// Prevent MaxListenersExceededWarning — postgres registers exit handlers
// in its constructor. Lazy init means this runs once when pool is created.
if (process.getMaxListeners() !== 0) {
  process.setMaxListeners(Math.max(process.getMaxListeners(), 20));
}

// Lazy pool: postgres() constructor is deferred until first use.
// Module import is free — zero TCP connections until getDb() or healthCheck().
type PgPool = ReturnType<typeof postgres>;
let _pool: PgPool | null = null;
let _db: PostgresJsDatabase<typeof schema> | null = null;

function createPool(): PgPool {
  const connectionString = process.env.DATABASE_URL;
  if (!connectionString) {
    throw new Error("DATABASE_URL environment variable is required");
  }
  const isLocal = connectionString.includes("localhost") || connectionString.includes("127.0.0.1");
  return postgres(connectionString, {
    prepare: false,
    ssl: isLocal ? false : { rejectUnauthorized: false },
    connect_timeout: 15,
    idle_timeout: 120,
    max_lifetime: 60 * 30,
    max: isLocal ? 20 : 10,
    keep_alive: 10_000,
    connection: {
      application_name: "masday",
    },
  });
}

/** Ensure pool exists, creating it lazily on first call. */
function ensurePool(): PgPool {
  if (!_pool) {
    _pool = createPool();
    _db = drizzle(_pool, { schema });
  }
  return _pool;
}

/** Get or create the Drizzle instance (lazy — first call creates the pool). */
export function getDb(): PostgresJsDatabase<typeof schema> {
  ensurePool();
  return _db!;
}

/** Eager export for backward compat — proxies to lazy getDb(). */
export const db = new Proxy({} as PostgresJsDatabase<typeof schema>, {
  get(_target, prop) {
    return (getDb() as never)[prop];
  },
});

export async function disconnectDb(): Promise<void> {
  if (_pool) {
    await _pool.end();
    _pool = null;
    _db = null;
  }
}

export async function healthCheck(timeoutMs = 5000): Promise<boolean> {
  try {
    const pool = ensurePool();
    await Promise.race([
      pool`SELECT 1`,
      new Promise<never>((_, reject) => setTimeout(() => reject(new Error("timeout")), timeoutMs)),
    ]);
    return true;
  } catch {
    return false;
  }
}
