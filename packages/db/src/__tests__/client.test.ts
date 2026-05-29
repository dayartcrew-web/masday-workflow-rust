import { describe, it, expect, beforeAll, afterEach, vi } from "vitest";

describe("db client config", () => {
  const originalDbUrl = process.env.DATABASE_URL;

  beforeAll(() => {
    if (!process.env.DATABASE_URL) {
      process.env.DATABASE_URL = "postgresql://USER:PASS@localhost:5432/masday_test";
    }
  });

  afterEach(() => {
    if (originalDbUrl) {
      process.env.DATABASE_URL = originalDbUrl;
    }
  });

  it("exports db as a drizzle instance", async () => {
    const mod = await import("../client.js");
    expect(mod.db).toBeDefined();
    expect(typeof mod.db.select).toBe("function");
    expect(typeof mod.db.insert).toBe("function");
    expect(typeof mod.db.update).toBe("function");
    expect(typeof mod.db.delete).toBe("function");
  });

  it("exports healthCheck as an async function", async () => {
    const mod = await import("../client.js");
    expect(typeof mod.healthCheck).toBe("function");
    expect(mod.healthCheck.constructor.name).toBe("AsyncFunction");
  });

  it("exports disconnectDb as an async function", async () => {
    const mod = await import("../client.js");
    expect(typeof mod.disconnectDb).toBe("function");
    expect(mod.disconnectDb.constructor.name).toBe("AsyncFunction");
  });

  it("healthCheck returns a boolean", async () => {
    const { healthCheck } = await import("../client.js");
    const result = await healthCheck(3000);
    expect(typeof result).toBe("boolean");
  });

  it("healthCheck respects custom timeout", async () => {
    const { healthCheck } = await import("../client.js");
    const start = Date.now();
    await healthCheck(100);
    const elapsed = Date.now() - start;
    expect(elapsed).toBeLessThan(5000);
  });

  it("disconnectDb resolves without error", async () => {
    const { disconnectDb } = await import("../client.js");
    await expect(disconnectDb()).resolves.toBeUndefined();
  });
});

describe("db client throws without DATABASE_URL", () => {
  it("should throw at import time when DATABASE_URL is not set", async () => {
    const originalUrl = process.env.DATABASE_URL;
    delete process.env.DATABASE_URL;

    try {
      vi.resetModules();
      await expect(import("../client.js")).rejects.toThrow("DATABASE_URL");
    } finally {
      if (originalUrl) process.env.DATABASE_URL = originalUrl;
      vi.resetModules();
    }
  });
});
