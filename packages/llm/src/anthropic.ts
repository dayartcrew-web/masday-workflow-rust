/**
 * Anthropic LLM provider using raw fetch (zero SDK dependencies).
 *
 * Handles the Anthropic Messages API format:
 * - System message is sent as a top-level parameter, not in the messages array.
 * - Response content is an array of content blocks.
 * - Token usage: input_tokens + output_tokens.
 */

import { createLogger } from '@mcp-rebuild/core';
import type { ILLMProvider, LLMOptions, LLMResponse, ChatMessage } from './types.js';
import { LLMError } from './error.js';
import { withRetry } from './utils.js';

const logger = createLogger('llm:anthropic');

const DEFAULT_BASE_URL = 'https://api.z.ai/api/anthropic';
const ANTHROPIC_VERSION = '2023-06-01';

export interface AnthropicConfig {
  apiKey: string;
  baseUrl?: string;
  defaultModel?: string;
}

export class AnthropicProvider implements ILLMProvider {
  private readonly apiKey: string;
  private readonly baseUrl: string;
  private readonly defaultModel: string;

  constructor(config: AnthropicConfig) {
    this.apiKey = config.apiKey;
    this.baseUrl = config.baseUrl || DEFAULT_BASE_URL;
    this.defaultModel = config.defaultModel || 'claude-sonnet-4-20250514';
  }

  async complete(prompt: string, options?: LLMOptions): Promise<LLMResponse> {
    return this.chat([{ role: 'user', content: prompt }], options);
  }

  async chat(messages: ChatMessage[], options?: LLMOptions): Promise<LLMResponse> {
    const model = options?.model || this.defaultModel;
    const maxTokens = options?.maxTokens || 4096;
    const retries = options?.retries ?? 2;

    // Extract system message separately per Anthropic format
    const systemMessages = messages.filter((m) => m.role === 'system');
    const nonSystemMessages = messages.filter((m) => m.role !== 'system');

    const body: Record<string, unknown> = {
      model,
      max_tokens: maxTokens,
      messages: nonSystemMessages.map((m) => ({
        role: m.role,
        content: m.content,
      })),
    };

    if (systemMessages.length > 0) {
      body.system = systemMessages.map((m) => m.content).join('\n');
    }

    if (options?.temperature !== undefined) {
      body.temperature = options.temperature;
    }

    const url = `${this.baseUrl}/messages`;
    const startTime = Date.now();

    logger.debug({ model, messageCount: messages.length }, 'Anthropic: sending request');

    const execute = async (): Promise<LLMResponse> => {
      const response = await globalThis.fetch(url, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          'x-api-key': this.apiKey,
          'anthropic-version': ANTHROPIC_VERSION,
        },
        body: JSON.stringify(body),
      });

      if (!response.ok) {
        const errorText = await response.text().catch(() => 'Unknown error');
        const retryable = response.status === 429 || response.status >= 500;
        logger.warn(
          { status: response.status, error: errorText, retryable },
          'Anthropic: request failed',
        );
        throw new LLMError({
          message: `Anthropic API error (${response.status}): ${errorText}`,
          provider: 'anthropic',
          statusCode: response.status,
          retryable,
        });
      }

      const data = (await response.json()) as Record<string, unknown>;

      let text: string;
      if (Array.isArray(data.content)) {
        text = (data.content as Array<{ type: string; text: string }>)
          .filter((block) => block.type === 'text')
          .map((block) => block.text)
          .join('');
      } else if (typeof data.content === 'string') {
        text = data.content;
      } else if (Array.isArray((data as { choices?: unknown[] }).choices)) {
        const choices = (data as { choices: Array<{ message?: { content?: string } }> }).choices;
        text = choices[0]?.message?.content || '';
      } else {
        text = String(data.text ?? data.message ?? '');
      }

      const usage = data.usage as { input_tokens?: number; output_tokens?: number; total_tokens?: number } | undefined;
      const tokensUsed = usage?.input_tokens ?? usage?.total_tokens ?? 0;
      const latencyMs = Date.now() - startTime;
      const responseModel = typeof data.model === 'string' ? data.model : model;
      const stopReason = data.stop_reason ?? data.finish_reason as string | undefined;

      logger.debug(
        { model: responseModel, tokensUsed, latencyMs, stopReason },
        'Anthropic: response received',
      );

      return {
        text,
        tokensUsed,
        latencyMs,
        model: responseModel,
        finishReason: stopReason as string | undefined,
      };
    };

    return withRetry(execute, retries, options?.retryDelayMs || 1000);
  }
}
