import { drizzle } from "drizzle-orm/postgres-js";
import postgres from "postgres";
import * as schema from "./schema.js";

const connectionString = process.env.DATABASE_URL!;
const client = postgres(connectionString);
export const db = drizzle(client, { schema });

// Legacy prisma stub — removed after full migration
// eslint-disable-next-line @typescript-eslint/no-explicit-any
let _prisma: any = null;
export { _prisma as prisma };

export async function disconnectDb(): Promise<void> {
  await client.end();
}

export async function healthCheck(): Promise<boolean> {
  try {
    await client`SELECT 1`;
    return true;
  } catch {
    return false;
  }
}
