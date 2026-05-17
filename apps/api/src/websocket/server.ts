// ============================================================
// WebSocket server — JWT auth, typed event channels, rate limiting
// ============================================================

import { WebSocketServer, WebSocket } from 'ws';
import { verifyToken } from '../auth/jwt';
import type { TokenPayload } from '../auth/jwt';
import type { EventBus, EventType, Event } from '@mcp-rebuild/core';

export interface WebSocketConfig {
  /** Server port (default 3001) */
  port: number;
  /** Max concurrent connections (default 100) */
  maxConnections: number;
  /** Max messages per client per rate window (default 10) */
  messageRateMax: number;
  /** Rate limit window in milliseconds (default 1000 = 1 sec) */
  messageRateWindow: number;
}

interface ClientState {
  socket: WebSocket;
  payload: TokenPayload;
  channels: Set<string>;
  rateBucket: { count: number; resetAt: number };
  connectedAt: number;
}

type EventChannel = 'workflow' | 'task' | 'memory' | 'system';

/** Map EventType to channel */
function eventToChannel(type: EventType): EventChannel | null {
  if (type.startsWith('workflow.')) return 'workflow';
  if (type.startsWith('task.') || type.startsWith('agent.')) return 'task';
  if (type.startsWith('memory.') || type.startsWith('search.')) return 'memory';
  if (type.startsWith('system.') || type.startsWith('health.') || type.startsWith('metrics.')) return 'system';
  return null;
}

export class WebSocketAPIServer {
  private wss: WebSocketServer | null = null;
  private config: WebSocketConfig;
  private clients: Map<WebSocket, ClientState> = new Map();
  private eventBus: EventBus | null = null;
  private startTime = Date.now();

  constructor(config?: Partial<WebSocketConfig>) {
    this.config = {
      port: 3001,
      maxConnections: 100,
      messageRateMax: 10,
      messageRateWindow: 1000,
      ...config,
    };
  }

  /** Set the EventBus to subscribe to events */
  setEventBus(eventBus: EventBus): void {
    this.eventBus = eventBus;
  }

  /** Start the WebSocket server */
  start(): Promise<void> {
    return new Promise((resolve) => {
      this.wss = new WebSocketServer({ port: this.config.port });

      this.wss.on('connection', (ws, req) => {
        this.handleConnection(ws, req);
      });

      this.wss.on('listening', () => {
        resolve();
      });
    });
  }

  /** Stop the WebSocket server */
  stop(): Promise<void> {
    return new Promise((resolve) => {
      if (!this.wss) return resolve();

      for (const [ws] of this.clients) {
        ws.close(1001, 'Server shutting down');
      }
      this.clients.clear();

      this.wss.close(() => resolve());
    });
  }

  /** Get the number of connected clients */
  getClientCount(): number {
    return this.clients.size;
  }

  /** Get server uptime in seconds */
  getUptime(): number {
    return Math.floor((Date.now() - this.startTime) / 1000);
  }

  private handleConnection(ws: WebSocket, req: import('http').IncomingMessage): void {
    // Connection limit check
    if (this.clients.size >= this.config.maxConnections) {
      ws.close(1013, 'Max connections reached');
      return;
    }

    // JWT authentication via URL query param
    const url = new URL(req.url || '/', `http://localhost:${this.config.port}`);
    const token = url.searchParams.get('token');

    if (!token) {
      ws.close(4001, 'Authentication required');
      return;
    }

    const payload = verifyToken(token);
    if (!payload) {
      ws.close(4003, 'Invalid or expired token');
      return;
    }

    // Initialize client state
    const clientState: ClientState = {
      socket: ws,
      payload,
      channels: new Set(['workflow', 'task', 'memory', 'system']),
      rateBucket: {
        count: 0,
        resetAt: Date.now() + this.config.messageRateWindow,
      },
      connectedAt: Date.now(),
    };
    this.clients.set(ws, clientState);

    // Send welcome message
    this.sendToClient(ws, {
      type: 'system.connected',
      data: { userId: payload.userId, channels: [...clientState.channels] },
    });

    // Handle incoming messages
    ws.on('message', (data: Buffer) => {
      this.handleMessage(ws, data);
    });

    ws.on('close', () => {
      this.clients.delete(ws);
    });

    ws.on('error', () => {
      this.clients.delete(ws);
    });
  }

  private handleMessage(ws: WebSocket, data: Buffer): void {
    const client = this.clients.get(ws);
    if (!client) return;

    // Rate limiting
    const now = Date.now();
    if (now >= client.rateBucket.resetAt) {
      client.rateBucket = { count: 0, resetAt: now + this.config.messageRateWindow };
    }
    client.rateBucket.count++;

    if (client.rateBucket.count > this.config.messageRateMax) {
      ws.close(1008, 'Rate limit exceeded');
      this.clients.delete(ws);
      return;
    }

    // Parse message
    try {
      const message = JSON.parse(data.toString()) as Record<string, unknown>;
      const action = message.action as string;

      if (action === 'subscribe') {
        const channel = message.channel as string;
        client.channels.add(channel);
        this.sendToClient(ws, { type: 'system.subscribed', data: { channel } });
      } else if (action === 'unsubscribe') {
        const channel = message.channel as string;
        client.channels.delete(channel);
        this.sendToClient(ws, { type: 'system.unsubscribed', data: { channel } });
      }
    } catch {
      this.sendToClient(ws, { type: 'system.error', data: { error: 'Invalid message format' } });
    }
  }

  /** Broadcast an event to subscribed clients */
  broadcast(event: Event): void {
    const channel = eventToChannel(event.type);

    for (const [, client] of this.clients) {
      if (channel && client.channels.has(channel)) {
        this.sendToClient(client.socket, {
          type: event.type,
          data: event.data,
          timestamp: event.timestamp.toISOString(),
        });
      }
    }
  }

  private sendToClient(ws: WebSocket, message: Record<string, unknown>): void {
    if (ws.readyState === WebSocket.OPEN) {
      ws.send(JSON.stringify(message));
    }
  }

  /** Subscribe to EventBus events and broadcast them */
  subscribeToEventBus(eventBus: EventBus): void {
    this.eventBus = eventBus;

    // Subscribe to all event types
    const allTypes: EventType[] = [
      'workflow.started', 'workflow.completed', 'workflow.failed', 'workflow.fixing',
      'workflow.paused', 'workflow.resumed', 'workflow.deleted', 'workflow.state.transition',
      'task.started', 'task.completed', 'task.failed',
      'agent.started', 'agent.task.started', 'agent.task.completed', 'agent.task.failed',
      'agent.message',
      'metrics.recorded', 'health.check.completed',
    ];

    for (const type of allTypes) {
      eventBus.on(type, (event: Event) => {
        this.broadcast(event);
      });
    }
  }
}
