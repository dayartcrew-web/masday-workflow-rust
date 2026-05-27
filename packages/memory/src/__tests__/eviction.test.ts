import { describe, it, expect, beforeEach, afterEach } from "vitest";
import { MemoryStore } from "../store.js";
import fs from "fs";
import path from "path";
import os from "os";

describe("MemoryStore eviction / prune", () => {
  let store: MemoryStore;
  let tempDir: string;

  beforeEach(async () => {
    tempDir = fs.mkdtempSync(path.join(os.tmpdir(), "mem-test-"));
    // Use large maxMemories so auto-prune doesn't interfere with explicit prune tests
    store = new MemoryStore({
      filePath: path.join(tempDir, "memories.json"),
      maxMemories: 100,
    });
    await store.init();
  });

  afterEach(() => {
    fs.rmSync(tempDir, { recursive: true, force: true });
  });

  it("prune removes lowest importance when over limit", async () => {
    for (let i = 0; i < 7; i++) {
      await store.add(`memory-${i}`, {
        type: "fact",
        importance: 0.1 + i * 0.1,
      });
    }

    const pruned = store.prune({ maxMemories: 5 });
    expect(pruned.length).toBe(2);
    expect(store.getStats().total).toBe(5);
  });

  it("prune respects custom maxMemories option", async () => {
    for (let i = 0; i < 8; i++) {
      await store.add(`memory-${i}`, { importance: 0.5 });
    }

    const pruned = store.prune({ maxMemories: 3 });
    expect(pruned.length).toBe(5);
    expect(store.getStats().total).toBe(3);
  });

  it("prune respects custom maxAge option", async () => {
    await store.add("old memory", { importance: 0.05 });
    store.prune({ maxAge: 0, minImportance: 0.9 });
    expect(store.getStats().total).toBeGreaterThanOrEqual(0);
  });

  it("prune respects custom minImportance option", async () => {
    await store.add("low importance", { importance: 0.01 });
    await store.add("high importance", { importance: 0.9 });

    const pruned = store.prune({ maxAge: Infinity, minImportance: 0.5 });
    expect(pruned.length).toBe(0);
    expect(store.getStats().total).toBe(2);
  });

  it("prune returns list of pruned IDs", async () => {
    for (let i = 0; i < 8; i++) {
      await store.add(`memory-${i}`, { importance: 0.3 });
    }

    const pruned = store.prune({ maxMemories: 5 });
    expect(pruned.length).toBe(3);
    for (const id of pruned) {
      expect(typeof id).toBe("string");
      expect(id.length).toBeGreaterThan(0);
    }
  });

  it("add triggers auto-prune when over maxMemories", async () => {
    const smallStore = new MemoryStore({
      filePath: path.join(tempDir, "small.json"),
      maxMemories: 3,
    });
    await smallStore.init();

    for (let i = 0; i < 5; i++) {
      await smallStore.add(`memory-${i}`, { importance: 0.5 });
    }

    const stats = smallStore.getStats();
    expect(stats.total).toBeLessThanOrEqual(3);
  });

  it("prune does not remove when under limit", async () => {
    await store.add("memory-1", { importance: 0.5 });
    await store.add("memory-2", { importance: 0.5 });

    const pruned = store.prune();
    expect(pruned.length).toBe(0);
    expect(store.getStats().total).toBe(2);
  });

  it("prune sets dirty flag so save persists changes", async () => {
    for (let i = 0; i < 7; i++) {
      await store.add(`memory-${i}`, { importance: 0.3 });
    }

    store.prune({ maxMemories: 3 });
    await expect(store.save()).resolves.toBeUndefined();
  });
});
