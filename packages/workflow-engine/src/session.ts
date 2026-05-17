/**
 * Session state (msd-mcp business logic)
 */

import { prisma } from "@mcp-rebuild/db";

export async function getOrCreateSessionState(sessionKey: string) {
  const existing = await prisma.sessionState.findUnique({
    where: { sessionKey },
  });

  if (existing) return existing;

  return prisma.sessionState.create({
    data: { sessionKey },
  });
}

export async function patchSessionState(
  sessionKey: string,
  patch: Record<string, unknown>,
) {
  await getOrCreateSessionState(sessionKey);

  return prisma.sessionState.update({
    where: { sessionKey },
    data: patch,
  });
}
