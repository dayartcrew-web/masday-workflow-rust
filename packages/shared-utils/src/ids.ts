export function nowIso(): string {
  return new Date().toISOString();
}

export function generateId(): string {
  return crypto.randomUUID();
}
