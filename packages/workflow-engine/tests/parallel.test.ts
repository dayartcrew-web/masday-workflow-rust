import { describe, it, expect, vi, beforeEach } from 'vitest';
import {
  createParallelBranches,
  listParallelBranches,
  completeParallelBranch,
  setExecutionMode,
  markSynthesisReady,
  markVerificationReady,
} from '../src/parallel';

vi.mock('drizzle-orm', () => ({
  eq: (col: any, val: any) => ({ col, val, op: 'eq' }),
  and: (...conds: any[]) => ({ conds, op: 'and' }),
  asc: (col: any) => ({ col, dir: 'asc' }),
  desc: (col: any) => ({ col, dir: 'desc' }),
}));

const mockParallelBranches = {
  id: 'id', workflowId: 'workflowId', taskId: 'taskId', branchKey: 'branchKey',
  role: 'role', status: 'status', input: 'input', output: 'output', createdAt: 'createdAt',
};
const mockSessionStates = {
  sessionKey: 'sessionKey', executionMode: 'executionMode',
  synthesisReady: 'synthesisReady', verificationReady: 'verificationReady',
};

const mockDb = { insert: vi.fn(), select: vi.fn(), update: vi.fn() };

vi.mock('@mcp-rebuild/db', () => ({
  get db() { return mockDb; },
  parallelBranches: mockParallelBranches,
  sessionStates: mockSessionStates,
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

describe('ParallelBranch', () => {
  beforeEach(() => { vi.clearAllMocks(); });

  function setupInsert() {
    mockDb.insert.mockReturnValue({ values: vi.fn().mockResolvedValue(undefined) });
  }
  function setupSelect(result: any) {
    mockDb.select.mockReturnValue(selectChain(result));
  }
  function setupUpdate(returnValue: any) {
    const setFn = vi.fn().mockReturnValue({
      where: vi.fn().mockReturnValue({ returning: vi.fn().mockResolvedValue([returnValue]) }),
    });
    mockDb.update.mockReturnValue({ set: setFn });
    return setFn;
  }

  describe('createParallelBranches', () => {
    it('creates branches and returns them sorted by createdAt', async () => {
      const mockResult = [
        { id: '1', workflowId: 'wf-1', taskId: 't-1', branchKey: 'backend', role: 'executor', status: 'pending', createdAt: new Date('2024-01-01') },
        { id: '2', workflowId: 'wf-1', taskId: 't-1', branchKey: 'frontend', role: 'executor', status: 'pending', createdAt: new Date('2024-01-02') },
      ];
      setupInsert();
      setupSelect(mockResult);

      const result = await createParallelBranches({
        workflowId: 'wf-1', taskId: 't-1',
        branches: [
          { branchKey: 'backend', role: 'executor', input: {} },
          { branchKey: 'frontend', role: 'executor', input: {} },
        ],
      });

      expect(mockDb.insert).toHaveBeenCalledTimes(2);
      expect(result).toEqual(mockResult);
    });

    it('creates a single branch', async () => {
      setupInsert();
      setupSelect([{ id: '1' }]);

      const result = await createParallelBranches({
        workflowId: 'wf-1', taskId: 't-1',
        branches: [{ branchKey: 'solo', role: 'executor', input: { key: 'value' } }],
      });

      expect(mockDb.insert).toHaveBeenCalledTimes(1);
      expect(result).toHaveLength(1);
    });

    it('creates zero branches when array is empty', async () => {
      setupSelect([]);

      const result = await createParallelBranches({
        workflowId: 'wf-1', taskId: 't-1', branches: [],
      });

      expect(mockDb.insert).not.toHaveBeenCalled();
      expect(result).toEqual([]);
    });

    it('passes input data through to db', async () => {
      const valuesSpy = vi.fn().mockResolvedValue(undefined);
      mockDb.insert.mockReturnValue({ values: valuesSpy });
      setupSelect([{ id: '1' }]);

      await createParallelBranches({
        workflowId: 'wf-1', taskId: 't-1',
        branches: [{ branchKey: 'data-test', role: 'qa', input: { scope: 'packages/core' } }],
      });

      expect(valuesSpy).toHaveBeenCalledWith(expect.objectContaining({ input: { scope: 'packages/core' } }));
    });
  });

  describe('listParallelBranches', () => {
    it('returns branches sorted by createdAt', async () => {
      const branches = [
        { id: '1', branchKey: 'first', createdAt: new Date('2024-01-01') },
        { id: '2', branchKey: 'second', createdAt: new Date('2024-01-02') },
      ];
      setupSelect(branches);

      const result = await listParallelBranches('wf-1', 't-1');

      expect(result).toEqual(branches);
    });

    it('returns empty array when no branches exist', async () => {
      setupSelect([]);

      const result = await listParallelBranches('wf-empty', 't-empty');

      expect(result).toEqual([]);
    });
  });

  describe('completeParallelBranch', () => {
    it('sets status to completed and stores output', async () => {
      const updated = { id: '1', status: 'completed', output: { files: ['a.ts'] } };
      const setFn = setupUpdate(updated);

      const result = await completeParallelBranch({ branchId: '1', output: { files: ['a.ts'] } });

      expect(setFn).toHaveBeenCalledWith({ status: 'completed', output: { files: ['a.ts'] } });
      expect(result).toEqual(updated);
    });

    it('stores complex output objects', async () => {
      const output = { summary: 'done', metrics: { tests: 10, passed: 10 } };
      const setFn = setupUpdate({ id: '1', output });

      await completeParallelBranch({ branchId: '1', output });

      expect(setFn).toHaveBeenCalledWith(expect.objectContaining({ output }));
    });
  });

  describe('setExecutionMode', () => {
    it('sets mode to parallel', async () => {
      const setFn = setupUpdate({ executionMode: 'parallel' });
      const result = await setExecutionMode('session-1', 'parallel');

      expect(setFn).toHaveBeenCalledWith({ executionMode: 'parallel' });
      expect(result).toEqual({ executionMode: 'parallel' });
    });

    it('sets mode to sequential', async () => {
      const setFn = setupUpdate({ executionMode: 'sequential' });
      const result = await setExecutionMode('session-1', 'sequential');

      expect(setFn).toHaveBeenCalledWith({ executionMode: 'sequential' });
      expect(result).toEqual({ executionMode: 'sequential' });
    });
  });

  describe('markSynthesisReady', () => {
    it('sets synthesisReady to true', async () => {
      const setFn = setupUpdate({ synthesisReady: true });
      const result = await markSynthesisReady('session-1', true);

      expect(setFn).toHaveBeenCalledWith({ synthesisReady: true });
      expect(result).toEqual({ synthesisReady: true });
    });

    it('sets synthesisReady to false', async () => {
      const setFn = setupUpdate({ synthesisReady: false });
      const result = await markSynthesisReady('session-1', false);

      expect(setFn).toHaveBeenCalledWith({ synthesisReady: false });
      expect(result).toEqual({ synthesisReady: false });
    });
  });

  describe('markVerificationReady', () => {
    it('sets verificationReady to true', async () => {
      const setFn = setupUpdate({ verificationReady: true });
      const result = await markVerificationReady('session-1', true);

      expect(setFn).toHaveBeenCalledWith({ verificationReady: true });
      expect(result).toEqual({ verificationReady: true });
    });

    it('sets verificationReady to false', async () => {
      const setFn = setupUpdate({ verificationReady: false });
      const result = await markVerificationReady('session-1', false);

      expect(setFn).toHaveBeenCalledWith({ verificationReady: false });
      expect(result).toEqual({ verificationReady: false });
    });
  });
});
