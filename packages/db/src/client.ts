import { drizzle } from "drizzle-orm/postgres-js";
import postgres from "postgres";
import * as schema from "./schema.js";

const connectionString = process.env.DATABASE_URL;
if (!connectionString) {
  throw new Error("DATABASE_URL environment variable is required");
}
const client = postgres(connectionString, {
  prepare: false,
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
