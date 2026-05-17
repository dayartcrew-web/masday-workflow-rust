import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { CircuitBreaker } from '../circuit-breaker.js';

describe('CircuitBreaker', () => {
  let breaker: CircuitBreaker;

  beforeEach(() => {
    breaker = new CircuitBreaker({
      failureThreshold: 3,
      resetTimeoutMs: 100,
      halfOpenMaxAttempts: 1,
    });
    vi.useFakeTimers();
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  describe('closed state', () => {
    it('should start in closed state', () => {
      expect(breaker.currentState).toBe('closed');
    });

    it('should pass through requests in closed state', async () => {
      const result = await breaker.execute(async () => 'success');
      expect(result).toBe('success');
    });

    it('should track failures and transition to open after threshold', async () => {
      for (let i = 0; i < 3; i++) {
        await expect(
          breaker.execute(async () => {
            throw new Error('fail');
          }),
        ).rejects.toThrow('fail');
      }

      expect(breaker.currentState).toBe('open');
    });

    it('should remain closed if failures are below threshold', async () => {
      await expect(
        breaker.execute(async () => {
          throw new Error('fail');
        }),
      ).rejects.toThrow('fail');

      await expect(
        breaker.execute(async () => {
          throw new Error('fail');
        }),
      ).rejects.toThrow('fail');

      expect(breaker.currentState).toBe('closed');
    });

    it('should reset failure count on success', async () => {
      // Two failures
      for (let i = 0; i < 2; i++) {
        await expect(
          breaker.execute(async () => {
            throw new Error('fail');
          }),
        ).rejects.toThrow('fail');
      }

      // Success resets count
      await breaker.execute(async () => 'ok');

      // Two more failures should not trip (count was reset)
      for (let i = 0; i < 2; i++) {
        await expect(
          breaker.execute(async () => {
            throw new Error('fail');
          }),
        ).rejects.toThrow('fail');
      }

      expect(breaker.currentState).toBe('closed');
    });
  });

  describe('open state', () => {
    beforeEach(async () => {
      // Trip the breaker
      for (let i = 0; i < 3; i++) {
        await expect(
          breaker.execute(async () => {
            throw new Error('fail');
          }),
        ).rejects.toThrow('fail');
      }
      expect(breaker.currentState).toBe('open');
    });

    it('should reject requests when open and cooldown not elapsed', async () => {
      await expect(breaker.execute(async () => 'success')).rejects.toThrow(
        'Circuit breaker is open',
      );
    });

    it('should transition to half-open after cooldown', async () => {
      vi.advanceTimersByTime(150); // past resetTimeoutMs of 100ms

      const result = await breaker.execute(async () => 'recovered');
      expect(result).toBe('recovered');
      expect(breaker.currentState).toBe('closed');
    });

    it('should transition back to open if half-open request fails', async () => {
      vi.advanceTimersByTime(150);

      await expect(
        breaker.execute(async () => {
          throw new Error('still failing');
        }),
      ).rejects.toThrow('still failing');

      expect(breaker.currentState).toBe('open');
    });
  });

  describe('half-open state', () => {
    beforeEach(async () => {
      // Trip the breaker
      for (let i = 0; i < 3; i++) {
        await expect(
          breaker.execute(async () => {
            throw new Error('fail');
          }),
        ).rejects.toThrow('fail');
      }
      // Advance past cooldown to enter half-open
      vi.advanceTimersByTime(150);
    });

    it('should transition to closed on successful request', async () => {
      const result = await breaker.execute(async () => 'recovered');
      expect(result).toBe('recovered');
      expect(breaker.currentState).toBe('closed');
    });

    it('should transition to open on failed request', async () => {
      await expect(
        breaker.execute(async () => {
          throw new Error('still failing');
        }),
      ).rejects.toThrow('still failing');
      expect(breaker.currentState).toBe('open');
    });
  });

  describe('reset', () => {
    it('should reset to closed state', async () => {
      for (let i = 0; i < 3; i++) {
        await expect(
          breaker.execute(async () => {
            throw new Error('fail');
          }),
        ).rejects.toThrow('fail');
      }
      expect(breaker.currentState).toBe('open');

      breaker.reset();
      expect(breaker.currentState).toBe('closed');

      const result = await breaker.execute(async () => 'success');
      expect(result).toBe('success');
    });
  });

  describe('defaults', () => {
    it('should use default config when none provided', () => {
      const defaultBreaker = new CircuitBreaker();
      expect(defaultBreaker.currentState).toBe('closed');
    });

    it('should merge partial config with defaults', () => {
      const partial = new CircuitBreaker({ failureThreshold: 10 });
      // Should still work with default resetTimeoutMs
      expect(partial.currentState).toBe('closed');
    });
  });
});
