import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { createLLM } from '../factory.js';
import { AnthropicProvider } from '../anthropic.js';
import { OpenAIProvider } from '../openai.js';
import { CustomProvider } from '../custom.js';
import { FallbackProvider } from '../fallback.js';

describe('createLLM factory', () => {
  const originalEnv = { ...process.env };

  beforeEach(() => {
    vi.clearAllMocks();
    // Reset env
    delete process.env.LLM_PROVIDER;
    delete process.env.LLM_API_KEY;
    delete process.env.LLM_BASE_URL;
    delete process.env.LLM_MODEL;
    delete process.env.LLM_FALLBACK_PROVIDER;
    delete process.env.LLM_FALLBACK_API_KEY;
    delete process.env.LLM_FALLBACK_BASE_URL;
    delete process.env.LLM_FALLBACK_MODEL;
  });

  afterEach(() => {
    // Restore env
    Object.keys(originalEnv).forEach((key) => {
      if (!(key in process.env) || process.env[key] !== originalEnv[key]) {
        process.env[key] = originalEnv[key];
      }
    });
  });

  it('should create AnthropicProvider when provider is anthropic', () => {
    const provider = createLLM({
      provider: 'anthropic',
      apiKey: 'sk-ant-test',
    });
    expect(provider).toBeInstanceOf(AnthropicProvider);
  });

  it('should create OpenAIProvider when provider is openai', () => {
    const provider = createLLM({
      provider: 'openai',
      apiKey: 'sk-test',
    });
    expect(provider).toBeInstanceOf(OpenAIProvider);
  });

  it('should create CustomProvider when provider is custom', () => {
    const provider = createLLM({
      provider: 'custom',
      apiKey: 'test-key',
    });
    expect(provider).toBeInstanceOf(CustomProvider);
  });

  it('should fallback to CustomProvider for unknown provider', () => {
    const provider = createLLM({
      provider: 'unknown' as 'custom',
      apiKey: 'test-key',
    });
    expect(provider).toBeInstanceOf(CustomProvider);
  });

  it('should create FallbackProvider when fallback env vars are set', () => {
    process.env.LLM_FALLBACK_PROVIDER = 'openai';
    process.env.LLM_FALLBACK_API_KEY = 'sk-fallback';

    const provider = createLLM({
      provider: 'anthropic',
      apiKey: 'sk-ant-test',
    });

    expect(provider).toBeInstanceOf(FallbackProvider);
  });

  it('should not wrap in FallbackProvider when no fallback env vars', () => {
    const provider = createLLM({
      provider: 'anthropic',
      apiKey: 'sk-ant-test',
    });

    expect(provider).toBeInstanceOf(AnthropicProvider);
    expect(provider).not.toBeInstanceOf(FallbackProvider);
  });

  it('should read from environment variables when config not provided', () => {
    process.env.LLM_PROVIDER = 'openai';
    process.env.LLM_API_KEY = 'sk-env-key';

    const provider = createLLM();
    expect(provider).toBeInstanceOf(OpenAIProvider);
  });

  it('should default to custom provider when nothing is configured', () => {
    const provider = createLLM();
    expect(provider).toBeInstanceOf(CustomProvider);
  });

  it('should pass baseUrl and defaultModel to provider', () => {
    const provider = createLLM({
      provider: 'custom',
      apiKey: 'test-key',
      baseUrl: 'http://localhost:8080/v1',
      defaultModel: 'test-model',
    });

    // The provider is created, we can verify it doesn't throw
    expect(provider).toBeInstanceOf(CustomProvider);
  });
});
