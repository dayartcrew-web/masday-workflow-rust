/**
 * Circuit Breaker pattern for LLM provider resilience.
 *
 * States:
 * - closed: Normal operation, requests pass through. Tracks failures.
 * - open: Requests are rejected until cooldown elapses, then switches to half-open.
 * - half-open: Allows a limited number of test requests. Success -> closed, failure -> open.
 */

import { createLogger } from '@mcp-rebuild/core';

const logger = createLogger('llm:circuit-breaker');

export type CircuitState = 'closed' | 'open' | 'half-open';

export interface CircuitBreakerConfig {
  failureThreshold: number;
  resetTimeoutMs: number;
  halfOpenMaxAttempts: number;
}

const DEFAULT_CONFIG: CircuitBreakerConfig = {
  failureThreshold: 5,
  resetTimeoutMs: 30000,
  halfOpenMaxAttempts: 1,
};

export class CircuitBreaker {
  private readonly config: CircuitBreakerConfig;
  private state: CircuitState = 'closed';
  private failureCount = 0;
  private lastFailureTime = 0;
  private halfOpenAttempts = 0;

  constructor(config?: Partial<CircuitBreakerConfig>) {
    this.config = { ...DEFAULT_CONFIG, ...config };
  }

  /** Current circuit state. */
  get currentState(): CircuitState {
    return this.state;
  }

  /** Execute a function through the circuit breaker. */
  async execute<T>(fn: () => Promise<T>): Promise<T> {
    if (this.state === 'open') {
      const elapsed = Date.now() - this.lastFailureTime;
      if (elapsed < this.config.resetTimeoutMs) {
        logger.debug(
          { state: this.state, elapsed, threshold: this.config.resetTimeoutMs },
          'Circuit is open, rejecting request',
        );
        throw new Error(
          `Circuit breaker is open. Retry after ${this.config.resetTimeoutMs - elapsed}ms.`,
        );
      }
      // Cooldown elapsed, transition to half-open
      this.transitionTo('half-open');
    }

    if (this.state === 'half-open') {
      if (this.halfOpenAttempts >= this.config.halfOpenMaxAttempts) {
        this.transitionTo('open');
        throw new Error('Circuit breaker is open. Half-open attempts exhausted.');
      }
      this.halfOpenAttempts++;
    }

    try {
      const result = await fn();
      this.onSuccess();
      return result;
    } catch (error: unknown) {
      this.onFailure();
      throw error;
    }
  }

  /** Manually reset the circuit breaker to closed state. */
  reset(): void {
    this.failureCount = 0;
    this.halfOpenAttempts = 0;
    this.transitionTo('closed');
  }

  private onSuccess(): void {
    if (this.state === 'half-open') {
      logger.info('Circuit breaker: half-open -> closed (success)');
    }
    this.failureCount = 0;
    this.halfOpenAttempts = 0;
    this.state = 'closed';
  }

  private onFailure(): void {
    this.failureCount++;
    this.lastFailureTime = Date.now();

    if (this.state === 'half-open') {
      logger.warn('Circuit breaker: half-open -> open (failure during test)');
      this.state = 'open';
      return;
    }

    if (this.failureCount >= this.config.failureThreshold) {
      logger.warn(
        { failureCount: this.failureCount, threshold: this.config.failureThreshold },
        'Circuit breaker: closed -> open (threshold reached)',
      );
      this.state = 'open';
    }
  }

  private transitionTo(newState: CircuitState): void {
    const oldState = this.state;
    this.state = newState;
    if (newState === 'half-open') {
      this.halfOpenAttempts = 0;
    }
    logger.debug({ from: oldState, to: newState }, 'Circuit breaker state transition');
  }
}
