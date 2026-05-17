import { PrismaClient } from "@prisma/client";

export const prisma = new PrismaClient({
  log: process.env.NODE_ENV === "development"
    ? ["query", "error", "warn"]
    : ["error"],
});

export async function disconnectDb(): Promise<void> {
  await prisma.$disconnect();
}

export async function healthCheck(): Promise<boolean> {
  try {
    await prisma.$queryRaw`SELECT 1`;
    return true;
  } catch {
    return false;
  }
}
