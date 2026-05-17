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
  getMetrics,
  setMetricsRegistry,
  resetMetricsRegistry,
  incrementCounter,
  recordGauge,
  recordHistogram,
  startTimer,
  MetricNames,
  type MetricsRegistry,
} from './metrics.js';

describe('Metrics', () => {
  beforeEach(() => {
    resetMetricsRegistry();
  });

  describe('SimpleMetricsRegistry', () => {
    it('increments counters', () => {
      const metrics = getMetrics();
      metrics.increment('test.counter');
      metrics.increment('test.counter', 5);

      const result = metrics.getCounter('test.counter');
      expect(result.count).toBe(2);
      expect(result.sum).toBe(6);
    });

    it('filters counters by tags', () => {
      const metrics = getMetrics();
      metrics.increment('test.counter', 1, { provider: 'anthropic' });
      metrics.increment('test.counter', 1, { provider: 'openai' });
      metrics.increment('test.counter', 1, { provider: 'anthropic' });

      const anthropicResult = metrics.getCounter('test.counter', { provider: 'anthropic' });
      expect(anthropicResult.count).toBe(2);
      expect(anthropicResult.sum).toBe(2);

      const openaiResult = metrics.getCounter('test.counter', { provider: 'openai' });
      expect(openaiResult.count).toBe(1);
    });

    it('records gauges', () => {
      const metrics = getMetrics();
      metrics.gauge('memory.size', 100);
      metrics.gauge('memory.size', 200);

      const points = metrics.getPoints('memory.size');
      expect(points.length).toBe(2);
      expect(points[0].type).toBe('gauge');
      expect(points[0].value).toBe(100);
    });

    it('records histograms and computes summaries', () => {
      const metrics = getMetrics();
      const values = [10, 20, 30, 40, 50, 60, 70, 80, 90, 100];
      for (const v of values) {
        metrics.histogram('latency', v);
      }

      const summary = metrics.getHistogramSummary('latency');
      expect(summary.count).toBe(10);
      expect(summary.min).toBe(10);
      expect(summary.max).toBe(100);
      expect(summary.avg).toBe(55);
      // p50 = values[floor(10*0.5)] = values[5] = 60 (0-indexed)
      expect(summary.p50).toBe(60);
      expect(summary.p95).toBe(100);
      expect(summary.p99).toBe(100);
    });

    it('returns empty summary for non-existent histogram', () => {
      const metrics = getMetrics();
      const summary = metrics.getHistogramSummary('nonexistent');
      expect(summary.count).toBe(0);
      expect(summary.avg).toBe(0);
    });

    it('returns all metric points', () => {
      const metrics = getMetrics();
      metrics.increment('counter.a');
      metrics.gauge('gauge.b', 42);
      metrics.histogram('histogram.c', 100);

      const allPoints = metrics.getPoints();
      expect(allPoints.length).toBe(3);

      const counterPoints = allPoints.filter(p => p.type === 'counter');
      expect(counterPoints.length).toBe(1);
    });

    it('resets all metrics', () => {
      const metrics = getMetrics();
      metrics.increment('test', 10);
      metrics.gauge('gauge', 42);
      metrics.histogram('hist', 100);

      metrics.reset();

      expect(metrics.getCounter('test').count).toBe(0);
      expect(metrics.getPoints().length).toBe(0);
    });
  });

  describe('Convenience functions', () => {
    it('incrementCounter uses global registry', () => {
      incrementCounter('global.test', 3);
      const result = getMetrics().getCounter('global.test');
      expect(result.sum).toBe(3);
    });

    it('recordGauge uses global registry', () => {
      recordGauge('global.gauge', 99);
      const points = getMetrics().getPoints('global.gauge');
      expect(points.length).toBe(1);
    });

    it('recordHistogram uses global registry', () => {
      recordHistogram('global.hist', 50);
      const summary = getMetrics().getHistogramSummary('global.hist');
      expect(summary.count).toBe(1);
      expect(summary.avg).toBe(50);
    });
  });

  describe('startTimer', () => {
    it('records duration on stop', () => {
      const timer = startTimer();
      const durationMs = timer.stop('test.duration');
      expect(durationMs).toBeGreaterThanOrEqual(0);

      const summary = getMetrics().getHistogramSummary('test.duration');
      expect(summary.count).toBe(1);
    });

    it('passes tags to histogram', () => {
      const timer = startTimer();
      timer.stop('tagged.duration', { operation: 'search' });

      const summary = getMetrics().getHistogramSummary('tagged.duration', { operation: 'search' });
      expect(summary.count).toBe(1);
    });
  });

  describe('MetricNames', () => {
    it('provides standard metric names', () => {
      expect(MetricNames.WORKFLOWS_CREATED).toBe('workflows.created');
      expect(MetricNames.LLM_TOKENS_USED).toBe('llm.tokens_used');
      expect(MetricNames.MEMORY_OPERATIONS).toBe('memory.operations');
      expect(MetricNames.SEARCH_DURATION_MS).toBe('search.duration_ms');
    });
  });

  describe('setMetricsRegistry', () => {
    it('allows replacing the global registry', () => {
      let customCount = 0;
      const customRegistry: MetricsRegistry = {
        increment: () => { customCount++; },
        gauge: () => {},
        histogram: () => {},
        getPoints: () => [],
        getCounter: () => ({ count: 0, sum: 0 }),
        getHistogramSummary: () => ({ count: 0, sum: 0, min: 0, max: 0, avg: 0, p50: 0, p95: 0, p99: 0 }),
        reset: () => {},
      };

      setMetricsRegistry(customRegistry);
      incrementCounter('custom.test');
      expect(customCount).toBe(1);

      resetMetricsRegistry();
    });
  });
});
