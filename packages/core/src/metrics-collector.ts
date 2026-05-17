import { EventBus } from './eventBus.js';
import { Event, EventType } from './types.js';
import { createLogger } from './logger.js';

const logger = createLogger('MetricsCollector');

interface EventData {
  workflowId?: string;
  taskId?: string;
  name?: string;
  skill?: string;
}

export interface MetricPoint {
  name: string;
  value: number;
  timestamp: Date;
  tags: Record<string, string>;
}

export interface MetricSummary {
  count: number;
  sum: number;
  min: number;
  max: number;
  avg: number;
}

export class MetricsCollector {
  private eventBus: EventBus;
  private metrics: Map<string, MetricPoint[]> = new Map();
  private startTimestamps: Map<string, Date> = new Map();
  private started = false;
  private handlerEntries: [EventType, (event: Event) => void][];

  constructor(eventBus: EventBus) {
    this.eventBus = eventBus;
    this.handlerEntries = [
      ['workflow.started', (e) => this.onStart(e)],
      ['task.started', (e) => this.onStart(e)],
      ['workflow.completed', (e) => this.onComplete(e, 'workflow.duration', true)],
      ['workflow.failed', (e) => this.onComplete(e, 'workflow.duration', false)],
      ['task.completed', (e) => this.onComplete(e, 'task.duration', true)],
      ['task.failed', (e) => this.onComplete(e, 'task.duration', false)],
    ];
  }

  start(): void {
    if (this.started) return;
    this.started = true;
    for (const [type, handler] of this.handlerEntries) {
      this.eventBus.on(type, handler);
    }
    logger.info('Metrics collector started');
  }

  stop(): void {
    if (!this.started) return;
    this.started = false;
    for (const [type, handler] of this.handlerEntries) {
      this.eventBus.off(type, handler);
    }
    logger.info('Metrics collector stopped');
  }

  getMetric(name: string): MetricPoint[] {
    return this.metrics.get(name) || [];
  }

  getSummary(name: string, tags?: Record<string, string>): MetricSummary {
    let points = this.getMetric(name);
    if (tags) {
      points = points.filter(p =>
        Object.entries(tags).every(([k, v]) => p.tags[k] === v)
      );
    }
    if (points.length === 0) {
      return { count: 0, sum: 0, min: 0, max: 0, avg: 0 };
    }
    const values = points.map(p => p.value);
    const sum = values.reduce((a, b) => a + b, 0);
    return {
      count: values.length,
      sum,
      min: Math.min(...values),
      max: Math.max(...values),
      avg: sum / values.length,
    };
  }

  getAllMetrics(): Map<string, MetricPoint[]> {
    return new Map(this.metrics);
  }

  private onStart(event: Event): void {
    const data = event.data as EventData;
    const id = data?.workflowId || data?.taskId;
    if (id) {
      this.startTimestamps.set(id, event.timestamp);
    }
  }

  private onComplete(event: Event, metricName: string, success: boolean): void {
    const data = event.data as EventData;
    const id = data?.workflowId || data?.taskId;
    if (!id) return;

    const startTime = this.startTimestamps.get(id);
    if (!startTime) return;

    const duration = event.timestamp.getTime() - startTime.getTime();
    this.startTimestamps.delete(id);

    const point: MetricPoint = {
      name: metricName,
      value: duration,
      timestamp: event.timestamp,
      tags: { success: String(success) },
    };

    if (!this.metrics.has(metricName)) {
      this.metrics.set(metricName, []);
    }
    this.metrics.get(metricName)!.push(point);

    this.eventBus.emit('metrics.recorded', { metric: point });
    logger.debug(`Recorded metric: ${metricName} = ${duration}ms`);
  }
}
