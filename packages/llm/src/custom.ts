/**
 * Custom LLM provider (GLM-compatible) using raw fetch.
 *
 * Same format as OpenAI but with configurable base URL.
 * Defaults to local Ollama-compatible endpoint.
 */

import { createLogger } from '@mcp-rebuild/core';
import type { ILLMProvider, LLMOptions, LLMResponse, ChatMessage } from './types.js';
import { LLMError } from './error.js';
import { withRetry } from './utils.js';

const logger = createLogger('llm:custom');

const DEFAULT_BASE_URL = 'http://localhost:11434/v1';

export interface CustomProviderConfig {
  apiKey: string;
  baseUrl?: string;
  defaultModel?: string;
}

export class CustomProvider implements ILLMProvider {
  private readonly apiKey: string;
  private readonly baseUrl: string;
  private readonly defaultModel: string;

  constructor(config: CustomProviderConfig) {
    this.apiKey = config.apiKey;
    this.baseUrl = config.baseUrl || DEFAULT_BASE_URL;
    this.defaultModel = config.defaultModel || 'GLM-4.5-Air';
  }

  async complete(prompt: string, options?: LLMOptions): Promise<LLMResponse> {
    return this.chat([{ role: 'user', content: prompt }], options);
  }

  async chat(messages: ChatMessage[], options?: LLMOptions): Promise<LLMResponse> {
    const model = options?.model || this.defaultModel;
    const maxTokens = options?.maxTokens || 4096;
    const retries = options?.retries ?? 2;

    const body: Record<string, unknown> = {
      model,
      max_tokens: maxTokens,
      messages: messages.map((m) => ({
        role: m.role,
        content: m.content,
      })),
    };

    if (options?.temperature !== undefined) {
      body.temperature = options.temperature;
    }

    const url = `${this.baseUrl}/chat/completions`;
    const startTime = Date.now();

    logger.debug({ model, messageCount: messages.length }, 'Custom: sending request');

    const execute = async (): Promise<LLMResponse> => {
      const response = await globalThis.fetch(url, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          Authorization: `Bearer ${this.apiKey}`,
        },
        body: JSON.stringify(body),
      });

      if (!response.ok) {
        const errorText = await response.text().catch(() => 'Unknown error');
        const retryable = response.status === 429 || response.status >= 500;
        logger.warn(
          { status: response.status, error: errorText, retryable },
          'Custom: request failed',
        );
        throw new LLMError({
          message: `Custom API error (${response.status}): ${errorText}`,
          provider: 'custom',
          statusCode: response.status,
          retryable,
        });
      }

      const data = (await response.json()) as {
        choices: Array<{
          message: { content: string };
          finish_reason?: string;
        }>;
        usage: { total_tokens: number; prompt_tokens: number; completion_tokens: number };
        model: string;
      };

      const text = data.choices[0]?.message?.content || '';
      const promptTokens = data.usage?.prompt_tokens || 0;
      const completionTokens = data.usage?.completion_tokens || 0;
      const tokensUsed = data.usage?.total_tokens || (promptTokens + completionTokens);
      const latencyMs = Date.now() - startTime;

      logger.debug(
        { model: data.model, tokensUsed, promptTokens, completionTokens, latencyMs, finishReason: data.choices[0]?.finish_reason },
        'Custom: response received',
      );

      return {
        text,
        tokensUsed,
        promptTokens,
        completionTokens,
        latencyMs,
        model: data.model,
        finishReason: data.choices[0]?.finish_reason,
      };
    };

    return withRetry(execute, retries, options?.retryDelayMs || 1000);
  }
}
