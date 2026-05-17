import { describe, it, expect, vi, beforeEach } from 'vitest';
import { FallbackProvider } from '../fallback.js';
import type { ILLMProvider, LLMResponse } from '../types.js';

function mockProvider(
  name: string,
  responseOverrides?: Partial<LLMResponse>,
  shouldFail = false,
): ILLMProvider {
  const defaultResponse: LLMResponse = {
    text: `response from ${name}`,
    tokensUsed: 50,
    latencyMs: 100,
    model: `${name}-model`,
    finishReason: 'stop',
    ...responseOverrides,
  };

  return {
    complete: shouldFail
      ? vi.fn().mockRejectedValue(new Error(`${name} failed`))
      : vi.fn().mockResolvedValue(defaultResponse),
    chat: shouldFail
      ? vi.fn().mockRejectedValue(new Error(`${name} failed`))
      : vi.fn().mockResolvedValue(defaultResponse),
  };
}

describe('FallbackProvider', () => {
  let primary: ILLMProvider;
  let fallback: ILLMProvider;

  beforeEach(() => {
    vi.clearAllMocks();
  });

  describe('when primary succeeds', () => {
    it('should use primary for complete', async () => {
      primary = mockProvider('primary');
      fallback = mockProvider('fallback');

      const provider = new FallbackProvider(primary, fallback, {
        failureThreshold: 1,
      });

      const result = await provider.complete('test prompt');

      expect(result.text).toBe('response from primary');
      expect(primary.complete).toHaveBeenCalledWith('test prompt', undefined);
      expect(fallback.complete).not.toHaveBeenCalled();
    });

    it('should use primary for chat', async () => {
      primary = mockProvider('primary');
      fallback = mockProvider('fallback');

      const provider = new FallbackProvider(primary, fallback, {
        failureThreshold: 1,
      });

      const messages = [{ role: 'user' as const, content: 'hello' }];
      const options = { temperature: 0.5 };
      const result = await provider.chat(messages, options);

      expect(result.text).toBe('response from primary');
      expect(primary.chat).toHaveBeenCalledWith(messages, options);
      expect(fallback.chat).not.toHaveBeenCalled();
    });
  });

  describe('when primary fails', () => {
    it('should fall back to fallback provider for complete', async () => {
      primary = mockProvider('primary', undefined, true);
      fallback = mockProvider('fallback');

      const provider = new FallbackProvider(primary, fallback, {
        failureThreshold: 1,
      });

      // First call: primary fails, fallback succeeds within same call
      const result = await provider.complete('test');
      expect(result.text).toBe('response from fallback');
      expect(fallback.complete).toHaveBeenCalledWith('test', undefined);
    });

    it('should fall back to fallback provider for chat', async () => {
      primary = mockProvider('primary', undefined, true);
      fallback = mockProvider('fallback');

      const provider = new FallbackProvider(primary, fallback, {
        failureThreshold: 1,
      });

      const messages = [{ role: 'user' as const, content: 'hello' }];
      const result = await provider.chat(messages);
      expect(result.text).toBe('response from fallback');
      expect(fallback.chat).toHaveBeenCalledWith(messages, undefined);
    });

    it('should throw when both providers fail', async () => {
      primary = mockProvider('primary', undefined, true);
      fallback = mockProvider('fallback', undefined, true);

      const provider = new FallbackProvider(primary, fallback, {
        failureThreshold: 1,
      });

      // Both fail: primary error is caught, then fallback error is thrown
      await expect(provider.complete('test')).rejects.toThrow('fallback failed');
    });

    it('should track circuit state after primary failures', async () => {
      primary = mockProvider('primary', undefined, true);
      fallback = mockProvider('fallback');

      const provider = new FallbackProvider(primary, fallback, {
        failureThreshold: 1,
      });

      // First failure causes circuit to open (threshold=1)
      await provider.complete('test');

      // Circuit should be open now because primary failed
      expect(provider.circuitState).toBe('open');

      // Subsequent calls should skip primary and go straight to fallback
      const result = await provider.complete('test2');
      expect(result.text).toBe('response from fallback');
    });
  });

  describe('circuit breaker state', () => {
    it('should expose circuit state', () => {
      primary = mockProvider('primary');
      fallback = mockProvider('fallback');

      const provider = new FallbackProvider(primary, fallback);
      expect(provider.circuitState).toBe('closed');
    });

    it('should allow manual circuit reset', async () => {
      primary = mockProvider('primary', undefined, true);
      fallback = mockProvider('fallback');

      const provider = new FallbackProvider(primary, fallback, {
        failureThreshold: 1,
      });

      // Trip the circuit
      await provider.complete('test');
      expect(provider.circuitState).toBe('open');

      // Reset
      provider.resetCircuit();
      expect(provider.circuitState).toBe('closed');
    });
  });
});
