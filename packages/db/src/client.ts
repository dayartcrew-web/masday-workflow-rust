import { drizzle } from "drizzle-orm/postgres-js";
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
// in its constructor. ESM hoists imports above top-level code in consumers,
// so this must run BEFORE the postgres() call here in client.ts.
if (process.getMaxListeners() !== 0) {
  process.setMaxListeners(Math.max(process.getMaxListeners(), 20));
}

const connectionString = process.env.DATABASE_URL;
if (!connectionString) {
  throw new Error("DATABASE_URL environment variable is required");
}
const isLocal = connectionString.includes("localhost") || connectionString.includes("127.0.0.1");
const client = postgres(connectionString, {
  prepare: false,
  ssl: isLocal ? false : { rejectUnauthorized: false },
  connect_timeout: 15,
  idle_timeout: 120,
  max_lifetime: 60 * 30,
  // Supabase pooler session mode limits pool_size to 15; keep max below that.
  max: isLocal ? 20 : 10,
  keep_alive: 10_000,
  connection: {
    application_name: "masday",
  },
});
export const db = drizzle(client, { schema });


export async function disconnectDb(): Promise<void> {
  await client.end();
}

export async function healthCheck(timeoutMs = 5000): Promise<boolean> {
  try {
    await Promise.race([
      client`SELECT 1`,
      new Promise<never>((_, reject) => setTimeout(() => reject(new Error("timeout")), timeoutMs)),
    ]);
    return true;
  } catch {
    return false;
  }
}
