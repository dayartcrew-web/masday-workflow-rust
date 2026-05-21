/**
 * Session state (msd-mcp business logic)
 */

import { eq } from "drizzle-orm";

export async function getOrCreateSessionState(sessionKey: string) {
  const { db, sessionStates } = await import("@mcp-rebuild/db");

  const [existing] = await db.select().from(sessionStates).where(eq(sessionStates.sessionKey, sessionKey));
  if (existing) return existing;

  const [created] = await db.insert(sessionStates).values({ sessionKey }).returning();
  return created;
}

export async function patchSessionState(
  sessionKey: string,
  patch: Record<string, unknown>,
) {
  await getOrCreateSessionState(sessionKey);

  const { db, sessionStates } = await import("@mcp-rebuild/db");
  const [updated] = await db.update(sessionStates).set(patch as Record<string, unknown>).where(eq(sessionStates.sessionKey, sessionKey)).returning();
  return updated;
}
