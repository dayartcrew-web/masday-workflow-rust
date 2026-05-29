import { createLogger } from '@mcp-rebuild/core';
import { sql } from 'drizzle-orm';

const logger = createLogger('memory:episodic');

// eslint-disable-next-line @typescript-eslint/no-explicit-any
let drizzleDb: any = null;

export function setEpisodicDb(client: unknown): void {
  drizzleDb = client;
}

export interface ChatMessage {
  role: 'user' | 'assistant' | 'system';
  content: string;
  timestamp: number;
  metadata?: Record<string, unknown>;
}

/**
 * EpisodicMemory - ring buffer of the last N messages per session.
 * Maintains recent conversation history with a fixed-size buffer.
 */
export class EpisodicMemory {
  private buffer: ChatMessage[] = [];
  private readonly maxSize: number;

  constructor(maxSize: number = 100) {
    if (maxSize < 1) {
      throw new Error('EpisodicMemory maxSize must be at least 1');
    }
    this.maxSize = maxSize;
  }

  /** Add a message to the ring buffer. Oldest messages are evicted when full. */
  add(role: ChatMessage['role'], content: string, metadata?: Record<string, unknown>): void {
    const message: ChatMessage = {
      role,
      content,
      timestamp: Date.now(),
      ...(metadata ? { metadata } : {}),
    };

    if (this.buffer.length >= this.maxSize) {
      this.buffer.shift();
    }

    this.buffer.push(message);
    this.persistToDb(message);
    logger.debug({ role, bufferSize: this.buffer.length }, 'Added message to episodic buffer');
  }

  /** Get the N most recent messages. */
  getRecent(count?: number): ChatMessage[] {
    const n = count ?? this.buffer.length;
    return this.buffer.slice(-n).map(msg => ({ ...msg }));
  }

  /** Get all messages in order. */
  getAll(): ChatMessage[] {
    return this.buffer.map(msg => ({ ...msg }));
  }

  /** Clear all messages. */
  clear(): void {
    this.buffer = [];
    logger.debug('Cleared episodic buffer');
  }

  /** Format recent messages for LLM prompt injection. */
  toPromptString(count?: number): string {
    const messages = this.getRecent(count);
    if (messages.length === 0) {
      return '';
    }

    const lines: string[] = ['## Recent Conversation'];
    for (const msg of messages) {
      const roleLabel = msg.role === 'user' ? 'User' : msg.role === 'assistant' ? 'Assistant' : 'System';
      lines.push(`**${roleLabel}:** ${msg.content}`);
    }

    return lines.join('\n');
  }

  /** Current number of messages in buffer. */
  get size(): number {
    return this.buffer.length;
  }

  /** Maximum capacity of the buffer. */
  get capacity(): number {
    return this.maxSize;
  }

  private seq = 0;
  private sessionId = `session-${Date.now()}`;

  private persistToDb(msg: ChatMessage): void {
    if (!drizzleDb) return;
    const attempt = (retriesLeft: number): void => {
      drizzleDb.execute(sql`INSERT INTO "EpisodicMemory" ("sessionId", role, content, "sequenceOrder") VALUES (${this.sessionId}, ${msg.role}, ${msg.content}, ${++this.seq})`).catch((err: unknown) => {
        if (retriesLeft > 0) {
          logger.debug({ retriesLeft }, 'Retrying episodic memory persist');
          setTimeout(() => attempt(retriesLeft - 1), 500);
        } else {
          logger.warn({ err: String(err) }, 'Failed to persist episodic memory to PostgreSQL after retries');
        }
      });
    };
    attempt(2);
  }
}
