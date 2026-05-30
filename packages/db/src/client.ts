import { drizzle } from "drizzle-orm/postgres-js";
import postgres from "postgres";
import * as schema from "./schema.js";

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
  max: 20,
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
