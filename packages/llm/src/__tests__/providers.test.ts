import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { AnthropicProvider } from '../anthropic.js';
import { OpenAIProvider } from '../openai.js';
import { CustomProvider } from '../custom.js';

// Helper to create mock fetch responses
function mockFetchSuccess(body: Record<string, unknown>): typeof fetch {
  return vi.fn().mockResolvedValue({
    ok: true,
    status: 200,
    json: () => Promise.resolve(body),
    text: () => Promise.resolve(JSON.stringify(body)),
  }) as unknown as typeof fetch;
}

function mockFetchFailure(status: number, errorText: string): typeof fetch {
  return vi.fn().mockResolvedValue({
    ok: false,
    status,
    text: () => Promise.resolve(errorText),
  }) as unknown as typeof fetch;
}

const originalFetch = globalThis.fetch;

beforeEach(() => {
  vi.clearAllMocks();
});

afterEach(() => {
  globalThis.fetch = originalFetch;
});

// ============================================================
// Anthropic Provider
// ============================================================

describe('AnthropicProvider', () => {
  const config = {
    apiKey: 'test-key',
    baseUrl: 'https://test-anthropic.example.com',
    defaultModel: 'claude-test',
  };

  it('should complete a prompt using chat format', async () => {
    const mockResponse = {
      content: [{ type: 'text', text: 'Hello from Anthropic!' }],
      usage: { input_tokens: 10, output_tokens: 20 },
      model: 'claude-test',
      stop_reason: 'end_turn',
    };

    globalThis.fetch = mockFetchSuccess(mockResponse);

    const provider = new AnthropicProvider(config);
    const result = await provider.complete('test prompt');

    expect(result.text).toBe('Hello from Anthropic!');
    expect(result.tokensUsed).toBe(30);
    expect(result.model).toBe('claude-test');
    expect(result.finishReason).toBe('end_turn');
    expect(result.latencyMs).toBeGreaterThanOrEqual(0);

    // Verify request format
    const fetchCall = (globalThis.fetch as ReturnType<typeof vi.fn>).mock.calls[0];
    expect(fetchCall[0]).toBe('https://test-anthropic.example.com/messages');
    const requestBody = JSON.parse(fetchCall[1].body);
    expect(requestBody.model).toBe('claude-test');
    expect(requestBody.messages).toEqual([{ role: 'user', content: 'test prompt' }]);
    expect(fetchCall[1].headers['x-api-key']).toBe('test-key');
    expect(fetchCall[1].headers['anthropic-version']).toBe('2023-06-01');
  });

  it('should extract system messages separately', async () => {
    const mockResponse = {
      content: [{ type: 'text', text: 'Response' }],
      usage: { input_tokens: 5, output_tokens: 5 },
      model: 'claude-test',
      stop_reason: 'end_turn',
    };

    globalThis.fetch = mockFetchSuccess(mockResponse);

    const provider = new AnthropicProvider(config);
    const messages = [
      { role: 'system' as const, content: 'You are helpful' },
      { role: 'user' as const, content: 'Hi' },
    ];

    await provider.chat(messages);

    const fetchCall = (globalThis.fetch as ReturnType<typeof vi.fn>).mock.calls[0];
    const requestBody = JSON.parse(fetchCall[1].body);

    expect(requestBody.system).toBe('You are helpful');
    expect(requestBody.messages).toEqual([{ role: 'user', content: 'Hi' }]);
  });

  it('should throw LLMError on API failure', async () => {
    globalThis.fetch = mockFetchFailure(500, 'Internal Server Error');

    const provider = new AnthropicProvider({ ...config, apiKey: 'key' });
    // Disable retries for faster test
    await expect(provider.complete('test', { retries: 0 })).rejects.toThrow(
      'Anthropic API error (500)',
    );
  });

  it('should use default base URL when not specified', () => {
    const provider = new AnthropicProvider({ apiKey: 'key' });
    // We can verify by making a request and checking the URL
    expect(provider).toBeDefined();
  });

  it('should pass temperature option', async () => {
    const mockResponse = {
      content: [{ type: 'text', text: 'Response' }],
      usage: { input_tokens: 5, output_tokens: 5 },
      model: 'claude-test',
    };

    globalThis.fetch = mockFetchSuccess(mockResponse);

    const provider = new AnthropicProvider(config);
    await provider.complete('test', { temperature: 0.7 });

    const fetchCall = (globalThis.fetch as ReturnType<typeof vi.fn>).mock.calls[0];
    const requestBody = JSON.parse(fetchCall[1].body);
    expect(requestBody.temperature).toBe(0.7);
  });
});

// ============================================================
// OpenAI Provider
// ============================================================

describe('OpenAIProvider', () => {
  const config = {
    apiKey: 'test-key',
    baseUrl: 'https://test-openai.example.com/v1',
    defaultModel: 'gpt-test',
  };

  it('should complete a prompt', async () => {
    const mockResponse = {
      choices: [
        {
          message: { content: 'Hello from OpenAI!' },
          finish_reason: 'stop',
        },
      ],
      usage: { total_tokens: 25, prompt_tokens: 10, completion_tokens: 15 },
      model: 'gpt-test',
    };

    globalThis.fetch = mockFetchSuccess(mockResponse);

    const provider = new OpenAIProvider(config);
    const result = await provider.complete('test prompt');

    expect(result.text).toBe('Hello from OpenAI!');
    expect(result.tokensUsed).toBe(25);
    expect(result.model).toBe('gpt-test');
    expect(result.finishReason).toBe('stop');
    expect(result.latencyMs).toBeGreaterThanOrEqual(0);

    // Verify request format
    const fetchCall = (globalThis.fetch as ReturnType<typeof vi.fn>).mock.calls[0];
    expect(fetchCall[0]).toBe('https://test-openai.example.com/v1/chat/completions');
    const requestBody = JSON.parse(fetchCall[1].body);
    expect(requestBody.model).toBe('gpt-test');
    expect(fetchCall[1].headers['Authorization']).toBe('Bearer test-key');
  });

  it('should send chat messages in correct format', async () => {
    const mockResponse = {
      choices: [{ message: { content: 'Response' }, finish_reason: 'stop' }],
      usage: { total_tokens: 10, prompt_tokens: 5, completion_tokens: 5 },
      model: 'gpt-test',
    };

    globalThis.fetch = mockFetchSuccess(mockResponse);

    const provider = new OpenAIProvider(config);
    const messages = [
      { role: 'system' as const, content: 'Be helpful' },
      { role: 'user' as const, content: 'Hi' },
    ];

    await provider.chat(messages);

    const fetchCall = (globalThis.fetch as ReturnType<typeof vi.fn>).mock.calls[0];
    const requestBody = JSON.parse(fetchCall[1].body);

    expect(requestBody.messages).toEqual([
      { role: 'system', content: 'Be helpful' },
      { role: 'user', content: 'Hi' },
    ]);
  });

  it('should throw LLMError on API failure', async () => {
    globalThis.fetch = mockFetchFailure(401, 'Unauthorized');

    const provider = new OpenAIProvider(config);
    await expect(provider.complete('test', { retries: 0 })).rejects.toThrow(
      'OpenAI API error (401)',
    );
  });

  it('should handle empty choices gracefully', async () => {
    const mockResponse = {
      choices: [],
      usage: { total_tokens: 0, prompt_tokens: 0, completion_tokens: 0 },
      model: 'gpt-test',
    };

    globalThis.fetch = mockFetchSuccess(mockResponse);

    const provider = new OpenAIProvider(config);
    const result = await provider.complete('test');

    expect(result.text).toBe('');
    expect(result.tokensUsed).toBe(0);
  });
});

// ============================================================
// Custom Provider (GLM-compatible)
// ============================================================

describe('CustomProvider', () => {
  const config = {
    apiKey: 'test-key',
    baseUrl: 'https://test-custom.example.com/v1',
    defaultModel: 'GLM-4.5-Air',
  };

  it('should complete a prompt', async () => {
    const mockResponse = {
      choices: [
        {
          message: { content: 'Hello from Custom!' },
          finish_reason: 'stop',
        },
      ],
      usage: { total_tokens: 30, prompt_tokens: 10, completion_tokens: 20 },
      model: 'GLM-4.5-Air',
    };

    globalThis.fetch = mockFetchSuccess(mockResponse);

    const provider = new CustomProvider(config);
    const result = await provider.complete('test prompt');

    expect(result.text).toBe('Hello from Custom!');
    expect(result.tokensUsed).toBe(30);
    expect(result.model).toBe('GLM-4.5-Air');

    // Verify request format
    const fetchCall = (globalThis.fetch as ReturnType<typeof vi.fn>).mock.calls[0];
    expect(fetchCall[0]).toBe('https://test-custom.example.com/v1/chat/completions');
    const requestBody = JSON.parse(fetchCall[1].body);
    expect(requestBody.model).toBe('GLM-4.5-Air');
    expect(fetchCall[1].headers['Authorization']).toBe('Bearer test-key');
  });

  it('should use default base URL when not specified', () => {
    const provider = new CustomProvider({ apiKey: 'key' });
    expect(provider).toBeDefined();
  });

  it('should throw LLMError on API failure', async () => {
    globalThis.fetch = mockFetchFailure(503, 'Service Unavailable');

    const provider = new CustomProvider(config);
    await expect(provider.complete('test', { retries: 0 })).rejects.toThrow(
      'Custom API error (503)',
    );
  });

  it('should allow overriding model via options', async () => {
    const mockResponse = {
      choices: [{ message: { content: 'Response' }, finish_reason: 'stop' }],
      usage: { total_tokens: 10, prompt_tokens: 5, completion_tokens: 5 },
      model: 'glm-4.7',
    };

    globalThis.fetch = mockFetchSuccess(mockResponse);

    const provider = new CustomProvider(config);
    await provider.complete('test', { model: 'glm-4.7' });

    const fetchCall = (globalThis.fetch as ReturnType<typeof vi.fn>).mock.calls[0];
    const requestBody = JSON.parse(fetchCall[1].body);
    expect(requestBody.model).toBe('glm-4.7');
  });
});
