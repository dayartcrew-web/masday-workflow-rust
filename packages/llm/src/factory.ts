/**
 * Factory function for creating LLM providers from configuration.
 *
 * Reads from environment variables when config is not provided explicitly.
 */

import { createLogger } from '@mcp-rebuild/core';
import type { ILLMProvider, LLMConfig } from './types.js';
import { AnthropicProvider } from './anthropic.js';
import { OpenAIProvider } from './openai.js';
import { CustomProvider } from './custom.js';
import { FallbackProvider } from './fallback.js';

const logger = createLogger('llm:factory');

/**
 * Create an LLM provider from configuration.
 * When config is not provided, reads from environment variables.
 * Falls back to a custom provider pointing at localhost if no keys are configured.
 */
export function createLLM(config?: Partial<LLMConfig>): ILLMProvider {
  const resolvedConfig: LLMConfig = {
    provider: config?.provider || (process.env.LLM_PROVIDER as LLMConfig['provider']) || 'custom',
    apiKey: config?.apiKey || process.env.LLM_API_KEY || '',
    baseUrl: config?.baseUrl || process.env.LLM_BASE_URL,
    defaultModel: config?.defaultModel || process.env.LLM_MODEL,
  };

  const primary = createProvider(resolvedConfig);

  // Check if a fallback provider is configured
  const fallbackProvider = process.env.LLM_FALLBACK_PROVIDER as LLMConfig['provider'] | undefined;
  const fallbackKey = process.env.LLM_FALLBACK_API_KEY;

  if (fallbackProvider && fallbackKey) {
    const fallbackConfig: LLMConfig = {
      provider: fallbackProvider,
      apiKey: fallbackKey,
      baseUrl: process.env.LLM_FALLBACK_BASE_URL,
      defaultModel: process.env.LLM_FALLBACK_MODEL,
    };
    const fallback = createProvider(fallbackConfig);
    logger.info(
      { primary: resolvedConfig.provider, fallback: fallbackProvider },
      'Creating LLM with fallback provider',
    );
    return new FallbackProvider(primary, fallback);
  }

  logger.info({ provider: resolvedConfig.provider }, 'Creating LLM provider');
  return primary;
}

function createProvider(config: LLMConfig): ILLMProvider {
  switch (config.provider) {
    case 'anthropic':
      return new AnthropicProvider({
        apiKey: config.apiKey,
        baseUrl: config.baseUrl,
        defaultModel: config.defaultModel,
      });

    case 'openai':
      return new OpenAIProvider({
        apiKey: config.apiKey,
        baseUrl: config.baseUrl,
        defaultModel: config.defaultModel,
      });

    case 'custom':
      return new CustomProvider({
        apiKey: config.apiKey,
        baseUrl: config.baseUrl,
        defaultModel: config.defaultModel,
      });

    default:
      logger.warn(
        { provider: config.provider },
        'Unknown provider, falling back to custom',
      );
      return new CustomProvider({
        apiKey: config.apiKey,
        baseUrl: config.baseUrl,
        defaultModel: config.defaultModel,
      });
  }
}
