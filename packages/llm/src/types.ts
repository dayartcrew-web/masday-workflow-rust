/**
 * LLM provider types and interfaces for @mcp-rebuild/llm.
 */

// --- Provider Names ---

export type LLMProviderName = 'openai' | 'anthropic' | 'custom';

// --- Configuration ---

export interface LLMConfig {
  provider: LLMProviderName;
  apiKey: string;
  baseUrl?: string;
  defaultModel?: string;
}

// --- Request Options ---

export interface LLMOptions {
  model?: string;
  temperature?: number;
  maxTokens?: number;
  stream?: boolean;
  retries?: number;
  retryDelayMs?: number;
}

// --- Response ---

export interface LLMResponse {
  text: string;
  tokensUsed: number;
  latencyMs: number;
  model: string;
  finishReason?: string;
}

// --- Streaming ---

export interface LLMStreamChunk {
  text: string;
  done: boolean;
  tokensUsed?: number;
}

// --- Chat Message ---

export interface ChatMessage {
  role: 'system' | 'user' | 'assistant';
  content: string;
}

// --- Provider Interface ---

export interface ILLMProvider {
  complete(prompt: string, options?: LLMOptions): Promise<LLMResponse>;
  chat(
    messages: Array<{ role: 'system' | 'user' | 'assistant'; content: string }>,
    options?: LLMOptions,
  ): Promise<LLMResponse>;
}

// --- Model Tiers ---

export const MODEL_TIERS = {
  cheap: process.env.LLM_MODEL_CHEAP || 'GLM-4.5-Air',
  medium: process.env.LLM_MODEL_MEDIUM || 'glm-4.7',
  powerful: process.env.LLM_MODEL_POWERFUL || 'glm-5',
} as const;

export type ModelTier = keyof typeof MODEL_TIERS;
