/**
 * Fallback provider that wraps a primary and fallback provider
 * with circuit breaker protection.
 *
 * Tries the primary provider through the circuit breaker.
 * If the circuit is open or the primary fails, falls back to the secondary.
 */

import { createLogger } from '@mcp-rebuild/core';
import type { ILLMProvider, LLMOptions, LLMResponse } from './types.js';
import { CircuitBreaker, type CircuitBreakerConfig } from './circuit-breaker.js';

const logger = createLogger('llm:fallback');

export class FallbackProvider implements ILLMProvider {
  private readonly primary: ILLMProvider;
  private readonly fallback: ILLMProvider;
  private readonly circuit: CircuitBreaker;

  constructor(
    primary: ILLMProvider,
    fallback: ILLMProvider,
    circuitConfig?: Partial<CircuitBreakerConfig>,
  ) {
    this.primary = primary;
    this.fallback = fallback;
    this.circuit = new CircuitBreaker(circuitConfig);
  }

  async complete(prompt: string, options?: LLMOptions): Promise<LLMResponse> {
    return this.executeWithFallback(
      () => this.primary.complete(prompt, options),
      () => this.fallback.complete(prompt, options),
      'complete',
    );
  }

  async chat(
    messages: Array<{ role: 'system' | 'user' | 'assistant'; content: string }>,
    options?: LLMOptions,
  ): Promise<LLMResponse> {
    return this.executeWithFallback(
      () => this.primary.chat(messages, options),
      () => this.fallback.chat(messages, options),
      'chat',
    );
  }

  /** Get current circuit breaker state for monitoring. */
  get circuitState() {
    return this.circuit.currentState;
  }

  /** Reset the circuit breaker. */
  resetCircuit(): void {
    this.circuit.reset();
  }

  private async executeWithFallback(
    primaryFn: () => Promise<LLMResponse>,
    fallbackFn: () => Promise<LLMResponse>,
    method: string,
  ): Promise<LLMResponse> {
    try {
      const result = await this.circuit.execute(primaryFn);
      return result;
    } catch (error: unknown) {
      const message = error instanceof Error ? error.message : String(error);
      logger.info(
        { method, error: message, circuitState: this.circuit.currentState },
        'Primary provider failed, trying fallback',
      );

      try {
        const fallbackResult = await fallbackFn();
        logger.info({ method }, 'Fallback provider succeeded');
        return fallbackResult;
      } catch (fallbackError: unknown) {
        const fallbackMessage =
          fallbackError instanceof Error ? fallbackError.message : String(fallbackError);
        logger.error(
          { method, primaryError: message, fallbackError: fallbackMessage },
          'Both primary and fallback providers failed',
        );
        throw fallbackError;
      }
    }
  }
}
