import { describe, it, expect, beforeEach, afterEach } from "vitest";
import fs from "fs";
import path from "path";
import os from "os";

interface MemRec {
  id: string;
  type: string;
  content: string;
  summary: string;
  source: string;
  importance: number;
  tags: string[];
  createdAt: number;
  projectPath?: string;
}

function createTempDir(): string {
  return fs.mkdtempSync(path.join(os.tmpdir(), "mem-scope-test-"));
}

function memFile(dir: string): string {
  return path.join(dir, "memories.json");
}

function loadMem(file: string): MemRec[] {
  if (!fs.existsSync(file)) return [];
  return JSON.parse(fs.readFileSync(file, "utf-8"));
}

function saveMem(file: string, mems: MemRec[]): void {
  fs.writeFileSync(file, JSON.stringify(mems, null, 2));
}

function loadMemProject(file: string, projectPath: string): MemRec[] {
  const all = loadMem(file);
  return all.filter(m => !m.projectPath || m.projectPath === projectPath);
}

describe("loadMemProject scoping", () => {
  let tempDir: string;
  let file: string;

  beforeEach(() => {
    tempDir = createTempDir();
    file = memFile(tempDir);
  });

  afterEach(() => {
    fs.rmSync(tempDir, { recursive: true, force: true });
  });

  it("returns empty array when file does not exist", () => {
    const result = loadMemProject(file, "/project/a");
    expect(result).toEqual([]);
  });

  it("includes memories with matching projectPath", () => {
    const mems: MemRec[] = [
      { id: "1", type: "decision", content: "test", summary: "s", source: "agent", importance: 0.5, tags: [], createdAt: Date.now(), projectPath: "/project/a" },
    ];
    saveMem(file, mems);
    const result = loadMemProject(file, "/project/a");
    expect(result).toHaveLength(1);
    expect(result[0].id).toBe("1");
  });

  it("excludes memories from different project", () => {
    const mems: MemRec[] = [
      { id: "1", type: "decision", content: "test", summary: "s", source: "agent", importance: 0.5, tags: [], createdAt: Date.now(), projectPath: "/project/b" },
    ];
    saveMem(file, mems);
    const result = loadMemProject(file, "/project/a");
    expect(result).toHaveLength(0);
  });

  it("includes legacy memories without projectPath", () => {
    const mems: MemRec[] = [
      { id: "1", type: "decision", content: "old", summary: "s", source: "agent", importance: 0.5, tags: [], createdAt: Date.now() },
    ];
    saveMem(file, mems);
    const result = loadMemProject(file, "/project/a");
    expect(result).toHaveLength(1);
  });

  it("filters correctly across mixed project sources", () => {
    const mems: MemRec[] = [
      { id: "1", type: "decision", content: "a", summary: "s", source: "agent", importance: 0.5, tags: [], createdAt: Date.now(), projectPath: "/project/a" },
      { id: "2", type: "decision", content: "b", summary: "s", source: "agent", importance: 0.5, tags: [], createdAt: Date.now(), projectPath: "/project/b" },
      { id: "3", type: "decision", content: "legacy", summary: "s", source: "agent", importance: 0.5, tags: [], createdAt: Date.now() },
    ];
    saveMem(file, mems);
    const result = loadMemProject(file, "/project/a");
    expect(result).toHaveLength(2);
    expect(result.map(m => m.id).sort()).toEqual(["1", "3"]);
  });
});

describe("memory.store adds projectPath", () => {
  let tempDir: string;
  let file: string;

  beforeEach(() => {
    tempDir = createTempDir();
    file = memFile(tempDir);
  });

  afterEach(() => {
    fs.rmSync(tempDir, { recursive: true, force: true });
  });

  it("persists memory with projectPath field", () => {
    const mem: MemRec = {
      id: "m1",
      type: "decision",
      content: "test content",
      summary: "test summary",
      source: "masday-executor",
      importance: 0.8,
      tags: ["test"],
      createdAt: Date.now(),
      projectPath: "/project/a",
    };
    const existing = loadMem(file);
    saveMem(file, [...existing, mem]);

    const loaded = loadMem(file);
    expect(loaded).toHaveLength(1);
    expect(loaded[0].projectPath).toBe("/project/a");
  });
});

describe("memory.update preserves other projects", () => {
  let tempDir: string;
  let file: string;

  beforeEach(() => {
    tempDir = createTempDir();
    file = memFile(tempDir);
  });

  afterEach(() => {
    fs.rmSync(tempDir, { recursive: true, force: true });
  });

  it("update only modifies target memory, keeps others intact", () => {
    const mems: MemRec[] = [
      { id: "1", type: "decision", content: "a", summary: "s", source: "agent", importance: 0.5, tags: [], createdAt: Date.now(), projectPath: "/project/a" },
      { id: "2", type: "decision", content: "b", summary: "s", source: "agent", importance: 0.5, tags: [], createdAt: Date.now(), projectPath: "/project/b" },
    ];
    saveMem(file, mems);

    const all = loadMem(file);
    const updated = all.map(m => m.id === "1" ? { ...m, content: "updated" } : m);
    saveMem(file, updated);

    const loaded = loadMem(file);
    expect(loaded).toHaveLength(2);
    expect(loaded.find(m => m.id === "1")!.content).toBe("updated");
    expect(loaded.find(m => m.id === "2")!.content).toBe("b");
  });
});

describe("memory.delete_by_workflow only deletes current project", () => {
  let tempDir: string;
  let file: string;

  beforeEach(() => {
    tempDir = createTempDir();
    file = memFile(tempDir);
  });

  afterEach(() => {
    fs.rmSync(tempDir, { recursive: true, force: true });
  });

  it("deletes only memories matching project and workflow tag", () => {
    const wfId = "wf-123";
    const mems: MemRec[] = [
      { id: "1", type: "decision", content: "a", summary: "s", source: "agent", importance: 0.5, tags: [wfId], createdAt: Date.now(), projectPath: "/project/a" },
      { id: "2", type: "decision", content: "b", summary: "s", source: "agent", importance: 0.5, tags: [wfId], createdAt: Date.now(), projectPath: "/project/b" },
      { id: "3", type: "decision", content: "c", summary: "s", source: "agent", importance: 0.5, tags: ["other"], createdAt: Date.now(), projectPath: "/project/a" },
    ];
    saveMem(file, mems);

    const filtered = mems.filter(m => m.projectPath !== "/project/a" || !m.tags.includes(wfId));
    saveMem(file, filtered);

    const loaded = loadMem(file);
    expect(loaded).toHaveLength(2);
    expect(loaded.find(m => m.id === "1")).toBeUndefined();
    expect(loaded.find(m => m.id === "2")).toBeDefined();
    expect(loaded.find(m => m.id === "3")).toBeDefined();
  });
});

describe("local.sync merge preserves other projects", () => {
  let tempDir: string;
  let file: string;

  beforeEach(() => {
    tempDir = createTempDir();
    file = memFile(tempDir);
  });

  afterEach(() => {
    fs.rmSync(tempDir, { recursive: true, force: true });
  });

  it("replaces current project memories but keeps other projects", () => {
    const localMems: MemRec[] = [
      { id: "1", type: "decision", content: "local-a", summary: "s", source: "agent", importance: 0.5, tags: [], createdAt: Date.now(), projectPath: "/project/a" },
      { id: "2", type: "decision", content: "local-b", summary: "s", source: "agent", importance: 0.5, tags: [], createdAt: Date.now(), projectPath: "/project/b" },
    ];
    saveMem(file, localMems);

    const cachedFromDb: MemRec[] = [
      { id: "3", type: "decision", content: "db-a", summary: "s", source: "agent", importance: 0.5, tags: [], createdAt: Date.now(), projectPath: "/project/a" },
    ];

    const existing = loadMem(file);
    const filtered = existing.filter(m => m.projectPath !== "/project/a");
    const synced = cachedFromDb.map(m => ({ ...m, projectPath: "/project/a" }));
    saveMem(file, [...filtered, ...synced]);

    const loaded = loadMem(file);
    expect(loaded).toHaveLength(2);
    const projB = loaded.filter(m => m.projectPath === "/project/b");
    const projA = loaded.filter(m => m.projectPath === "/project/a");
    expect(projB).toHaveLength(1);
    expect(projB[0].content).toBe("local-b");
    expect(projA).toHaveLength(1);
    expect(projA[0].content).toBe("db-a");
  });
});
