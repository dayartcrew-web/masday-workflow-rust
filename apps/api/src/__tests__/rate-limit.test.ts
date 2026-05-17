// ============================================================
// Tests for rate limiting
// ============================================================

import { describe, it, expect, beforeEach } from 'vitest';
import { RateLimiter } from '../rate-limit';

describe('RateLimiter', () => {
  let limiter: RateLimiter;

  beforeEach(() => {
    limiter = new RateLimiter({ windowMs: 1000, max: 5 });
  });

  it('should allow requests under the limit', () => {
    const result = limiter.check('client1');
    expect(result.allowed).toBe(true);
    expect(result.remaining).toBe(4);
  });

  it('should block requests over the limit', () => {
    for (let i = 0; i < 5; i++) {
      limiter.check('client1');
    }
    const result = limiter.check('client1');
    expect(result.allowed).toBe(false);
    expect(result.remaining).toBe(0);
    expect(result.retryAfter).toBeGreaterThan(0);
  });

  it('should track clients independently', () => {
    for (let i = 0; i < 5; i++) {
      limiter.check('client1');
    }
    // client1 is at limit
    const result1 = limiter.check('client1');
    expect(result1.allowed).toBe(false);

    // client2 should still be allowed
    const result2 = limiter.check('client2');
    expect(result2.allowed).toBe(true);
    expect(result2.remaining).toBe(4);
  });

  it('should reset after window expires', () => {
    for (let i = 0; i < 5; i++) {
      limiter.check('client1');
    }
    // At limit
    expect(limiter.check('client1').allowed).toBe(false);

    // Wait for window to expire (fast test with 1ms fake)
    // We use a new limiter with expired window
    const fastLimiter = new RateLimiter({ windowMs: 1, max: 2 });
    fastLimiter.check('fast1');
    fastLimiter.check('fast1');

    return new Promise<void>((resolve) => {
      setTimeout(() => {
        const result = fastLimiter.check('fast1');
        expect(result.allowed).toBe(true);
        resolve();
      }, 10);
    });
  });

  it('should support per-route configurations', () => {
    limiter.setRouteConfig('/api/auth', { windowMs: 60000, max: 2 });

    // Auth route should hit limit at 2
    const r1 = limiter.check('client1', '/api/auth/login');
    expect(r1.allowed).toBe(true);
    const r2 = limiter.check('client1', '/api/auth/login');
    expect(r2.allowed).toBe(true);
    const r3 = limiter.check('client1', '/api/auth/login');
    expect(r3.allowed).toBe(false);

    // Other routes still have limit of 5
    const r4 = limiter.check('client1', '/api/workflows');
    expect(r4.allowed).toBe(true);
  });

  it('should return correct resetAt timestamp', () => {
    const before = Date.now();
    const result = limiter.check('client1');
    expect(result.resetAt).toBeGreaterThanOrEqual(before);
    expect(result.resetAt).toBeLessThanOrEqual(before + 2000);
  });

  it('should reset all state', () => {
    for (let i = 0; i < 5; i++) {
      limiter.check('client1');
    }
    expect(limiter.check('client1').allowed).toBe(false);

    limiter.reset();
    expect(limiter.check('client1').allowed).toBe(true);
  });
});
