import { describe, it, expect, vi, beforeEach } from 'vitest';
import { getOrCreateSessionState, patchSessionState } from '../src/session.js';
import { validateExecution, validateParallelCompletion } from '../src/policy.js';

vi.mock('drizzle-orm', () => ({
  eq: (col: any, val: any) => ({ col, val, op: 'eq' }),
  and: (...conds: any[]) => ({ conds, op: 'and' }),
  asc: (col: any) => ({ col, dir: 'asc' }),
  desc: (col: any) => ({ col, dir: 'desc' }),
}));

const mockSessionStates = {
  sessionKey: 'sessionKey', executionMode: 'executionMode',
  synthesisReady: 'synthesisReady', verificationReady: 'verificationReady',
  workflowLoaded: 'workflowLoaded', planLoaded: 'planLoaded',
  taskLoaded: 'taskLoaded', contextLoaded: 'contextLoaded',
};
const mockWorkflows = { id: 'id', currentTaskId: 'currentTaskId' };
const mockParallelBranches = { id: 'id', workflowId: 'workflowId', taskId: 'taskId', status: 'status' };
const mockReviewDecisions = { workflowId: 'workflowId', taskId: 'taskId', decision: 'decision', createdAt: 'createdAt', testsVerified: 'testsVerified' };
const mockTasks = { id: 'id', title: 'title', acceptanceCriteria: 'acceptanceCriteria', requiredContext: 'requiredContext', requiresTdd: 'requiresTdd', testEvidence: 'testEvidence' };

const mockDb = { insert: vi.fn(), select: vi.fn(), update: vi.fn() };

vi.mock('@mcp-rebuild/db', () => ({
  get db() { return mockDb; },
  sessionStates: mockSessionStates,
  workflows: mockWorkflows,
  parallelBranches: mockParallelBranches,
  reviewDecisions: mockReviewDecisions,
  tasks: mockTasks,
}));

function createResolvable(result: any) {
  return {
    then: (resolve: any, reject?: any) => Promise.resolve(result).then(resolve, reject),
    orderBy: vi.fn().mockImplementation(() => createResolvable(result)),
    limit: vi.fn().mockImplementation(() => createResolvable(result)),
  };
}

function selectChain(result: any) {
  return { from: vi.fn().mockReturnValue({ where: vi.fn().mockReturnValue(createResolvable(result)) }) };
}

describe('SessionState', () => {
  beforeEach(() => { vi.clearAllMocks(); });

  // ─── getOrCreateSessionState ───

  describe('getOrCreateSessionState', () => {
    it('returns existing session when found', async () => {
      const existing = { sessionKey: 'key-1', workflowLoaded: true };
      mockDb.select.mockReturnValueOnce(selectChain([existing]));

      const result = await getOrCreateSessionState('key-1');

      expect(result).toBe(existing);
      expect(mockDb.insert).not.toHaveBeenCalled();
    });

    it('creates new session when not found', async () => {
      mockDb.select.mockReturnValueOnce(selectChain([]));
      const created = { sessionKey: 'key-new', workflowLoaded: false };
      mockDb.insert.mockReturnValueOnce({
        values: vi.fn().mockReturnValue({ returning: vi.fn().mockResolvedValue([created]) }),
      });

      const result = await getOrCreateSessionState('key-new');

      expect(result).toBe(created);
    });

    it('creates session with only sessionKey, no extra fields', async () => {
      mockDb.select.mockReturnValueOnce(selectChain([]));
      const valuesMock = vi.fn().mockReturnValue({
        returning: vi.fn().mockResolvedValue([{ sessionKey: 'key-minimal' }]),
      });
      mockDb.insert.mockReturnValueOnce({ values: valuesMock });

      await getOrCreateSessionState('key-minimal');

      expect(valuesMock).toHaveBeenCalledWith({ sessionKey: 'key-minimal' });
    });
  });

  // ─── patchSessionState ───

  describe('patchSessionState', () => {
    it('ensures session exists before patching', async () => {
      mockDb.select.mockReturnValueOnce(selectChain([]));
      mockDb.insert.mockReturnValueOnce({
        values: vi.fn().mockReturnValue({ returning: vi.fn().mockResolvedValue([{ sessionKey: 'key-1' }]) }),
      });
      mockDb.update.mockReturnValue({
        set: vi.fn().mockReturnValue({
          where: vi.fn().mockReturnValue({ returning: vi.fn().mockResolvedValue([{ sessionKey: 'key-1', workflowLoaded: true }]) }),
        }),
      });

      await patchSessionState('key-1', { workflowLoaded: true });

      expect(mockDb.select).toHaveBeenCalled();
      expect(mockDb.insert).toHaveBeenCalled();
    });

    it('patches multiple fields at once', async () => {
      mockDb.select.mockReturnValueOnce(selectChain([{ sessionKey: 'key-1' }]));
      const setFn = vi.fn().mockReturnValue({
        where: vi.fn().mockReturnValue({
          returning: vi.fn().mockResolvedValue([{ sessionKey: 'key-1', workflowLoaded: true, planLoaded: true, taskLoaded: true }]),
        }),
      });
      mockDb.update.mockReturnValue({ set: setFn });

      await patchSessionState('key-1', { workflowLoaded: true, planLoaded: true, taskLoaded: true });

      expect(setFn).toHaveBeenCalledWith({ workflowLoaded: true, planLoaded: true, taskLoaded: true });
    });

    it('patches executionMode', async () => {
      mockDb.select.mockReturnValueOnce(selectChain([{ sessionKey: 'key-1' }]));
      const setFn = vi.fn().mockReturnValue({
        where: vi.fn().mockReturnValue({ returning: vi.fn().mockResolvedValue([{ executionMode: 'parallel' }]) }),
      });
      mockDb.update.mockReturnValue({ set: setFn });

      await patchSessionState('key-1', { executionMode: 'parallel' });

      expect(setFn).toHaveBeenCalledWith({ executionMode: 'parallel' });
    });

    it('patches contextFingerprint', async () => {
      mockDb.select.mockReturnValueOnce(selectChain([{ sessionKey: 'key-1' }]));
      const setFn = vi.fn().mockReturnValue({
        where: vi.fn().mockReturnValue({ returning: vi.fn().mockResolvedValue([{ contextFingerprint: 'abc123' }]) }),
      });
      mockDb.update.mockReturnValue({ set: setFn });

      await patchSessionState('key-1', { contextFingerprint: 'abc123' });

      expect(setFn).toHaveBeenCalledWith({ contextFingerprint: 'abc123' });
    });

    it('patches activeBranchIds', async () => {
      mockDb.select.mockReturnValueOnce(selectChain([{ sessionKey: 'key-1' }]));
      const setFn = vi.fn().mockReturnValue({
        where: vi.fn().mockReturnValue({ returning: vi.fn().mockResolvedValue([{ activeBranchIds: ['branch-1', 'branch-2'] }]) }),
      });
      mockDb.update.mockReturnValue({ set: setFn });

      await patchSessionState('key-1', { activeBranchIds: ['branch-1', 'branch-2'] });

      expect(setFn).toHaveBeenCalledWith({ activeBranchIds: ['branch-1', 'branch-2'] });
    });

    it('patches metadata object', async () => {
      mockDb.select.mockReturnValueOnce(selectChain([{ sessionKey: 'key-1' }]));
      const setFn = vi.fn().mockReturnValue({
        where: vi.fn().mockReturnValue({ returning: vi.fn().mockResolvedValue([{ metadata: { lastCommand: 'workflow_execute' } }]) }),
      });
      mockDb.update.mockReturnValue({ set: setFn });

      await patchSessionState('key-1', { metadata: { lastCommand: 'workflow_execute' } });

      expect(setFn).toHaveBeenCalledWith({ metadata: { lastCommand: 'workflow_execute' } });
    });

    it('returns the updated session', async () => {
      mockDb.select.mockReturnValueOnce(selectChain([{ sessionKey: 'key-1' }]));
      const expected = { sessionKey: 'key-1', workflowLoaded: true, planLoaded: false };
      mockDb.update.mockReturnValue({
        set: vi.fn().mockReturnValue({
          where: vi.fn().mockReturnValue({ returning: vi.fn().mockResolvedValue([expected]) }),
        }),
      });

      const result = await patchSessionState('key-1', { workflowLoaded: true });

      expect(result).toEqual(expected);
    });
  });

  // ─── validateExecution (SessionState gate) ───

  describe('validateExecution', () => {
    const baseInput = { workflowId: 'wf-1', taskId: 'task-1', sessionKey: 'session-1' };

    it('passes when session state is complete', async () => {
      mockDb.select
        .mockReturnValueOnce(selectChain([{ id: 'wf-1', currentTaskId: 'task-1' }]))
        .mockReturnValueOnce(selectChain([{ sessionKey: 'session-1', workflowLoaded: true, planLoaded: true, taskLoaded: true, contextLoaded: true }]));

      const result = await validateExecution(baseInput);

      expect(result).toEqual({ ok: true });
    });

    it('fails when workflowLoaded is false', async () => {
      mockDb.select
        .mockReturnValueOnce(selectChain([{ id: 'wf-1', currentTaskId: 'task-1' }]))
        .mockReturnValueOnce(selectChain([{ sessionKey: 'session-1', workflowLoaded: false, planLoaded: true, taskLoaded: true, contextLoaded: true }]));

      await expect(validateExecution(baseInput)).rejects.toThrow('Execution blocked: session state is incomplete');
    });

    it('fails when planLoaded is false', async () => {
      mockDb.select
        .mockReturnValueOnce(selectChain([{ id: 'wf-1', currentTaskId: 'task-1' }]))
        .mockReturnValueOnce(selectChain([{ sessionKey: 'session-1', workflowLoaded: true, planLoaded: false, taskLoaded: true, contextLoaded: true }]));

      await expect(validateExecution(baseInput)).rejects.toThrow('Execution blocked: session state is incomplete');
    });

    it('fails when taskLoaded is false', async () => {
      mockDb.select
        .mockReturnValueOnce(selectChain([{ id: 'wf-1', currentTaskId: 'task-1' }]))
        .mockReturnValueOnce(selectChain([{ sessionKey: 'session-1', workflowLoaded: true, planLoaded: true, taskLoaded: false, contextLoaded: true }]));

      await expect(validateExecution(baseInput)).rejects.toThrow('Execution blocked: session state is incomplete');
    });

    it('fails when contextLoaded is false', async () => {
      mockDb.select
        .mockReturnValueOnce(selectChain([{ id: 'wf-1', currentTaskId: 'task-1' }]))
        .mockReturnValueOnce(selectChain([{ sessionKey: 'session-1', workflowLoaded: true, planLoaded: true, taskLoaded: true, contextLoaded: false }]));

      await expect(validateExecution(baseInput)).rejects.toThrow('Execution blocked: session state is incomplete');
    });

    it('fails when session is null', async () => {
      mockDb.select
        .mockReturnValueOnce(selectChain([{ id: 'wf-1', currentTaskId: 'task-1' }]))
        .mockReturnValueOnce(selectChain([]));

      await expect(validateExecution(baseInput)).rejects.toThrow('Execution blocked: session state is incomplete');
    });

    it('fails when task is not current active task', async () => {
      mockDb.select
        .mockReturnValueOnce(selectChain([{ id: 'wf-1', currentTaskId: 'task-2' }]))
        .mockReturnValueOnce(selectChain([{ sessionKey: 'session-1', workflowLoaded: true, planLoaded: true, taskLoaded: true, contextLoaded: true }]));

      await expect(validateExecution(baseInput)).rejects.toThrow('Execution blocked: task is not current active task');
    });
  });

  // ─── validateParallelCompletion (SessionState gate) ───

  describe('validateParallelCompletion', () => {
    const baseInput = { workflowId: 'wf-1', taskId: 'task-1', sessionKey: 'session-1' };

    it('skips validation when executionMode is not parallel', async () => {
      mockDb.select.mockReturnValueOnce(selectChain([{ sessionKey: 'session-1', executionMode: 'sequential' }]));

      const result = await validateParallelCompletion(baseInput);

      expect(result).toEqual({ ok: true, skipped: true });
    });

    it('skips validation when executionMode is null', async () => {
      mockDb.select.mockReturnValueOnce(selectChain([{ sessionKey: 'session-1', executionMode: null }]));

      const result = await validateParallelCompletion(baseInput);

      expect(result).toEqual({ ok: true, skipped: true });
    });

    it('fails when branches are not all completed', async () => {
      mockDb.select
        .mockReturnValueOnce(selectChain([{ sessionKey: 'session-1', executionMode: 'parallel' }]))
        .mockReturnValueOnce(selectChain([{ id: 'b1', status: 'completed' }, { id: 'b2', status: 'pending' }]));

      await expect(validateParallelCompletion(baseInput)).rejects.toThrow('Parallel completion blocked: some branches are not completed');
    });

    it('fails when synthesis is not ready', async () => {
      mockDb.select
        .mockReturnValueOnce(selectChain([{ sessionKey: 'session-1', executionMode: 'parallel', synthesisReady: false }]))
        .mockReturnValueOnce(selectChain([{ id: 'b1', status: 'completed' }]));

      await expect(validateParallelCompletion(baseInput)).rejects.toThrow('Parallel completion blocked: synthesis is not ready');
    });

    it('fails when verification is not ready', async () => {
      mockDb.select
        .mockReturnValueOnce(selectChain([{ sessionKey: 'session-1', executionMode: 'parallel', synthesisReady: true, verificationReady: false }]))
        .mockReturnValueOnce(selectChain([{ id: 'b1', status: 'completed' }]));

      await expect(validateParallelCompletion(baseInput)).rejects.toThrow('Parallel completion blocked: verification is not ready');
    });

    it('fails when review is not approved', async () => {
      mockDb.select
        .mockReturnValueOnce(selectChain([{ sessionKey: 'session-1', executionMode: 'parallel', synthesisReady: true, verificationReady: true }]))
        .mockReturnValueOnce(selectChain([{ id: 'b1', status: 'completed' }]))
        .mockReturnValueOnce(selectChain([]));

      await expect(validateParallelCompletion(baseInput)).rejects.toThrow('Parallel completion blocked: review is not approved');
    });

    it('passes when all conditions are met', async () => {
      mockDb.select
        .mockReturnValueOnce(selectChain([{ sessionKey: 'session-1', executionMode: 'parallel', synthesisReady: true, verificationReady: true }]))
        .mockReturnValueOnce(selectChain([{ id: 'b1', status: 'completed' }, { id: 'b2', status: 'completed' }]))
        .mockReturnValueOnce(selectChain([{ decision: 'APPROVED' }]));

      const result = await validateParallelCompletion(baseInput);

      expect(result).toEqual({ ok: true });
    });

    it('passes with empty branches array', async () => {
      mockDb.select
        .mockReturnValueOnce(selectChain([{ sessionKey: 'session-1', executionMode: 'parallel', synthesisReady: true, verificationReady: true }]))
        .mockReturnValueOnce(selectChain([]))
        .mockReturnValueOnce(selectChain([{ decision: 'APPROVED' }]));

      const result = await validateParallelCompletion(baseInput);

      expect(result).toEqual({ ok: true });
    });
  });
});
