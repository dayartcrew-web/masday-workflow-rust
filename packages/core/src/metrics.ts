/**
 * OpenTelemetry Metrics Integration
 *
 * Provides counters and histograms for:
 * - Workflows: created, completed, failed, duration
 * - Tasks: started, completed, failed, duration
 * - Memory: operations (add, search, get), hit rate
 * - LLM: tokens used, latency, provider calls
 *
 * Uses a lightweight API that can be backed by OpenTelemetry SDK
 * or the built-in simple aggregator for zero-dependency usage.
 */

import { createLogger } from './logger.js';

const logger = createLogger('metrics');

// --- Metric Types ---

export type MetricType = 'counter' | 'gauge' | 'histogram';

export interface TelemetryMetricPoint {
  readonly name: string;
  readonly type: MetricType;
  readonly value: number;
  readonly timestamp: number;
  readonly tags: Readonly<Record<string, string>>;
}

// --- Metric Names ---

export const MetricNames = {
  // Workflow
  WORKFLOWS_CREATED: 'workflows.created',
  WORKFLOWS_COMPLETED: 'workflows.completed',
  WORKFLOWS_FAILED: 'workflows.failed',
  WORKFLOW_DURATION_MS: 'workflow.duration_ms',

  // Tasks
  TASKS_STARTED: 'tasks.started',
  TASKS_COMPLETED: 'tasks.completed',
  TASKS_FAILED: 'tasks.failed',
  TASK_DURATION_MS: 'task.duration_ms',

  // Memory
  MEMORY_OPERATIONS: 'memory.operations',
  MEMORY_STORE_SIZE: 'memory.store_size',
  MEMORY_SEARCH_RESULTS: 'memory.search_results',
  MEMORY_HIT_RATE: 'memory.hit_rate',

  // LLM
  LLM_TOKENS_USED: 'llm.tokens_used',
  LLM_LATENCY_MS: 'llm.latency_ms',
  LLM_PROVIDER_CALLS: 'llm.provider_calls',
  LLM_CIRCUIT_STATE: 'llm.circuit_state',

  // Search
  SEARCH_OPERATIONS: 'search.operations',
  SEARCH_DURATION_MS: 'search.duration_ms',
  SEARCH_RESULT_COUNT: 'search.result_count',
} as const;

// --- Metrics Registry ---

export interface MetricsRegistry {
  /** Increment a counter by the given value (default 1). */
  increment(name: string, value?: number, tags?: Record<string, string>): void;
  /** Record a gauge value. */
  gauge(name: string, value: number, tags?: Record<string, string>): void;
  /** Record a histogram value (e.g., duration). */
  histogram(name: string, value: number, tags?: Record<string, string>): void;
  /** Get all recorded metric points. */
  getPoints(name?: string): ReadonlyArray<TelemetryMetricPoint>;
  /** Get a summary of a counter. */
  getCounter(name: string, tags?: Record<string, string>): { count: number; sum: number };
  /** Get a summary of a histogram. */
  getHistogramSummary(name: string, tags?: Record<string, string>): {
    count: number;
    sum: number;
    min: number;
    max: number;
    avg: number;
    p50: number;
    p95: number;
    p99: number;
  };
  /** Reset all metrics. */
  reset(): void;
}

// --- Simple In-Memory Implementation ---

class SimpleMetricsRegistry implements MetricsRegistry {
  private counters: Map<string, Array<{ value: number; timestamp: number; tags: Record<string, string> }>> = new Map();
  private gauges: Map<string, Array<{ value: number; timestamp: number; tags: Record<string, string> }>> = new Map();
  private histograms: Map<string, Array<{ value: number; timestamp: number; tags: Record<string, string> }>> = new Map();

  increment(name: string, value: number = 1, tags: Record<string, string> = {}): void {
    if (!this.counters.has(name)) {
      this.counters.set(name, []);
    }
    this.counters.get(name)!.push({
      value,
      timestamp: Date.now(),
      tags: { ...tags },
    });
  }

  gauge(name: string, value: number, tags: Record<string, string> = {}): void {
    if (!this.gauges.has(name)) {
      this.gauges.set(name, []);
    }
    this.gauges.get(name)!.push({
      value,
      timestamp: Date.now(),
      tags: { ...tags },
    });
  }

  histogram(name: string, value: number, tags: Record<string, string> = {}): void {
    if (!this.histograms.has(name)) {
      this.histograms.set(name, []);
    }
    this.histograms.get(name)!.push({
      value,
      timestamp: Date.now(),
      tags: { ...tags },
    });
  }

  getPoints(name?: string): ReadonlyArray<TelemetryMetricPoint> {
    const points: TelemetryMetricPoint[] = [];

    const collect = (map: Map<string, Array<{ value: number; timestamp: number; tags: Record<string, string> }>>, type: MetricType) => {
      for (const [metricName, entries] of map) {
        if (name && metricName !== name) continue;
        for (const entry of entries) {
          points.push({
            name: metricName,
            type,
            value: entry.value,
            timestamp: entry.timestamp,
            tags: entry.tags,
          });
        }
      }
    };

    collect(this.counters, 'counter');
    collect(this.gauges, 'gauge');
    collect(this.histograms, 'histogram');

    return points;
  }

  getCounter(name: string, tags?: Record<string, string>): { count: number; sum: number } {
    const entries = this.counters.get(name) ?? [];
    const filtered = tags
      ? entries.filter(e => Object.entries(tags).every(([k, v]) => e.tags[k] === v))
      : entries;

    return {
      count: filtered.length,
      sum: filtered.reduce((acc, e) => acc + e.value, 0),
    };
  }

  getHistogramSummary(name: string, tags?: Record<string, string>): {
    count: number;
    sum: number;
    min: number;
    max: number;
    avg: number;
    p50: number;
    p95: number;
    p99: number;
  } {
    const entries = this.histograms.get(name) ?? [];
    const filtered = tags
      ? entries.filter(e => Object.entries(tags).every(([k, v]) => e.tags[k] === v))
      : entries;

    if (filtered.length === 0) {
      return { count: 0, sum: 0, min: 0, max: 0, avg: 0, p50: 0, p95: 0, p99: 0 };
    }

    const values = filtered.map(e => e.value).sort((a, b) => a - b);
    const sum = values.reduce((acc, v) => acc + v, 0);

    return {
      count: values.length,
      sum,
      min: values[0],
      max: values[values.length - 1],
      avg: sum / values.length,
      p50: values[Math.floor(values.length * 0.5)],
      p95: values[Math.floor(values.length * 0.95)],
      p99: values[Math.min(Math.floor(values.length * 0.99), values.length - 1)],
    };
  }

  reset(): void {
    this.counters.clear();
    this.gauges.clear();
    this.histograms.clear();
    logger.info('Metrics registry reset');
  }
}

// --- Global Registry Singleton ---

let globalRegistry: MetricsRegistry = new SimpleMetricsRegistry();

/** Get the global metrics registry. */
export function getMetrics(): MetricsRegistry {
  return globalRegistry;
}

/** Set a custom metrics registry. */
export function setMetricsRegistry(registry: MetricsRegistry): void {
  globalRegistry = registry;
  logger.info('Global metrics registry replaced');
}

/** Reset to the default simple registry. */
export function resetMetricsRegistry(): void {
  globalRegistry = new SimpleMetricsRegistry();
  logger.info('Global metrics registry reset to simple');
}

// --- Convenience Functions ---

/** Increment a counter. */
export function incrementCounter(name: string, value?: number, tags?: Record<string, string>): void {
  globalRegistry.increment(name, value, tags);
}

/** Record a gauge value. */
export function recordGauge(name: string, value: number, tags?: Record<string, string>): void {
  globalRegistry.gauge(name, value, tags);
}

/** Record a histogram value (e.g., duration). */
export function recordHistogram(name: string, value: number, tags?: Record<string, string>): void {
  globalRegistry.histogram(name, value, tags);
}

// --- Timer Utility ---

/**
 * Create a timer that records duration in a histogram when stopped.
 * Usage: const timer = startTimer(); ... timer.stop('workflow.duration_ms', { id: wfId });
 */
export function startTimer(): { stop: (metricName: string, tags?: Record<string, string>) => number } {
  const start = Date.now();
  return {
    stop(metricName: string, tags?: Record<string, string>): number {
      const durationMs = Date.now() - start;
      globalRegistry.histogram(metricName, durationMs, tags);
      return durationMs;
    },
  };
}
