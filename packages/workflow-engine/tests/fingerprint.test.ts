import { describe, it, expect } from 'vitest';
import { makeFingerprint } from '../src/fingerprint.js';
import type { FingerprintInput } from '../src/fingerprint.js';

function baseInput(overrides?: Partial<FingerprintInput>): FingerprintInput {
  return {
    workflowId: 'wf-123',
    planId: 'plan-456',
    taskId: 'task-789',
    acceptanceCriteria: ['Must handle input', 'Must return JSON'],
    requiredContext: ['API spec', 'Database schema'],
    documentIds: ['doc-1', 'doc-2'],
    memoryIds: ['mem-1', 'mem-2'],
    ...overrides,
  };
}

describe('makeFingerprint', () => {
  it('returns deterministic hash for same input', () => {
    const input = baseInput();
    const a = makeFingerprint(input);
    const b = makeFingerprint(input);
    expect(a).toBe(b);
  });

  it('different workflowId produces different hash', () => {
    const a = makeFingerprint(baseInput({ workflowId: 'wf-a' }));
    const b = makeFingerprint(baseInput({ workflowId: 'wf-b' }));
    expect(a).not.toBe(b);
  });

  it('different planId produces different hash', () => {
    const a = makeFingerprint(baseInput({ planId: 'plan-a' }));
    const b = makeFingerprint(baseInput({ planId: 'plan-b' }));
    expect(a).not.toBe(b);
  });

  it('different taskId produces different hash', () => {
    const a = makeFingerprint(baseInput({ taskId: 'task-a' }));
    const b = makeFingerprint(baseInput({ taskId: 'task-b' }));
    expect(a).not.toBe(b);
  });

  it('acceptanceCriteria order does not affect hash (sorted internally)', () => {
    const a = makeFingerprint(
      baseInput({ acceptanceCriteria: ['alpha', 'beta', 'gamma'] }),
    );
    const b = makeFingerprint(
      baseInput({ acceptanceCriteria: ['gamma', 'alpha', 'beta'] }),
    );
    expect(a).toBe(b);
  });

  it('requiredContext order does not affect hash (sorted internally)', () => {
    const a = makeFingerprint(
      baseInput({ requiredContext: ['x', 'y', 'z'] }),
    );
    const b = makeFingerprint(
      baseInput({ requiredContext: ['z', 'x', 'y'] }),
    );
    expect(a).toBe(b);
  });

  it('empty arrays produce valid hash', () => {
    const hash = makeFingerprint(
      baseInput({
        acceptanceCriteria: [],
        requiredContext: [],
        documentIds: [],
        memoryIds: [],
      }),
    );
    expect(hash).toHaveLength(64);
    expect(hash).toMatch(/^[0-9a-f]{64}$/);
  });

  it('returns 64-char hex string', () => {
    const hash = makeFingerprint(baseInput());
    expect(hash).toHaveLength(64);
    expect(hash).toMatch(/^[0-9a-f]{64}$/);
  });

  it('different acceptanceCriteria content produces different hash', () => {
    const a = makeFingerprint(baseInput({ acceptanceCriteria: ['Must log errors'] }));
    const b = makeFingerprint(baseInput({ acceptanceCriteria: ['Must send email'] }));
    expect(a).not.toBe(b);
  });

  it('documentIds order does not affect hash', () => {
    const a = makeFingerprint(baseInput({ documentIds: ['d1', 'd2', 'd3'] }));
    const b = makeFingerprint(baseInput({ documentIds: ['d3', 'd1', 'd2'] }));
    expect(a).toBe(b);
  });

  it('memoryIds order does not affect hash', () => {
    const a = makeFingerprint(baseInput({ memoryIds: ['m1', 'm2', 'm3'] }));
    const b = makeFingerprint(baseInput({ memoryIds: ['m3', 'm1', 'm2'] }));
    expect(a).toBe(b);
  });

  it('all fields changed produces different hash', () => {
    const a = makeFingerprint(baseInput());
    const b = makeFingerprint({
      workflowId: 'wf-diff',
      planId: 'plan-diff',
      taskId: 'task-diff',
      acceptanceCriteria: ['different criteria'],
      requiredContext: ['different context'],
      documentIds: ['doc-diff'],
      memoryIds: ['mem-diff'],
    });
    expect(a).not.toBe(b);
  });

  it('handles very long strings without error', () => {
    const longString = 'x'.repeat(10000);
    const hash = makeFingerprint(
      baseInput({
        workflowId: longString,
        acceptanceCriteria: [longString],
      }),
    );
    expect(hash).toHaveLength(64);
    expect(hash).toMatch(/^[0-9a-f]{64}$/);
  });

  it('handles unicode and special characters', () => {
    const hash = makeFingerprint(
      baseInput({
        workflowId: 'wf-🔥-日本語',
        planId: 'plan-<script>alert(1)</script>',
        taskId: "task-with'quotes\"and\\backslash",
        acceptanceCriteria: ['ac\nwith\nnewlines', 'ac\twith\ttabs'],
      }),
    );
    expect(hash).toHaveLength(64);
    expect(hash).toMatch(/^[0-9a-f]{64}$/);
  });

  it('null-like string values are treated as distinct', () => {
    const a = makeFingerprint(baseInput({ workflowId: 'null' }));
    const b = makeFingerprint(baseInput({ workflowId: 'undefined' }));
    const c = makeFingerprint(baseInput({ workflowId: '' }));
    expect(a).not.toBe(b);
    expect(a).not.toBe(c);
    expect(b).not.toBe(c);
  });

  it('duplicate items in arrays are preserved (not deduplicated)', () => {
    const a = makeFingerprint(baseInput({ acceptanceCriteria: ['ac', 'ac', 'ac'] }));
    const b = makeFingerprint(baseInput({ acceptanceCriteria: ['ac'] }));
    // After sorting: ['ac','ac','ac'] vs ['ac'] — different payloads
    expect(a).not.toBe(b);
  });

  it('fingerprint is stable across multiple calls with same object reference', () => {
    const input = baseInput();
    const hashes = Array.from({ length: 100 }, () => makeFingerprint(input));
    const unique = new Set(hashes);
    expect(unique.size).toBe(1);
  });

  it('empty string fields produce same hash as other empty strings', () => {
    const a = makeFingerprint(baseInput({ workflowId: '' }));
    const b = makeFingerprint(baseInput({ workflowId: '' }));
    expect(a).toBe(b);
  });

  it('single-item arrays sort to same result', () => {
    const a = makeFingerprint(baseInput({ acceptanceCriteria: ['only'] }));
    const b = makeFingerprint(baseInput({ acceptanceCriteria: ['only'] }));
    expect(a).toBe(b);
  });

  it('fingerprint changes when any single array element changes', () => {
    const base = baseInput({ acceptanceCriteria: ['a', 'b', 'c'] });
    const original = makeFingerprint(base);

    const changed1 = makeFingerprint({ ...base, acceptanceCriteria: ['a', 'B', 'c'] });
    const changed2 = makeFingerprint({ ...base, acceptanceCriteria: ['a', 'b', 'c', 'd'] });
    const changed3 = makeFingerprint({ ...base, acceptanceCriteria: ['a', 'b'] });

    expect(changed1).not.toBe(original);
    expect(changed2).not.toBe(original);
    expect(changed3).not.toBe(original);
  });
});
