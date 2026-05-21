import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { getOrCreateSessionState, patchSessionState } from '../src/session.js';
import { validateExecution, validateParallelCompletion } from '../src/policy.js';

// Mock the db module
const mockPrisma = {
  sessionState: {
    findUnique: vi.fn(),
    findUniqueOrThrow: vi.fn(),
    create: vi.fn(),
    update: vi.fn(),
  },
  workflow: {
    findUniqueOrThrow: vi.fn(),
  },
  reviewDecision: {
    findFirst: vi.fn(),
  },
  task: {
    findUniqueOrThrow: vi.fn(),
  },
  parallelBranch: {
    findMany: vi.fn(),
  },
};

vi.mock('@mcp-rebuild/db', () => ({
  get prisma() {
    return mockPrisma;
  },
}));

describe('SessionState', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  afterEach(() => {
    vi.resetAllMocks();
  });

  // ─── getOrCreateSessionState ───

  describe('getOrCreateSessionState', () => {
    it('returns existing session when found', async () => {
      const existing = { sessionKey: 'key-1', workflowLoaded: true };
      mockPrisma.sessionState.findUnique.mockResolvedValue(existing);

      const result = await getOrCreateSessionState('key-1');

      expect(result).toBe(existing);
      expect(mockPrisma.sessionState.findUnique).toHaveBeenCalledWith({
        where: { sessionKey: 'key-1' },
      });
      expect(mockPrisma.sessionState.create).not.toHaveBeenCalled();
    });

    it('creates new session when not found', async () => {
      mockPrisma.sessionState.findUnique.mockResolvedValue(null);
      const created = { sessionKey: 'key-new', workflowLoaded: false };
      mockPrisma.sessionState.create.mockResolvedValue(created);

      const result = await getOrCreateSessionState('key-new');

      expect(result).toBe(created);
      expect(mockPrisma.sessionState.create).toHaveBeenCalledWith({
        data: { sessionKey: 'key-new' },
      });
    });

    it('creates session with only sessionKey, no extra fields', async () => {
      mockPrisma.sessionState.findUnique.mockResolvedValue(null);
      mockPrisma.sessionState.create.mockResolvedValue({ sessionKey: 'key-minimal' });

      await getOrCreateSessionState('key-minimal');

      expect(mockPrisma.sessionState.create).toHaveBeenCalledWith({
        data: { sessionKey: 'key-minimal' },
      });
    });
  });

  // ─── patchSessionState ───

  describe('patchSessionState', () => {
    it('ensures session exists before patching', async () => {
      mockPrisma.sessionState.findUnique.mockResolvedValue(null);
      mockPrisma.sessionState.create.mockResolvedValue({ sessionKey: 'key-1' });
      mockPrisma.sessionState.update.mockResolvedValue({ sessionKey: 'key-1', workflowLoaded: true });

      await patchSessionState('key-1', { workflowLoaded: true });

      expect(mockPrisma.sessionState.findUnique).toHaveBeenCalled();
      expect(mockPrisma.sessionState.create).toHaveBeenCalled();
      expect(mockPrisma.sessionState.update).toHaveBeenCalledWith({
        where: { sessionKey: 'key-1' },
        data: { workflowLoaded: true },
      });
    });

    it('patches multiple fields at once', async () => {
      mockPrisma.sessionState.findUnique.mockResolvedValue({ sessionKey: 'key-1' });
      mockPrisma.sessionState.update.mockResolvedValue({
        sessionKey: 'key-1',
        workflowLoaded: true,
        planLoaded: true,
        taskLoaded: true,
      });

      await patchSessionState('key-1', {
        workflowLoaded: true,
        planLoaded: true,
        taskLoaded: true,
      });

      expect(mockPrisma.sessionState.update).toHaveBeenCalledWith({
        where: { sessionKey: 'key-1' },
        data: {
          workflowLoaded: true,
          planLoaded: true,
          taskLoaded: true,
        },
      });
    });

    it('patches executionMode', async () => {
      mockPrisma.sessionState.findUnique.mockResolvedValue({ sessionKey: 'key-1' });
      mockPrisma.sessionState.update.mockResolvedValue({ executionMode: 'parallel' });

      await patchSessionState('key-1', { executionMode: 'parallel' });

      expect(mockPrisma.sessionState.update).toHaveBeenCalledWith({
        where: { sessionKey: 'key-1' },
        data: { executionMode: 'parallel' },
      });
    });

    it('patches contextFingerprint', async () => {
      mockPrisma.sessionState.findUnique.mockResolvedValue({ sessionKey: 'key-1' });
      mockPrisma.sessionState.update.mockResolvedValue({
        contextFingerprint: 'abc123',
      });

      await patchSessionState('key-1', {
        contextFingerprint: 'abc123',
      });

      expect(mockPrisma.sessionState.update).toHaveBeenCalledWith({
        where: { sessionKey: 'key-1' },
        data: { contextFingerprint: 'abc123' },
      });
    });

    it('patches activeBranchIds', async () => {
      mockPrisma.sessionState.findUnique.mockResolvedValue({ sessionKey: 'key-1' });
      mockPrisma.sessionState.update.mockResolvedValue({
        activeBranchIds: ['branch-1', 'branch-2'],
      });

      await patchSessionState('key-1', {
        activeBranchIds: ['branch-1', 'branch-2'],
      });

      expect(mockPrisma.sessionState.update).toHaveBeenCalledWith({
        where: { sessionKey: 'key-1' },
        data: { activeBranchIds: ['branch-1', 'branch-2'] },
      });
    });

    it('patches metadata object', async () => {
      mockPrisma.sessionState.findUnique.mockResolvedValue({ sessionKey: 'key-1' });
      mockPrisma.sessionState.update.mockResolvedValue({
        metadata: { lastCommand: 'workflow.execute' },
      });

      await patchSessionState('key-1', {
        metadata: { lastCommand: 'workflow.execute' },
      });

      expect(mockPrisma.sessionState.update).toHaveBeenCalledWith({
        where: { sessionKey: 'key-1' },
        data: { metadata: { lastCommand: 'workflow.execute' } },
      });
    });

    it('returns the updated session', async () => {
      mockPrisma.sessionState.findUnique.mockResolvedValue({ sessionKey: 'key-1' });
      const expected = {
        sessionKey: 'key-1',
        workflowLoaded: true,
        planLoaded: false,
      };
      mockPrisma.sessionState.update.mockResolvedValue(expected);

      const result = await patchSessionState('key-1', { workflowLoaded: true });

      expect(result).toEqual(expected);
    });
  });

  // ─── validateExecution (SessionState gate) ───

  describe('validateExecution', () => {
    const baseInput = {
      workflowId: 'wf-1',
      taskId: 'task-1',
      sessionKey: 'session-1',
    };

    it('passes when session state is complete', async () => {
      mockPrisma.workflow.findUniqueOrThrow.mockResolvedValue({
        id: 'wf-1',
        currentTaskId: 'task-1',
      });
      mockPrisma.sessionState.findUnique.mockResolvedValue({
        sessionKey: 'session-1',
        workflowLoaded: true,
        planLoaded: true,
        taskLoaded: true,
        contextLoaded: true,
      });

      const result = await validateExecution(baseInput);

      expect(result).toEqual({ ok: true });
    });

    it('fails when workflowLoaded is false', async () => {
      mockPrisma.workflow.findUniqueOrThrow.mockResolvedValue({
        id: 'wf-1',
        currentTaskId: 'task-1',
      });
      mockPrisma.sessionState.findUnique.mockResolvedValue({
        sessionKey: 'session-1',
        workflowLoaded: false,
        planLoaded: true,
        taskLoaded: true,
        contextLoaded: true,
      });

      await expect(validateExecution(baseInput)).rejects.toThrow(
        'Execution blocked: session state is incomplete',
      );
    });

    it('fails when planLoaded is false', async () => {
      mockPrisma.workflow.findUniqueOrThrow.mockResolvedValue({
        id: 'wf-1',
        currentTaskId: 'task-1',
      });
      mockPrisma.sessionState.findUnique.mockResolvedValue({
        sessionKey: 'session-1',
        workflowLoaded: true,
        planLoaded: false,
        taskLoaded: true,
        contextLoaded: true,
      });

      await expect(validateExecution(baseInput)).rejects.toThrow(
        'Execution blocked: session state is incomplete',
      );
    });

    it('fails when taskLoaded is false', async () => {
      mockPrisma.workflow.findUniqueOrThrow.mockResolvedValue({
        id: 'wf-1',
        currentTaskId: 'task-1',
      });
      mockPrisma.sessionState.findUnique.mockResolvedValue({
        sessionKey: 'session-1',
        workflowLoaded: true,
        planLoaded: true,
        taskLoaded: false,
        contextLoaded: true,
      });

      await expect(validateExecution(baseInput)).rejects.toThrow(
        'Execution blocked: session state is incomplete',
      );
    });

    it('fails when contextLoaded is false', async () => {
      mockPrisma.workflow.findUniqueOrThrow.mockResolvedValue({
        id: 'wf-1',
        currentTaskId: 'task-1',
      });
      mockPrisma.sessionState.findUnique.mockResolvedValue({
        sessionKey: 'session-1',
        workflowLoaded: true,
        planLoaded: true,
        taskLoaded: true,
        contextLoaded: false,
      });

      await expect(validateExecution(baseInput)).rejects.toThrow(
        'Execution blocked: session state is incomplete',
      );
    });

    it('fails when session is null', async () => {
      mockPrisma.workflow.findUniqueOrThrow.mockResolvedValue({
        id: 'wf-1',
        currentTaskId: 'task-1',
      });
      mockPrisma.sessionState.findUnique.mockResolvedValue(null);

      await expect(validateExecution(baseInput)).rejects.toThrow(
        'Execution blocked: session state is incomplete',
      );
    });

    it('fails when task is not current active task', async () => {
      mockPrisma.workflow.findUniqueOrThrow.mockResolvedValue({
        id: 'wf-1',
        currentTaskId: 'task-2',
      });
      mockPrisma.sessionState.findUnique.mockResolvedValue({
        sessionKey: 'session-1',
        workflowLoaded: true,
        planLoaded: true,
        taskLoaded: true,
        contextLoaded: true,
      });

      await expect(validateExecution(baseInput)).rejects.toThrow(
        'Execution blocked: task is not current active task',
      );
    });
  });

  // ─── validateParallelCompletion (SessionState gate) ───

  describe('validateParallelCompletion', () => {
    const baseInput = {
      workflowId: 'wf-1',
      taskId: 'task-1',
      sessionKey: 'session-1',
    };

    it('skips validation when executionMode is not parallel', async () => {
      mockPrisma.sessionState.findUniqueOrThrow.mockResolvedValue({
        sessionKey: 'session-1',
        executionMode: 'sequential',
      });

      const result = await validateParallelCompletion(baseInput);

      expect(result).toEqual({ ok: true, skipped: true });
      expect(mockPrisma.parallelBranch.findMany).not.toHaveBeenCalled();
    });

    it('skips validation when executionMode is null', async () => {
      mockPrisma.sessionState.findUniqueOrThrow.mockResolvedValue({
        sessionKey: 'session-1',
        executionMode: null,
      });

      const result = await validateParallelCompletion(baseInput);

      expect(result).toEqual({ ok: true, skipped: true });
    });

    it('fails when branches are not all completed', async () => {
      mockPrisma.sessionState.findUniqueOrThrow.mockResolvedValue({
        sessionKey: 'session-1',
        executionMode: 'parallel',
      });
      mockPrisma.parallelBranch.findMany.mockResolvedValue([
        { id: 'b1', status: 'completed' },
        { id: 'b2', status: 'pending' },
      ]);

      await expect(validateParallelCompletion(baseInput)).rejects.toThrow(
        'Parallel completion blocked: some branches are not completed',
      );
    });

    it('fails when synthesis is not ready', async () => {
      mockPrisma.sessionState.findUniqueOrThrow.mockResolvedValue({
        sessionKey: 'session-1',
        executionMode: 'parallel',
        synthesisReady: false,
      });
      mockPrisma.parallelBranch.findMany.mockResolvedValue([
        { id: 'b1', status: 'completed' },
      ]);

      await expect(validateParallelCompletion(baseInput)).rejects.toThrow(
        'Parallel completion blocked: synthesis is not ready',
      );
    });

    it('fails when verification is not ready', async () => {
      mockPrisma.sessionState.findUniqueOrThrow.mockResolvedValue({
        sessionKey: 'session-1',
        executionMode: 'parallel',
        synthesisReady: true,
        verificationReady: false,
      });
      mockPrisma.parallelBranch.findMany.mockResolvedValue([
        { id: 'b1', status: 'completed' },
      ]);

      await expect(validateParallelCompletion(baseInput)).rejects.toThrow(
        'Parallel completion blocked: verification is not ready',
      );
    });

    it('fails when review is not approved', async () => {
      mockPrisma.sessionState.findUniqueOrThrow.mockResolvedValue({
        sessionKey: 'session-1',
        executionMode: 'parallel',
        synthesisReady: true,
        verificationReady: true,
      });
      mockPrisma.parallelBranch.findMany.mockResolvedValue([
        { id: 'b1', status: 'completed' },
      ]);
      mockPrisma.reviewDecision.findFirst.mockResolvedValue(null);

      await expect(validateParallelCompletion(baseInput)).rejects.toThrow(
        'Parallel completion blocked: review is not approved',
      );
    });

    it('passes when all conditions are met', async () => {
      mockPrisma.sessionState.findUniqueOrThrow.mockResolvedValue({
        sessionKey: 'session-1',
        executionMode: 'parallel',
        synthesisReady: true,
        verificationReady: true,
      });
      mockPrisma.parallelBranch.findMany.mockResolvedValue([
        { id: 'b1', status: 'completed' },
        { id: 'b2', status: 'completed' },
      ]);
      mockPrisma.reviewDecision.findFirst.mockResolvedValue({
        decision: 'APPROVED',
      });
      mockPrisma.task.findUniqueOrThrow.mockResolvedValue({
        title: 'Test task',
        acceptanceCriteria: [],
        requiredContext: [],
      });

      const result = await validateParallelCompletion(baseInput);

      expect(result).toEqual({ ok: true });
    });

    it('passes with empty branches array', async () => {
      mockPrisma.sessionState.findUniqueOrThrow.mockResolvedValue({
        sessionKey: 'session-1',
        executionMode: 'parallel',
        synthesisReady: true,
        verificationReady: true,
      });
      mockPrisma.parallelBranch.findMany.mockResolvedValue([]);
      mockPrisma.reviewDecision.findFirst.mockResolvedValue({
        decision: 'APPROVED',
      });
      mockPrisma.task.findUniqueOrThrow.mockResolvedValue({
        title: 'Test task',
        acceptanceCriteria: [],
        requiredContext: [],
      });

      const result = await validateParallelCompletion(baseInput);

      expect(result).toEqual({ ok: true });
    });
  });
});
