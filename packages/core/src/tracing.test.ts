import { describe, it, expect, vi, beforeEach } from 'vitest';

vi.mock('@mcp-rebuild/core', async () => {
  const actual = await vi.importActual('@mcp-rebuild/core');
  return {
    ...actual,
    createLogger: () => ({
      info: vi.fn(),
      warn: vi.fn(),
      error: vi.fn(),
      debug: vi.fn(),
    }),
  };
});

import {
  getTracer,
  setTracer,
  resetTracer,
  trace,
  startSpan,
  startChildSpan,
  SpanNames,
  type Tracer,
  type Span,
  type SpanOptions,
} from './tracing.js';

describe('Tracing', () => {
  beforeEach(() => {
    resetTracer();
  });

  describe('NoopTracer (default)', () => {
    it('creates spans via startSpan', () => {
      const span = startSpan('test.operation');
      expect(span.name).toBe('test.operation');
    });

    it('supports fluent API on spans', () => {
      const span = startSpan('test');
      const result = span
        .setAttribute('key', 'value')
        .setAttributes({ a: 1, b: true })
        .addEvent('something happened')
        .setStatus('OK');

      expect(result).toBe(span);
    });

    it('records duration when ended', () => {
      const span = startSpan('timed');
      expect(span.durationMs).toBeUndefined();
      span.end();
      expect(span.durationMs).toBeGreaterThanOrEqual(0);
    });

    it('records errors on spans', () => {
      const span = startSpan('error-test');
      span.recordError(new Error('test error'));
      span.end();
      expect(span.durationMs).toBeGreaterThanOrEqual(0);
    });
  });

  describe('trace() helper', () => {
    it('wraps async functions with spans', async () => {
      const result = await trace('test.operation', async (span) => {
        span.setAttribute('input', 42);
        return 'done';
      });

      expect(result).toBe('done');
    });

    it('records errors and re-throws', async () => {
      await expect(
        trace('failing.op', async () => {
          throw new Error('boom');
        }),
      ).rejects.toThrow('boom');
    });
  });

  describe('startChildSpan', () => {
    it('creates a child span with parent reference', () => {
      const parent = startSpan('parent');
      const child = startChildSpan(parent, 'child', { operation: 'search' } as unknown as Record<string, string>);
      expect(child.name).toBe('child');
    });
  });

  describe('setTracer / resetTracer', () => {
    it('allows replacing the global tracer', async () => {
      const spans: string[] = [];
      const customTracer: Tracer = {
        startSpan(name: string, _options?: SpanOptions): Span {
          return {
            name,
            setAttribute() { return this; },
            setAttributes() { return this; },
            addEvent() { return this; },
            recordError() { return this; },
            setStatus() { return this; },
            end() { spans.push(name); },
            durationMs: 0,
          };
        },
        async trace<T>(name: string, fn: (span: Span) => Promise<T>): Promise<T> {
          const span = this.startSpan(name);
          try {
            return await fn(span);
          } finally {
            span.end();
          }
        },
      };

      setTracer(customTracer);
      await trace('custom.op', async () => 'result');
      expect(spans).toContain('custom.op');

      resetTracer();
    });
  });

  describe('SpanNames', () => {
    it('provides standard span names', () => {
      expect(SpanNames.WORKFLOW_CREATE).toBe('workflow.create');
      expect(SpanNames.MEMORY_SEARCH).toBe('memory.search');
      expect(SpanNames.LLM_COMPLETE).toBe('llm.complete');
      expect(SpanNames.SEARCH_HYBRID).toBe('search.hybrid');
    });
  });
});
