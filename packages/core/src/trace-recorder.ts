import { EventBus } from './eventBus.js';
import { Event, EventType } from './types.js';
import { createLogger } from './logger.js';

const logger = createLogger('TraceRecorder');

interface EventData {
  workflowId?: string;
  taskId?: string;
  name?: string;
  skill?: string;
}

export interface TraceSpan {
  traceId: string;
  spanId: string;
  parentSpanId?: string;
  operation: string;
  startTime: Date;
  endTime?: Date;
  status: 'ok' | 'error';
  attributes: Record<string, unknown>;
}

export class TraceRecorder {
  private eventBus: EventBus;
  private traces: Map<string, TraceSpan[]> = new Map();
  private started = false;
  private handlerEntries: [EventType, (event: Event) => void][];

  constructor(eventBus: EventBus) {
    this.eventBus = eventBus;
    this.handlerEntries = [
      ['workflow.started', (e) => this.onWorkflowStart(e)],
      ['workflow.completed', (e) => this.onWorkflowEnd(e, 'ok')],
      ['workflow.failed', (e) => this.onWorkflowEnd(e, 'error')],
      ['task.started', (e) => this.onTaskStart(e)],
      ['task.completed', (e) => this.onTaskEnd(e, 'ok')],
      ['task.failed', (e) => this.onTaskEnd(e, 'error')],
    ];
  }

  start(): void {
    if (this.started) return;
    this.started = true;
    for (const [type, handler] of this.handlerEntries) {
      this.eventBus.on(type, handler);
    }
    logger.info('Trace recorder started');
  }

  stop(): void {
    if (!this.started) return;
    this.started = false;
    for (const [type, handler] of this.handlerEntries) {
      this.eventBus.off(type, handler);
    }
    logger.info('Trace recorder stopped');
  }

  getTrace(traceId: string): TraceSpan[] {
    return this.traces.get(traceId) || [];
  }

  getAllTraces(): Map<string, TraceSpan[]> {
    return new Map(this.traces);
  }

  private onWorkflowStart(event: Event): void {
    const data = event.data as EventData;
    const workflowId = data?.workflowId;
    if (!workflowId) return;

    const span: TraceSpan = {
      traceId: workflowId,
      spanId: workflowId,
      operation: 'workflow',
      startTime: event.timestamp,
      status: 'ok',
      attributes: { name: data?.name || '' },
    };

    this.traces.set(workflowId, [span]);
    this.eventBus.emit('trace.started', { traceId: workflowId, spanId: workflowId });
  }

  private onWorkflowEnd(event: Event, status: 'ok' | 'error'): void {
    const data = event.data as EventData;
    const workflowId = data?.workflowId;
    if (!workflowId) return;

    const spans = this.traces.get(workflowId);
    if (spans && spans.length > 0) {
      spans[0].endTime = event.timestamp;
      spans[0].status = status;
    }
    this.eventBus.emit('trace.completed', { traceId: workflowId, status });
  }

  private onTaskStart(event: Event): void {
    const data = event.data as EventData;
    const taskId = data?.taskId;
    // Find the active workflow trace to attach the task span
    const workflowId = data?.workflowId;
    if (!taskId) return;

    // Use workflowId if available, otherwise create standalone trace
    const traceId = workflowId || taskId;
    const span: TraceSpan = {
      traceId,
      spanId: taskId,
      parentSpanId: workflowId ? workflowId : undefined,
      operation: 'task',
      startTime: event.timestamp,
      status: 'ok',
      attributes: { skill: data?.skill || '' },
    };

    if (!this.traces.has(traceId)) {
      this.traces.set(traceId, []);
    }
    this.traces.get(traceId)!.push(span);
    this.eventBus.emit('trace.started', { traceId, spanId: taskId });
  }

  private onTaskEnd(event: Event, status: 'ok' | 'error'): void {
    const data = event.data as EventData;
    const taskId = data?.taskId;
    if (!taskId) return;

    for (const spans of this.traces.values()) {
      const span = spans.find(s => s.spanId === taskId && !s.endTime);
      if (span) {
        span.endTime = event.timestamp;
        span.status = status;
        break;
      }
    }
  }
}
