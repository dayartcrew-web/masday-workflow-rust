import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { useWebSocketStore } from '@/stores/websocket-store';
import { wsClient } from '@/lib/api-client';

vi.mock('@/lib/api-client', () => ({
  wsClient: {
    connect: vi.fn(),
    disconnect: vi.fn(),
    onEvent: vi.fn(() => vi.fn()),
    connected: false,
  },
}));

describe('useWebSocketStore', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.useFakeTimers();
    (wsClient.connected as boolean) = false;
    useWebSocketStore.setState({
      connected: false,
      events: [],
      latestEvent: null,
      eventCount: 0,
    });
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it('initializes with disconnected status and empty events', () => {
    const state = useWebSocketStore.getState();
    expect(state.connected).toBe(false);
    expect(state.events).toEqual([]);
    expect(state.latestEvent).toBeNull();
    expect(state.eventCount).toBe(0);
  });

  it('connect calls wsClient.connect and sets up event listener', () => {
    const onEventMock = vi.fn(() => vi.fn());
    (wsClient.onEvent as ReturnType<typeof vi.fn>).mockImplementation(onEventMock);

    useWebSocketStore.getState().connect('test-token');

    expect(wsClient.connect).toHaveBeenCalledWith('test-token');
    expect(wsClient.onEvent).toHaveBeenCalled();
  });

  it('disconnect calls wsClient.disconnect and updates state', () => {
    useWebSocketStore.setState({ connected: true });

    useWebSocketStore.getState().disconnect();

    expect(wsClient.disconnect).toHaveBeenCalled();
    expect(useWebSocketStore.getState().connected).toBe(false);
  });

  it('clearEvents resets events array and counters', () => {
    useWebSocketStore.setState({
      events: [{ id: '1', type: 'test' }],
      latestEvent: { id: '1', type: 'test' },
      eventCount: 5,
    });

    useWebSocketStore.getState().clearEvents();

    const state = useWebSocketStore.getState();
    expect(state.events).toEqual([]);
    expect(state.latestEvent).toBeNull();
    expect(state.eventCount).toBe(0);
  });
});
