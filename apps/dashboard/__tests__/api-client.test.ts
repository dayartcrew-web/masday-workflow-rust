import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { authApi, providerApi, WebSocketClient } from '@/lib/api-client';

type MockFetchResponse = {
  ok: boolean;
  status: number;
  statusText?: string;
  headers?: Record<string, string>;
  json?: () => Promise<unknown>;
  text?: () => Promise<string>;
};

function createMockFetchResponse(input: MockFetchResponse): Response {
  const headers = new Headers(input.headers ?? {});

  const res = {
    ok: input.ok,
    status: input.status,
    statusText: input.statusText ?? '',
    headers,
    json: input.json ?? (async () => ({})),
    text: input.text ?? (async () => ''),
  };

  return res as unknown as Response;
}

class MockWebSocket {
  static instances: MockWebSocket[] = [];
  static OPEN = 1;

  readyState = MockWebSocket.OPEN;
  onopen: (() => void) | null = null;
  onmessage: ((event: MessageEvent) => void) | null = null;
  onclose: (() => void) | null = null;
  onerror: (() => void) | null = null;
  sent: string[] = [];

  constructor(public readonly url: string) {
    MockWebSocket.instances.push(this);
  }

  send(message: string): void {
    this.sent.push(message);
  }

  close(): void {
    this.onclose?.();
  }
}

describe('WebSocketClient', () => {
  beforeEach(() => {
    vi.useFakeTimers();
    MockWebSocket.instances = [];
    vi.stubGlobal('WebSocket', MockWebSocket as unknown as typeof WebSocket);
  });

  afterEach(() => {
    vi.unstubAllGlobals();
    vi.useRealTimers();
  });

  it('does not reconnect after an intentional disconnect', () => {
    const client = new WebSocketClient();

    client.connect('token-1');
    expect(MockWebSocket.instances).toHaveLength(1);

    client.disconnect();
    vi.runAllTimers();

    expect(MockWebSocket.instances).toHaveLength(1);
    expect(client.connected).toBe(false);
  });
});

describe('HTTP request parsing', () => {
  beforeEach(() => {
    vi.stubGlobal('fetch', vi.fn());
  });

  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it('returns undefined for 204 responses with no body', async () => {
    const jsonSpy = vi.fn(async () => {
      throw new Error('json should not be called');
    });

    (fetch as unknown as ReturnType<typeof vi.fn>).mockResolvedValue(
      createMockFetchResponse({
        ok: true,
        status: 204,
        headers: { 'content-type': 'application/json' },
        json: jsonSpy,
      }),
    );

    await expect(authApi.getMe() as unknown as Promise<unknown>).resolves.toBeUndefined();
    expect(jsonSpy).not.toHaveBeenCalled();
  });

  it('falls back to text when content-type is not JSON', async () => {
    const textSpy = vi.fn(async () => 'OK');

    (fetch as unknown as ReturnType<typeof vi.fn>).mockResolvedValue(
      createMockFetchResponse({
        ok: true,
        status: 200,
        headers: { 'content-type': 'text/plain' },
        text: textSpy,
      }),
    );

    await expect(providerApi.test('demo') as Promise<unknown>).resolves.toBe('OK');
    expect(textSpy).toHaveBeenCalledTimes(1);
  });

  it('throws a rich error message for non-JSON error responses', async () => {
    const textSpy = vi.fn(async () => 'Bad Request');

    (fetch as unknown as ReturnType<typeof vi.fn>).mockResolvedValue(
      createMockFetchResponse({
        ok: false,
        status: 400,
        statusText: 'Bad Request',
        headers: { 'content-type': 'text/plain' },
        text: textSpy,
      }),
    );

    await expect(providerApi.test('demo') as Promise<unknown>).rejects.toThrow(/400/);
    await expect(providerApi.test('demo') as Promise<unknown>).rejects.toThrow(/Bad Request/);
    expect(textSpy).toHaveBeenCalled();
  });
});
