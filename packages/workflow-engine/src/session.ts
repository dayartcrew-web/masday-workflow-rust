/**
 * Session state (msd-mcp business logic)
 */



export async function getOrCreateSessionState(sessionKey: string) {
  const existing = await (await import("@mcp-rebuild/db")).prisma.sessionState.findUnique({
    where: { sessionKey },
  });

  if (existing) return existing;

  return (await import("@mcp-rebuild/db")).prisma.sessionState.create({
    data: { sessionKey },
  });
}

export async function patchSessionState(
  sessionKey: string,
  patch: Record<string, unknown>,
) {
  await getOrCreateSessionState(sessionKey);

  return (await import("@mcp-rebuild/db")).prisma.sessionState.update({
    where: { sessionKey },
    data: patch,
  });
}
