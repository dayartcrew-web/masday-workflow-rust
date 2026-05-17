/**
 * OpenTelemetry Tracing Integration
 *
 * Provides trace spans for workflow execution, memory operations,
 * and LLM calls. Uses a lightweight API that can be backed by
 * OpenTelemetry SDK or a no-op implementation for zero overhead.
 */

import { createLogger } from './logger.js';

const logger = createLogger('tracing');

// --- Span Status ---

export type SpanStatus = 'UNSET' | 'OK' | 'ERROR';

// --- Span Attributes ---

export type SpanAttributeValue = string | number | boolean;

export interface Span {
  /** Span name (e.g. "workflow.execute", "memory.search") */
  readonly name: string;
  /** Set an attribute on the span. */
  setAttribute(key: string, value: SpanAttributeValue): Span;
  /** Set multiple attributes. */
  setAttributes(attrs: Record<string, SpanAttributeValue>): Span;
  /** Add an event to the span. */
  addEvent(name: string, attributes?: Record<string, SpanAttributeValue>): Span;
  /** Record an error and set status to ERROR. */
  recordError(error: Error): Span;
  /** Set the span status. */
  setStatus(status: SpanStatus): Span;
  /** End the span. Records duration from start. */
  end(): void;
  /** Get span duration in ms (only available after end()). */
  readonly durationMs: number | undefined;
}

// --- Tracer Interface ---

export interface Tracer {
  /** Start a new span. */
  startSpan(name: string, options?: SpanOptions): Span;
  /** Wrap an async function with a span. */
  trace<T>(name: string, fn: (span: Span) => Promise<T>): Promise<T>;
}

export interface SpanOptions {
  /** Parent span for nesting. */
  parent?: Span;
  /** Initial attributes. */
  attributes?: Record<string, SpanAttributeValue>;
}

// --- No-op Implementation (default, zero overhead) ---

class NoopSpan implements Span {
  readonly name: string;
  private startTime: number;
  private _durationMs: number | undefined;
  private _status: SpanStatus = 'UNSET';

  constructor(name: string, private readonly options?: SpanOptions) {
    this.name = name;
    this.startTime = Date.now();
  }

  setAttribute(_key: string, _value: SpanAttributeValue): Span {
    return this;
  }

  setAttributes(_attrs: Record<string, SpanAttributeValue>): Span {
    return this;
  }

  addEvent(_name: string, _attributes?: Record<string, SpanAttributeValue>): Span {
    return this;
  }

  recordError(error: Error): Span {
    this._status = 'ERROR';
    logger.debug({ span: this.name, error: error.message }, 'Span recorded error');
    return this;
  }

  setStatus(status: SpanStatus): Span {
    this._status = status;
    return this;
  }

  end(): void {
    this._durationMs = Date.now() - this.startTime;
    logger.debug(
      { span: this.name, durationMs: this._durationMs, status: this._status },
      'Span ended',
    );
  }

  get durationMs(): number | undefined {
    return this._durationMs;
  }
}

class NoopTracer implements Tracer {
  startSpan(name: string, options?: SpanOptions): Span {
    return new NoopSpan(name, options);
  }

  async trace<T>(name: string, fn: (span: Span) => Promise<T>): Promise<T> {
    const span = this.startSpan(name);
    try {
      const result = await fn(span);
      span.setStatus('OK');
      return result;
    } catch (error: unknown) {
      if (error instanceof Error) {
        span.recordError(error);
      }
      span.setStatus('ERROR');
      throw error;
    } finally {
      span.end();
    }
  }
}

// --- Tracer Singleton ---

let globalTracer: Tracer = new NoopTracer();

/** Get the global tracer instance. */
export function getTracer(): Tracer {
  return globalTracer;
}

/** Set a custom tracer implementation. */
export function setTracer(tracer: Tracer): void {
  globalTracer = tracer;
  logger.info('Global tracer replaced');
}

/** Reset to the default no-op tracer. */
export function resetTracer(): void {
  globalTracer = new NoopTracer();
  logger.info('Global tracer reset to no-op');
}

// --- Convenience Functions ---

/** Trace an async operation with automatic span lifecycle. */
export async function trace<T>(name: string, fn: (span: Span) => Promise<T>): Promise<T> {
  return globalTracer.trace(name, fn);
}

/** Start a new span from the global tracer. */
export function startSpan(name: string, options?: SpanOptions): Span {
  return globalTracer.startSpan(name, options);
}

// --- Span Helpers ---

/** Create child span under a parent. */
export function startChildSpan(parent: Span, name: string, attributes?: Record<string, SpanAttributeValue>): Span {
  return globalTracer.startSpan(name, { parent, attributes });
}

/** Standard span names for consistency across packages. */
export const SpanNames = {
  // Workflow
  WORKFLOW_CREATE: 'workflow.create',
  WORKFLOW_EXECUTE: 'workflow.execute',
  WORKFLOW_PLAN: 'workflow.plan',
  WORKFLOW_VERIFY: 'workflow.verify',
  WORKFLOW_FIX: 'workflow.fix',
  TASK_EXECUTE: 'task.execute',

  // Memory
  MEMORY_ADD: 'memory.add',
  MEMORY_SEARCH: 'memory.search',
  MEMORY_GET: 'memory.get',
  MEMORY_PRUNE: 'memory.prune',
  MEMORY_REFLECT: 'memory.reflect',
  MEMORY_EMBED: 'memory.embed',
  GRAPH_QUERY: 'graph.query',
  GRAPH_TRAVERSE: 'graph.traverse',

  // LLM
  LLM_COMPLETE: 'llm.complete',
  LLM_CHAT: 'llm.chat',
  LLM_EMBED: 'llm.embed',

  // Policy
  POLICY_VALIDATE: 'policy.validate',
  POLICY_AUDIT: 'policy.audit',

  // Intelligence
  SEARCH_HYBRID: 'search.hybrid',
  SEARCH_BM25: 'search.bm25',
  SEARCH_VECTOR: 'search.vector',
} as const;
