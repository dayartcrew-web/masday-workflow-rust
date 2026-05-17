'use client';

import { useWebSocketStore } from '@/stores/websocket-store';

export function ConnectionStatus() {
  const connected = useWebSocketStore((s) => s.connected);
  const eventCount = useWebSocketStore((s) => s.eventCount);

  return (
    <div className="glass-surface flex items-center justify-between px-4 py-2 rounded-lg text-xs" style={{ border: '1px solid var(--color-border-subtle)' }}>
      <div className="flex items-center gap-2">
        <span className="relative flex h-2.5 w-2.5">
          {connected && (
            <span className="animate-ping absolute inline-flex h-full w-full rounded-full opacity-75" style={{ background: 'var(--color-neon-green)' }} />
          )}
          <span className="relative inline-flex rounded-full h-2.5 w-2.5" style={{ background: connected ? 'var(--color-neon-green)' : 'var(--color-error)' }} />
        </span>
        <span className="text-[var(--color-text-secondary)] font-medium" style={{ fontSize: 'var(--font-size-small)' }}>
          {connected ? 'Connected' : 'Disconnected'}
        </span>
      </div>
      {connected && (
        <span className="text-[var(--color-text-secondary)]" style={{ fontSize: 'var(--font-size-small)' }}>
          {eventCount} events
        </span>
      )}
    </div>
  );
}
