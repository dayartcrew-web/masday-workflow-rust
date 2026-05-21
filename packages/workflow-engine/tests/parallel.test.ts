import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import {
  createParallelBranches,
  listParallelBranches,
  completeParallelBranch,
  setExecutionMode,
  markSynthesisReady,
  markVerificationReady,
} from '../src/parallel';

// Mock the db module
const mockPrisma = {
  parallelBranch: {
    create: vi.fn(),
    findMany: vi.fn(),
    update: vi.fn(),
  },
  sessionState: {
    update: vi.fn(),
  },
};

vi.mock('@mcp-rebuild/db', () => ({
  get prisma() {
    return mockPrisma;
  },
}));

describe('ParallelBranch', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  afterEach(() => {
    vi.resetAllMocks();
  });

  describe('createParallelBranches', () => {
    it('creates branches and returns them sorted by createdAt', async () => {
      const mockBranches = [
        { id: '1', workflowId: 'wf-1', taskId: 't-1', branchKey: 'backend', role: 'executor', status: 'pending', createdAt: new Date('2024-01-01') },
        { id: '2', workflowId: 'wf-1', taskId: 't-1', branchKey: 'frontend', role: 'executor', status: 'pending', createdAt: new Date('2024-01-02') },
      ];
      mockPrisma.parallelBranch.create.mockResolvedValue(mockBranches[0]);
      mockPrisma.parallelBranch.findMany.mockResolvedValue(mockBranches);

      const result = await createParallelBranches({
        workflowId: 'wf-1',
        taskId: 't-1',
        branches: [
          { branchKey: 'backend', role: 'executor', input: {} },
          { branchKey: 'frontend', role: 'executor', input: {} },
        ],
      });

      expect(mockPrisma.parallelBranch.create).toHaveBeenCalledTimes(2);
      expect(mockPrisma.parallelBranch.create).toHaveBeenNthCalledWith(1, {
        data: {
          workflowId: 'wf-1',
          taskId: 't-1',
          branchKey: 'backend',
          role: 'executor',
          status: 'pending',
          input: {},
        },
      });
      expect(result).toEqual(mockBranches);
    });

    it('creates a single branch', async () => {
      mockPrisma.parallelBranch.create.mockResolvedValue({ id: '1' });
      mockPrisma.parallelBranch.findMany.mockResolvedValue([{ id: '1' }]);

      const result = await createParallelBranches({
        workflowId: 'wf-1',
        taskId: 't-1',
        branches: [{ branchKey: 'solo', role: 'executor', input: { key: 'value' } }],
      });

      expect(mockPrisma.parallelBranch.create).toHaveBeenCalledTimes(1);
      expect(mockPrisma.parallelBranch.create).toHaveBeenCalledWith({
        data: {
          workflowId: 'wf-1',
          taskId: 't-1',
          branchKey: 'solo',
          role: 'executor',
          status: 'pending',
          input: { key: 'value' },
        },
      });
      expect(result).toHaveLength(1);
    });

    it('creates zero branches when array is empty', async () => {
      mockPrisma.parallelBranch.findMany.mockResolvedValue([]);

      const result = await createParallelBranches({
        workflowId: 'wf-1',
        taskId: 't-1',
        branches: [],
      });

      expect(mockPrisma.parallelBranch.create).not.toHaveBeenCalled();
      expect(result).toEqual([]);
    });

    it('passes input data through to Prisma', async () => {
      mockPrisma.parallelBranch.create.mockResolvedValue({ id: '1' });
      mockPrisma.parallelBranch.findMany.mockResolvedValue([{ id: '1' }]);

      await createParallelBranches({
        workflowId: 'wf-1',
        taskId: 't-1',
        branches: [{ branchKey: 'data-test', role: 'qa', input: { scope: 'packages/core' } }],
      });

      expect(mockPrisma.parallelBranch.create).toHaveBeenCalledWith(
        expect.objectContaining({
          data: expect.objectContaining({
            input: { scope: 'packages/core' },
          }),
        })
      );
    });
  });

  describe('listParallelBranches', () => {
    it('returns branches sorted by createdAt', async () => {
      const branches = [
        { id: '1', branchKey: 'first', createdAt: new Date('2024-01-01') },
        { id: '2', branchKey: 'second', createdAt: new Date('2024-01-02') },
      ];
      mockPrisma.parallelBranch.findMany.mockResolvedValue(branches);

      const result = await listParallelBranches('wf-1', 't-1');

      expect(mockPrisma.parallelBranch.findMany).toHaveBeenCalledWith({
        where: { workflowId: 'wf-1', taskId: 't-1' },
        orderBy: { createdAt: 'asc' },
      });
      expect(result).toEqual(branches);
    });

    it('returns empty array when no branches exist', async () => {
      mockPrisma.parallelBranch.findMany.mockResolvedValue([]);

      const result = await listParallelBranches('wf-empty', 't-empty');

      expect(result).toEqual([]);
    });
  });

  describe('completeParallelBranch', () => {
    it('sets status to completed and stores output', async () => {
      const updated = { id: '1', status: 'completed', output: { files: ['a.ts'] } };
      mockPrisma.parallelBranch.update.mockResolvedValue(updated);

      const result = await completeParallelBranch({
        branchId: '1',
        output: { files: ['a.ts'] },
      });

      expect(mockPrisma.parallelBranch.update).toHaveBeenCalledWith({
        where: { id: '1' },
        data: {
          status: 'completed',
          output: { files: ['a.ts'] },
        },
      });
      expect(result).toEqual(updated);
    });

    it('stores complex output objects', async () => {
      const output = { summary: 'done', metrics: { tests: 10, passed: 10 } };
      mockPrisma.parallelBranch.update.mockResolvedValue({ id: '1', output });

      await completeParallelBranch({ branchId: '1', output });

      expect(mockPrisma.parallelBranch.update).toHaveBeenCalledWith(
        expect.objectContaining({
          data: expect.objectContaining({ output }),
        })
      );
    });
  });

  describe('setExecutionMode', () => {
    it('sets mode to parallel', async () => {
      mockPrisma.sessionState.update.mockResolvedValue({ executionMode: 'parallel' });

      await setExecutionMode('session-1', 'parallel');

      expect(mockPrisma.sessionState.update).toHaveBeenCalledWith({
        where: { sessionKey: 'session-1' },
        data: { executionMode: 'parallel' },
      });
    });

    it('sets mode to sequential', async () => {
      mockPrisma.sessionState.update.mockResolvedValue({ executionMode: 'sequential' });

      await setExecutionMode('session-1', 'sequential');

      expect(mockPrisma.sessionState.update).toHaveBeenCalledWith({
        where: { sessionKey: 'session-1' },
        data: { executionMode: 'sequential' },
      });
    });
  });

  describe('markSynthesisReady', () => {
    it('sets synthesisReady to true', async () => {
      mockPrisma.sessionState.update.mockResolvedValue({ synthesisReady: true });

      await markSynthesisReady('session-1', true);

      expect(mockPrisma.sessionState.update).toHaveBeenCalledWith({
        where: { sessionKey: 'session-1' },
        data: { synthesisReady: true },
      });
    });

    it('sets synthesisReady to false', async () => {
      mockPrisma.sessionState.update.mockResolvedValue({ synthesisReady: false });

      await markSynthesisReady('session-1', false);

      expect(mockPrisma.sessionState.update).toHaveBeenCalledWith({
        where: { sessionKey: 'session-1' },
        data: { synthesisReady: false },
      });
    });
  });

  describe('markVerificationReady', () => {
    it('sets verificationReady to true', async () => {
      mockPrisma.sessionState.update.mockResolvedValue({ verificationReady: true });

      await markVerificationReady('session-1', true);

      expect(mockPrisma.sessionState.update).toHaveBeenCalledWith({
        where: { sessionKey: 'session-1' },
        data: { verificationReady: true },
      });
    });

    it('sets verificationReady to false', async () => {
      mockPrisma.sessionState.update.mockResolvedValue({ verificationReady: false });

      await markVerificationReady('session-1', false);

      expect(mockPrisma.sessionState.update).toHaveBeenCalledWith({
        where: { sessionKey: 'session-1' },
        data: { verificationReady: false },
      });
    });
  });
});
