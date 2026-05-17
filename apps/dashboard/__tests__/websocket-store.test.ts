import { beforeEach, describe, expect, it, vi } from 'vitest';

const mockState = {
  connected: false,
  connect: vi.fn(),
  disconnect: vi.fn(),
  onEvent: vi.fn(),
};

vi.mock('@/lib/api-client', () => ({
  wsClient: mockState,
}));

describe('useWebSocketStore', () => {
  beforeEach(() => {
    vi.resetModules();
    vi.clearAllMocks();
    mockState.connected = false;
    mockState.onEvent.mockReturnValue(() => {});
  });

  it('replaces the previous event subscription and polling interval on reconnect', async () => {
    vi.useFakeTimers();

    const clearIntervalSpy = vi.spyOn(globalThis, 'clearInterval');
    const firstUnsubscribe = vi.fn();
    const secondUnsubscribe = vi.fn();
    mockState.onEvent
      .mockReturnValueOnce(firstUnsubscribe)
      .mockReturnValueOnce(secondUnsubscribe);

    const { useWebSocketStore } = await import('@/stores/websocket-store');

    useWebSocketStore.getState().connect('first-token');
    useWebSocketStore.getState().connect('second-token');

    expect(firstUnsubscribe).toHaveBeenCalledTimes(1);
    expect(clearIntervalSpy).toHaveBeenCalledTimes(1);
    expect(mockState.connect).toHaveBeenNthCalledWith(1, 'first-token');
    expect(mockState.connect).toHaveBeenNthCalledWith(2, 'second-token');

    vi.useRealTimers();
  });
});
