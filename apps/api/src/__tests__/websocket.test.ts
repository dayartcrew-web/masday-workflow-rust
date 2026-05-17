// ============================================================
// Tests for WebSocket server
// ============================================================

import { describe, it, expect, beforeAll, afterAll } from 'vitest';
import { WebSocketAPIServer } from '../websocket/server';
import { createUser, signToken } from '../auth/jwt';
import { WebSocket } from 'ws';

describe('WebSocketAPIServer', () => {
  const TEST_PORT = 3099;
  let wsServer: WebSocketAPIServer;
  let validToken: string;

  beforeAll(async () => {
    const user = createUser({ email: 'ws-test@example.com', name: 'WS Test' });
    validToken = signToken({ userId: user.id, email: user.email, role: user.role });

    wsServer = new WebSocketAPIServer({
      port: TEST_PORT,
      maxConnections: 5,
      messageRateMax: 5,
      messageRateWindow: 1000,
    });

    await wsServer.start();
  });

  afterAll(async () => {
    await wsServer.stop();
  });

  it('should reject connection without token', async () => {
    return new Promise<void>((resolve) => {
      const ws = new WebSocket(`ws://localhost:${TEST_PORT}`);

      ws.on('close', (code: number, _reason: Buffer) => {
        expect(code).toBe(4001);
        resolve();
      });

      ws.on('error', () => {
        // Expected
      });
    });
  });

  it('should reject connection with invalid token', async () => {
    return new Promise<void>((resolve) => {
      const ws = new WebSocket(`ws://localhost:${TEST_PORT}?token=invalid`);

      ws.on('close', (code: number) => {
        expect(code).toBe(4003);
        resolve();
      });

      ws.on('error', () => {
        // Expected
      });
    });
  });

  it('should accept connection with valid token', async () => {
    return new Promise<void>((resolve) => {
      const ws = new WebSocket(`ws://localhost:${TEST_PORT}?token=${validToken}`);

      ws.on('message', (data: Buffer) => {
        const msg = JSON.parse(data.toString());
        expect(msg.type).toBe('system.connected');
        expect(msg.data.userId).toBeTruthy();
        ws.close();
        resolve();
      });

      ws.on('error', () => {
        // May happen on close
      });
    });
  });

  it('should handle subscribe/unsubscribe messages', async () => {
    return new Promise<void>((resolve) => {
      const ws = new WebSocket(`ws://localhost:${TEST_PORT}?token=${validToken}`);
      let messageCount = 0;

      ws.on('message', (data: Buffer) => {
        const msg = JSON.parse(data.toString());
        messageCount++;

        if (messageCount === 1) {
          // Connected — subscribe to a new channel
          expect(msg.type).toBe('system.connected');
          ws.send(JSON.stringify({ action: 'subscribe', channel: 'custom' }));
        } else if (messageCount === 2) {
          // Subscribed
          expect(msg.type).toBe('system.subscribed');
          expect(msg.data.channel).toBe('custom');
          ws.send(JSON.stringify({ action: 'unsubscribe', channel: 'custom' }));
        } else if (messageCount === 3) {
          // Unsubscribed
          expect(msg.type).toBe('system.unsubscribed');
          expect(msg.data.channel).toBe('custom');
          ws.close();
          resolve();
        }
      });

      ws.on('error', () => {});
    });
  });

  it('should enforce rate limits on messages', async () => {
    return new Promise<void>((resolve) => {
      const ws = new WebSocket(`ws://localhost:${TEST_PORT}?token=${validToken}`);

      ws.on('message', (data: Buffer) => {
        const msg = JSON.parse(data.toString());
        if (msg.type === 'system.connected') {
          // Send many messages quickly (rate max is 5)
          for (let i = 0; i < 10; i++) {
            ws.send(JSON.stringify({ action: 'subscribe', channel: `ch${i}` }));
          }
        }
      });

      ws.on('close', (code: number) => {
        expect(code).toBe(1008); // Rate limit exceeded
        resolve();
      });

      ws.on('error', () => {});
    });
  });

  it('should report client count', () => {
    // After tests, clients should be disconnected
    // Just verify the method exists and returns a number
    expect(typeof wsServer.getClientCount()).toBe('number');
  });

  it('should report uptime', () => {
    const uptime = wsServer.getUptime();
    expect(uptime).toBeGreaterThanOrEqual(0);
  });
});
