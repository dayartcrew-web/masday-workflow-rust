import { describe, it, expect } from 'vitest';
import { estimateTokens } from './token-estimator.js';

describe('estimateTokens', () => {
  it('returns 0 for empty string', () => {
    expect(estimateTokens('')).toBe(0);
  });

  it('returns positive count for non-empty string', () => {
    const count = estimateTokens('Hello, world!');
    expect(count).toBeGreaterThan(0);
  });

  it('estimates more tokens for longer strings', () => {
    const short = estimateTokens('hi');
    const long = estimateTokens('This is a much longer string with many more words and characters.');
    expect(long).toBeGreaterThan(short);
  });

  it('handles JSON input', () => {
    const json = JSON.stringify({ workflowId: 'abc-123', tasks: ['task1', 'task2'] });
    const count = estimateTokens(json);
    expect(count).toBeGreaterThan(0);
  });

  it('handles unicode text', () => {
    const count = estimateTokens('こんにちは世界 🌍');
    expect(count).toBeGreaterThan(0);
  });
});
