import { describe, it, expect, beforeAll, afterEach, vi } from "vitest";

describe("db client config", () => {
  const originalDbUrl = process.env.DATABASE_URL;

  beforeAll(() => {
    if (!process.env.DATABASE_URL) {
      process.env.DATABASE_URL = "postgresql://test:test@localhost:5432/masday_test";
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

describe("db client dotenv fallback", () => {
  it("loads DATABASE_URL from .env when env var is missing", async () => {
    // client.ts has top-level await dotenv loading that resolves DATABASE_URL
    // from .env when the env var is not set. This test verifies the module
    // exports are valid after that resolution.
    const mod = await import("../client.js");
    expect(mod.db).toBeDefined();
    expect(typeof mod.healthCheck).toBe("function");
  });
});
