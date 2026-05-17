// ============================================================
// WebSocket Store — connection state, event stream
// ============================================================

import { create } from 'zustand';
import { wsClient } from '@/lib/api-client';
import type { WSEvent } from '@/lib/types';

interface WebSocketState {
  connected: boolean;
  events: WSEvent[];
  latestEvent: WSEvent | null;
  eventCount: number;
  connect: (token: string) => void;
  disconnect: () => void;
  clearEvents: () => void;
}

let unsubscribeFromEvents: (() => void) | null = null;
let connectionPollInterval: ReturnType<typeof setInterval> | null = null;

function clearConnectionResources(): void {
  if (unsubscribeFromEvents) {
    unsubscribeFromEvents();
    unsubscribeFromEvents = null;
  }
  if (connectionPollInterval) {
    clearInterval(connectionPollInterval);
    connectionPollInterval = null;
  }
}

export const useWebSocketStore = create<WebSocketState>((set) => ({
  connected: false,
  events: [],
  latestEvent: null,
  eventCount: 0,

  connect: (token: string) => {
    clearConnectionResources();

    unsubscribeFromEvents = wsClient.onEvent((event: WSEvent) => {
      set((s) => ({
        events: [...s.events.slice(-199), event], // Keep last 200
        latestEvent: event,
        eventCount: s.eventCount + 1,
      }));
    });
    wsClient.connect(token);

    // Poll connection state until the underlying client offers push-based status updates.
    connectionPollInterval = setInterval(() => {
      set({ connected: wsClient.connected });
    }, 1000);
  },

  disconnect: () => {
    clearConnectionResources();
    wsClient.disconnect();
    set({ connected: false });
  },

  clearEvents: () => set({ events: [], latestEvent: null, eventCount: 0 }),
}));
