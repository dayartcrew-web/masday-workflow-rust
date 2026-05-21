import { drizzle } from "drizzle-orm/postgres-js";
import postgres from "postgres";
import * as schema from "./schema.js";

const connectionString = process.env.DATABASE_URL!;
const client = postgres(connectionString);
export const db = drizzle(client, { schema });

// Deprecated: apps/api still imports { prisma } — migrate to Drizzle db and remove this stub
// eslint-disable-next-line @typescript-eslint/no-explicit-any
let _prisma: any = null;
export { _prisma as prisma };

export async function disconnectDb(): Promise<void> {
  await client.end();
}

export async function healthCheck(timeoutMs = 2000): Promise<boolean> {
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
