import { EventBus } from './eventBus.js';
import { createLogger } from './logger.js';

const logger = createLogger('HealthChecker');

export interface HealthCheck {
  name: string;
  status: 'pass' | 'fail' | 'warn';
  message: string;
  duration: number;
}

export interface HealthStatus {
  status: 'healthy' | 'degraded' | 'unhealthy';
  checks: HealthCheck[];
  timestamp: Date;
}

type HealthCheckFn = () => Promise<HealthCheck>;

export class HealthChecker {
  private eventBus: EventBus;
  private checks: Map<string, HealthCheckFn> = new Map();
  private intervalId: ReturnType<typeof setInterval> | null = null;
  private checkInterval: number;

  constructor(eventBus: EventBus, options?: { checkInterval?: number }) {
    this.eventBus = eventBus;
    this.checkInterval = options?.checkInterval ?? 30000;
  }

  registerCheck(name: string, check: HealthCheckFn): void {
    this.checks.set(name, check);
    logger.info(`Registered health check: ${name}`);
  }

  removeCheck(name: string): void {
    this.checks.delete(name);
    logger.info(`Removed health check: ${name}`);
  }

  start(): void {
    if (this.intervalId) return;

    // Register built-in checks
    this.registerCheck('eventbus.history', async () => ({
      name: 'eventbus.history',
      status: 'pass',
      message: `Event history: ${this.eventBus.getHistory().length} events`,
      duration: 0,
    }));

    this.registerCheck('memory', async () => {
      const usage = process.memoryUsage();
      const heapUsedMb = Math.round(usage.heapUsed / 1024 / 1024);
      return {
        name: 'memory',
        status: heapUsedMb > 500 ? 'warn' : 'pass',
        message: `Heap: ${heapUsedMb}MB`,
        duration: 0,
      };
    });

    this.intervalId = setInterval(() => {
      this.check();
    }, this.checkInterval);

    logger.info(`Health checker started (interval: ${this.checkInterval}ms)`);
  }

  stop(): void {
    if (this.intervalId) {
      clearInterval(this.intervalId);
      this.intervalId = null;
    }
    logger.info('Health checker stopped');
  }

  async check(): Promise<HealthStatus> {
    const startTime = Date.now();
    const checks: HealthCheck[] = [];

    for (const [name, checkFn] of this.checks) {
      try {
        const checkStart = Date.now();
        const result = await checkFn();
        result.duration = Date.now() - checkStart;
        checks.push(result);
      } catch (error) {
        checks.push({
          name,
          status: 'fail',
          message: String(error),
          duration: Date.now() - startTime,
        });
      }
    }

    const hasFail = checks.some(c => c.status === 'fail');
    const hasWarn = checks.some(c => c.status === 'warn');

    const status: HealthStatus = {
      status: hasFail ? 'unhealthy' : hasWarn ? 'degraded' : 'healthy',
      checks,
      timestamp: new Date(),
    };

    this.eventBus.emit('health.check.completed', status);
    return status;
  }
}
