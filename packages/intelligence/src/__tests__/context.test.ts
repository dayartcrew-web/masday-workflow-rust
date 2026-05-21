import { describe, it, expect } from 'vitest';
import {
  computeFingerprint,
  buildContextPack,
  buildHybridContextPack,
} from '../context.js';
import type { MemoryProvider, DocumentProvider, EmbeddingProvider, ChunkProvider } from '../context.js';
import type { ContextPackInput, ContextPackDoc, IndexedChunk } from '../types.js';

function baseInput(): Parameters<typeof computeFingerprint>[0] {
  return {
    workflowId: 'wf-1',
    planId: 'plan-1',
    taskId: 'task-1',
    acceptanceCriteria: ['ac1', 'ac2'],
    requiredContext: ['rc1', 'rc2'],
    memoryIds: ['mem-1', 'mem-2'],
    docIds: ['doc-1', 'doc-2'],
  };
}

function baseContextPackInput(overrides?: Partial<ContextPackInput>): ContextPackInput {
  return {
    workflowId: 'wf-1',
    planId: 'plan-1',
    taskId: 'task-1',
    planSummary: 'test plan',
    taskTitle: 'test task',
    acceptanceCriteria: ['ac1', 'ac2'],
    requiredContext: ['rc1'],
    recentProgress: ['progress 1'],
    ...overrides,
  };
}

describe('computeFingerprint', () => {
  it('returns deterministic hash for same input', () => {
    const input = baseInput();
    const a = computeFingerprint(input);
    const b = computeFingerprint(input);
    expect(a).toBe(b);
  });

  it('different inputs produce different hashes', () => {
    const a = computeFingerprint(baseInput());
    const b = computeFingerprint({ ...baseInput(), workflowId: 'wf-2' });
    expect(a).not.toBe(b);
  });

  it('returns 64-char hex string', () => {
    const hash = computeFingerprint(baseInput());
    expect(hash).toHaveLength(64);
    expect(hash).toMatch(/^[0-9a-f]{64}$/);
  });

  it('memoryIds sorted order does not affect hash', () => {
    const a = computeFingerprint({ ...baseInput(), memoryIds: ['mem-1', 'mem-2', 'mem-3'] });
    const b = computeFingerprint({ ...baseInput(), memoryIds: ['mem-3', 'mem-1', 'mem-2'] });
    expect(a).toBe(b);
  });

  it('docIds sorted order does not affect hash', () => {
    const a = computeFingerprint({ ...baseInput(), docIds: ['doc-1', 'doc-2', 'doc-3'] });
    const b = computeFingerprint({ ...baseInput(), docIds: ['doc-3', 'doc-1', 'doc-2'] });
    expect(a).toBe(b);
  });

  it('empty arrays produce valid hash', () => {
    const hash = computeFingerprint({
      ...baseInput(),
      acceptanceCriteria: [],
      requiredContext: [],
      memoryIds: [],
      docIds: [],
    });
    expect(hash).toHaveLength(64);
    expect(hash).toMatch(/^[0-9a-f]{64}$/);
  });

  it('special characters in strings do not break', () => {
    const hash = computeFingerprint({
      ...baseInput(),
      workflowId: 'wf-\\"<>&\n\t',
      planId: 'plan-👋',
      taskId: 'task-unicode-✓',
      acceptanceCriteria: ['ac<>&"'],
    });
    expect(hash).toHaveLength(64);
    expect(hash).toMatch(/^[0-9a-f]{64}$/);
  });

  it('different acceptanceCriteria produce different hashes', () => {
    const a = computeFingerprint({ ...baseInput(), acceptanceCriteria: ['ac1'] });
    const b = computeFingerprint({ ...baseInput(), acceptanceCriteria: ['ac2'] });
    expect(a).not.toBe(b);
  });

  it('different requiredContext produce different hashes', () => {
    const a = computeFingerprint({ ...baseInput(), requiredContext: ['rc1'] });
    const b = computeFingerprint({ ...baseInput(), requiredContext: ['rc2'] });
    expect(a).not.toBe(b);
  });

  it('acceptanceCriteria order affects hash (not sorted)', () => {
    const a = computeFingerprint({ ...baseInput(), acceptanceCriteria: ['a', 'b', 'c'] });
    const b = computeFingerprint({ ...baseInput(), acceptanceCriteria: ['c', 'b', 'a'] });
    expect(a).not.toBe(b);
  });
});

function noopMemoryProvider(): MemoryProvider {
  return {
    async search(_query, _options) {
      return [];
    },
  };
}

function noopDocumentProvider(): DocumentProvider {
  return {
    async search(_query, _options) {
      return [];
    },
    async getAll(_workflowId, _options) {
      return [];
    },
  };
}

describe('buildContextPack', () => {
  it('returns valid result with fingerprint and contextSufficient', async () => {
    const result = await buildContextPack(baseContextPackInput(), {
      memory: noopMemoryProvider(),
      documents: noopDocumentProvider(),
    });
    expect(result).toHaveProperty('fingerprint');
    expect(result).toHaveProperty('contextSufficient');
    expect(result.fingerprint).toHaveLength(64);
  });

  it('contextSufficient is false when no memories or docs', async () => {
    const result = await buildContextPack(baseContextPackInput(), {
      memory: noopMemoryProvider(),
      documents: noopDocumentProvider(),
    });
    expect(result.contextSufficient).toBe(false);
  });

  it('contextSufficient is true when memories found', async () => {
    const providers = {
      memory: {
        async search(_query: string, _options?: { limit?: number; threshold?: number }) {
          return [
            {
              memory: { id: 'mem-1', content: 'test memory content', type: 'episodic', importance: 5 },
              score: 0.9,
            },
          ];
        },
      } satisfies MemoryProvider,
      documents: noopDocumentProvider(),
    };
    const result = await buildContextPack(baseContextPackInput(), providers);
    expect(result.contextSufficient).toBe(true);
    expect(result.semanticMemories).toHaveLength(1);
  });

  it('contextSufficient is true when docs found', async () => {
    const providers = {
      memory: noopMemoryProvider(),
      documents: {
        async search(_query: string, _options?: { limit?: number }) {
          return [{ id: 'doc-1', title: 'Test Doc', content: 'doc content' }];
        },
        async getAll(_workflowId: string, _options?: { limit?: number }) {
          return [];
        },
      } satisfies DocumentProvider,
    };
    const result = await buildContextPack(baseContextPackInput(), providers);
    expect(result.contextSufficient).toBe(true);
    expect(result.semanticDocs).toHaveLength(1);
  });

  it('returns input metadata in result', async () => {
    const input = baseContextPackInput({ taskTitle: 'specific task' });
    const result = await buildContextPack(input, {
      memory: noopMemoryProvider(),
      documents: noopDocumentProvider(),
    });
    expect(result.workflowId).toBe('wf-1');
    expect(result.planId).toBe('plan-1');
    expect(result.taskId).toBe('task-1');
    expect(result.taskTitle).toBe('specific task');
    expect(result.acceptanceCriteria).toEqual(['ac1', 'ac2']);
    expect(result.recentProgress).toEqual(['progress 1']);
  });

  it('handles provider errors gracefully and still returns result', async () => {
    const providers = {
      memory: {
        async search(_query: string, _options?: { limit?: number; threshold?: number }) {
          throw new Error('memory search failed');
        },
      } satisfies MemoryProvider,
      documents: noopDocumentProvider(),
    };
    const result = await buildContextPack(baseContextPackInput(), providers);
    expect(result).toHaveProperty('fingerprint');
    expect(result.contextSufficient).toBe(false);
  });

  it('handles without any providers', async () => {
    const result = await buildContextPack(baseContextPackInput(), {});
    expect(result).toHaveProperty('fingerprint');
    expect(result.semanticMemories).toEqual([]);
    expect(result.semanticDocs).toEqual([]);
  });
});

describe('buildHybridContextPack', () => {
  it('returns valid result with fingerprint and contextSufficient', async () => {
    const result = await buildHybridContextPack(baseContextPackInput(), {
      memory: noopMemoryProvider(),
      documents: noopDocumentProvider(),
    });
    expect(result).toHaveProperty('fingerprint');
    expect(result).toHaveProperty('contextSufficient');
    expect(result).toHaveProperty('executionMode');
    expect(result.fingerprint).toHaveLength(64);
  });

  it('contextSufficient is false when no memories or docs', async () => {
    const result = await buildHybridContextPack(baseContextPackInput(), {
      memory: noopMemoryProvider(),
      documents: noopDocumentProvider(),
    });
    expect(result.contextSufficient).toBe(false);
  });

  it('contextSufficient is true when either memories or docs found', async () => {
    const providers = {
      memory: {
        async search(_query: string, _options?: { limit?: number; threshold?: number }) {
          return [
            {
              memory: { id: 'mem-1', content: 'test memory content', type: 'episodic', importance: 5 },
              score: 0.9,
            },
          ];
        },
      } satisfies MemoryProvider,
      documents: noopDocumentProvider(),
    };
    const result = await buildHybridContextPack(baseContextPackInput(), providers);
    expect(result.contextSufficient).toBe(true);
  });

  it('sets executionMode to parallel when acceptanceCriteria > 3', async () => {
    const input = baseContextPackInput({ acceptanceCriteria: ['a', 'b', 'c', 'd'] });
    const result = await buildHybridContextPack(input, {
      memory: noopMemoryProvider(),
      documents: noopDocumentProvider(),
    });
    expect(result.executionMode).toBe('parallel');
  });

  it('sets executionMode to sequential when acceptanceCriteria <= 3', async () => {
    const input = baseContextPackInput({ acceptanceCriteria: ['a', 'b', 'c'] });
    const result = await buildHybridContextPack(input, {
      memory: noopMemoryProvider(),
      documents: noopDocumentProvider(),
    });
    expect(result.executionMode).toBe('sequential');
  });

  it('handles without any providers', async () => {
    const result = await buildHybridContextPack(baseContextPackInput(), {});
    expect(result).toHaveProperty('fingerprint');
    expect(result.executionMode).toBe('sequential');
  });
});
