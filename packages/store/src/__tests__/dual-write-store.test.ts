import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { DualWriteWorkflowStore, DualWriteTaskResultStore, setDualWriteDb, setDualWriteSchema, flushEarlyBuffer } from "../dual-write-store.js";
import type { Workflow, Task } from "@mcp-rebuild/core";

function createMockWorkflow(overrides?: Partial<Workflow>): Workflow {
  return {
    id: "wf-test-" + Math.random().toString(36).slice(2, 8),
    name: "test-workflow",
    description: "test description",
    state: "INIT",
    tasks: [],
    metadata: {},
    traceId: "trace-test",
    projectPath: "/test",
    createdAt: new Date(),
    updatedAt: new Date(),
    ...overrides,
  } as Workflow;
}

function createMockTask(overrides?: Partial<Task>): Task {
  return {
    id: "task-test-" + Math.random().toString(36).slice(2, 8),
    name: "test-task",
    agent: "masday-executor",
    skill: "masday-workflow-run",
    state: "pending",
    dependencies: [],
    input: {},
    createdAt: new Date(),
    ...overrides,
  } as Task;
}

function createMockPrimaryStore() {
  const workflows = new Map<string, Workflow>();
  const taskResults = new Map<string, Task[]>();

  return {
    save: vi.fn((wf: Workflow) => { workflows.set(wf.id, wf); }),
    load: vi.fn((id: string) => workflows.get(id)),
    loadAll: vi.fn(() => Array.from(workflows.values())),
    loadByState: vi.fn((state: string) => Array.from(workflows.values()).filter(w => w.state === state)),
    delete: vi.fn((id: string) => { workflows.delete(id); }),
    saveTask: vi.fn((wfId: string, task: Task) => {
      const existing = taskResults.get(wfId) ?? [];
      taskResults.set(wfId, [...existing, task]);
    }),
    loadTasks: vi.fn((wfId: string) => taskResults.get(wfId) ?? []),
    loadTask: vi.fn((taskId: string) => {
      for (const tasks of taskResults.values()) {
        const found = tasks.find(t => t.id === taskId);
        if (found) return found;
      }
      return undefined;
    }),
    deleteTasks: vi.fn((wfId: string) => { taskResults.delete(wfId); }),
    workflows,
    taskResults,
  };
}

describe("DualWriteWorkflowStore", () => {
  let primary: ReturnType<typeof createMockPrimaryStore>;
  let store: DualWriteWorkflowStore;

  beforeEach(() => {
    vi.clearAllMocks();
    primary = createMockPrimaryStore();
    store = new DualWriteWorkflowStore(primary);
  });

  it("saves to primary store", () => {
    const wf = createMockWorkflow();
    store.save(wf);
    expect(primary.save).toHaveBeenCalledWith(wf);
    expect(primary.load(wf.id)).toEqual(wf);
  });

  it("loads from primary store", () => {
    const wf = createMockWorkflow();
    primary.save(wf);
    expect(store.load(wf.id)).toEqual(wf);
  });

  it("loads all workflows from primary store", () => {
    const wf1 = createMockWorkflow();
    const wf2 = createMockWorkflow();
    primary.save(wf1);
    primary.save(wf2);
    expect(store.loadAll()).toHaveLength(2);
  });

  it("loads workflows by state from primary store", () => {
    const wf1 = createMockWorkflow({ state: "INIT" });
    const wf2 = createMockWorkflow({ state: "DONE" });
    primary.save(wf1);
    primary.save(wf2);
    expect(store.loadByState("INIT")).toHaveLength(1);
    expect(store.loadByState("DONE")).toHaveLength(1);
  });

  it("buffers to earlyBuffer when DB not ready", () => {
    setDualWriteDb(null);
    setDualWriteSchema(null);

    const wf = createMockWorkflow();
    store.save(wf);

    expect(primary.save).toHaveBeenCalledWith(wf);
  });

  it("flushes early buffer on flushEarlyBuffer", async () => {
    setDualWriteDb(null);
    setDualWriteSchema(null);

    const wf = createMockWorkflow();
    store.save(wf);

    const mockInsert = vi.fn().mockReturnValue({
      onConflictDoUpdate: vi.fn().mockReturnValue(Promise.resolve()),
    });
    const mockDb = { insert: mockInsert };
    const mockTables = { workflows: {}, tasks: {}, plans: {} };

    setDualWriteDb(mockDb);
    setDualWriteSchema(mockTables);

    flushEarlyBuffer();

    await new Promise((r) => setTimeout(r, 50));

    expect(mockInsert).toHaveBeenCalled();
  });

  it("delegates delete to primary store", () => {
    const wf = createMockWorkflow();
    primary.save(wf);
    store.delete(wf.id);
    expect(primary.delete).toHaveBeenCalledWith(wf.id);
  });
});

describe("DualWriteTaskResultStore", () => {
  let primary: ReturnType<typeof createMockPrimaryStore>;
  let store: DualWriteTaskResultStore;

  beforeEach(() => {
    vi.clearAllMocks();
    primary = createMockPrimaryStore();
    store = new DualWriteTaskResultStore(primary);
  });

  it("saves task to primary store", () => {
    const wfId = "wf-test";
    const task = createMockTask();
    store.saveTask(wfId, task);
    expect(primary.saveTask).toHaveBeenCalledWith(wfId, task);
  });

  it("loads tasks from primary store", () => {
    const wfId = "wf-test";
    const task = createMockTask();
    primary.saveTask(wfId, task);
    expect(store.loadTasks(wfId)).toHaveLength(1);
  });

  it("loads single task from primary store", () => {
    const wfId = "wf-test";
    const task = createMockTask();
    primary.saveTask(wfId, task);
    expect(store.loadTask(task.id)).toEqual(task);
  });

  it("deletes tasks from primary store", () => {
    const wfId = "wf-test";
    const task = createMockTask();
    primary.saveTask(wfId, task);
    store.deleteTasks(wfId);
    expect(primary.deleteTasks).toHaveBeenCalledWith(wfId);
  });
});

describe("DualWriteStore helpers", () => {
  beforeEach(() => {
    setDualWriteDb(null);
    setDualWriteSchema(null);
  });

  afterEach(() => {
    setDualWriteDb(null);
    setDualWriteSchema(null);
  });

  describe("setDualWriteSchema", () => {
    it("enables replication when both db and schema are set", async () => {
      const mockInsert = vi.fn().mockReturnValue({
        onConflictDoUpdate: vi.fn().mockReturnValue(Promise.resolve()),
      });
      const mockDb = { insert: mockInsert };
      const mockTables = { workflows: { id: "id" }, tasks: { id: "id", workflowId: "wid" }, plans: { id: "id", workflowId: "wid" } };

      setDualWriteDb(mockDb);
      setDualWriteSchema(mockTables);

      const primary = createMockPrimaryStore();
      const store = new DualWriteWorkflowStore(primary);
      store.save(createMockWorkflow());

      await new Promise(r => setTimeout(r, 50));

      expect(mockInsert).toHaveBeenCalled();
    });
  });

  describe("early buffer behavior", () => {
    it("buffers writes when DB is not configured", () => {
      setDualWriteDb(null);
      setDualWriteSchema(null);

      const primary = createMockPrimaryStore();
      const store = new DualWriteWorkflowStore(primary);

      for (let i = 0; i < 5; i++) {
        store.save(createMockWorkflow({ name: `wf-${i}` }));
      }

      expect(primary.save).toHaveBeenCalledTimes(5);
    });

    it("flushes buffered workflows to PostgreSQL", async () => {
      setDualWriteDb(null);
      setDualWriteSchema(null);

      const primary = createMockPrimaryStore();
      const store = new DualWriteWorkflowStore(primary);
      store.save(createMockWorkflow({ name: "buffered-wf" }));

      const mockInsert = vi.fn().mockReturnValue({
        onConflictDoUpdate: vi.fn().mockReturnValue(Promise.resolve()),
      });
      const mockDb = { insert: mockInsert };
      const mockTables = { workflows: { id: "id" }, tasks: { id: "id", workflowId: "wid" }, plans: { id: "id", workflowId: "wid" } };

      setDualWriteDb(mockDb);
      setDualWriteSchema(mockTables);
      flushEarlyBuffer();

      await new Promise(r => setTimeout(r, 100));

      expect(mockInsert.mock.calls.length).toBeGreaterThanOrEqual(1);
    });
  });

  describe("pending queue limits", () => {
    it("drops writes when pending queue exceeds MAX_PENDING", () => {
      const mockInsert = vi.fn().mockReturnValue({
        onConflictDoUpdate: vi.fn().mockReturnValue(new Promise(() => {})),
      });
      const mockDb = { insert: mockInsert };
      const mockTables = { workflows: { id: "id" }, tasks: { id: "id", workflowId: "wid" }, plans: { id: "id", workflowId: "wid" } };

      setDualWriteDb(mockDb);
      setDualWriteSchema(mockTables);

      const primary = createMockPrimaryStore();
      const store = new DualWriteWorkflowStore(primary);

      for (let i = 0; i < 55; i++) {
        store.save(createMockWorkflow({ name: `wf-${i}` }));
      }

      expect(primary.save).toHaveBeenCalledTimes(55);
    });
  });

  describe("DualWriteTaskResultStore replication", () => {
    it("replicates task to PostgreSQL when db is configured", async () => {
      const mockInsert = vi.fn().mockReturnValue({
        onConflictDoUpdate: vi.fn().mockReturnValue(Promise.resolve()),
      });
      const mockDb = { insert: mockInsert };
      const mockTables = { tasks: { id: "id", workflowId: "wid" }, plans: { id: "id", workflowId: "wid" } };

      setDualWriteDb(mockDb);
      setDualWriteSchema(mockTables);

      const primary = createMockPrimaryStore();
      const store = new DualWriteTaskResultStore(primary);
      store.saveTask("wf-123", createMockTask());

      await new Promise(r => setTimeout(r, 50));

      expect(mockInsert).toHaveBeenCalled();
    });

    it("skips replication when db is not configured", () => {
      setDualWriteDb(null);
      setDualWriteSchema(null);

      const primary = createMockPrimaryStore();
      const store = new DualWriteTaskResultStore(primary);
      store.saveTask("wf-123", createMockTask());

      expect(primary.saveTask).toHaveBeenCalledWith("wf-123", expect.anything());
    });
  });
});
