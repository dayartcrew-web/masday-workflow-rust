/**
 * @mcp-rebuild/llm - LLM provider integration with resilience patterns.
 *
 * Exports:
 * - Provider classes: AnthropicProvider, OpenAIProvider, CustomProvider
 * - Resilience: CircuitBreaker, FallbackProvider, BudgetManager
 * - Utilities: createLLM factory, estimateTokens, withRetry
 * - Types: All LLM-related types and interfaces
 */

// Types
export type {
  LLMProviderName,
  LLMConfig,
  LLMOptions,
  LLMResponse,
  LLMStreamChunk,
  ChatMessage,
  ILLMProvider,
  ModelTier,
} from './types.js';
export { MODEL_TIERS } from './types.js';

// Error
export { LLMError } from './error.js';

// Utilities
export { estimateTokens, withRetry } from './utils.js';

// Circuit Breaker
export { CircuitBreaker } from './circuit-breaker.js';
export type { CircuitState, CircuitBreakerConfig } from './circuit-breaker.js';

// Budget Manager
export { BudgetManager } from './budget.js';
export type { BudgetConfig } from './budget.js';

// Providers
export { AnthropicProvider } from './anthropic.js';
export type { AnthropicConfig } from './anthropic.js';

export { OpenAIProvider } from './openai.js';
export type { OpenAIConfig } from './openai.js';

export { CustomProvider } from './custom.js';
export type { CustomProviderConfig } from './custom.js';

// Fallback
export { FallbackProvider } from './fallback.js';

// Factory
export { createLLM } from './factory.js';
