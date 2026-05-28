import { describe, it, expect } from "vitest";

describe("db client config", () => {
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
});
