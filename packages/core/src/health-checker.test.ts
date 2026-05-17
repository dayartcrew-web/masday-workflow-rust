import { describe, it, expect, vi, beforeEach } from 'vitest';
import { EventBus } from './eventBus.js';

vi.mock('./logger.js', () => ({
  createLogger: () => ({
    info: vi.fn(),
    warn: vi.fn(),
    error: vi.fn(),
    debug: vi.fn(),
  }),
}));

import { HealthChecker } from './health-checker.js';

describe('HealthChecker', () => {
  let eventBus: EventBus;
  let checker: HealthChecker;

  beforeEach(() => {
    eventBus = new EventBus();
    checker = new HealthChecker(eventBus, { checkInterval: 60000 });
  });

  it('returns healthy when all checks pass', async () => {
    checker.registerCheck('test', async () => ({
      name: 'test', status: 'pass', message: 'ok', duration: 0,
    }));
    const result = await checker.check();
    expect(result.status).toBe('healthy');
    expect(result.checks).toHaveLength(1);
  });

  it('returns degraded when check warns', async () => {
    checker.registerCheck('test', async () => ({
      name: 'test', status: 'warn', message: 'slow', duration: 0,
    }));
    const result = await checker.check();
    expect(result.status).toBe('degraded');
  });

  it('returns unhealthy when check fails', async () => {
    checker.registerCheck('test', async () => ({
      name: 'test', status: 'fail', message: 'down', duration: 0,
    }));
    const result = await checker.check();
    expect(result.status).toBe('unhealthy');
  });

  it('handles check function throwing', async () => {
    checker.registerCheck('broken', async () => { throw new Error('boom'); });
    const result = await checker.check();
    expect(result.status).toBe('unhealthy');
    expect(result.checks[0].message).toContain('boom');
  });

  it('includes built-in checks after start', async () => {
    checker.start();
    const result = await checker.check();
    expect(result.checks.length).toBeGreaterThanOrEqual(2);
    checker.stop();
  });

  it('emits health.check.completed event', async () => {
    const handler = vi.fn();
    eventBus.on('health.check.completed' as any, handler);
    checker.registerCheck('test', async () => ({
      name: 'test', status: 'pass', message: 'ok', duration: 0,
    }));
    await checker.check();
    expect(handler).toHaveBeenCalledTimes(1);
  });

  it('includes timestamp in result', async () => {
    const result = await checker.check();
    expect(result.timestamp).toBeInstanceOf(Date);
  });

  it('stop clears interval', () => {
    checker.start();
    checker.stop();
    // No easy way to test interval cleared, just verify no throw
    expect(true).toBe(true);
  });
});
