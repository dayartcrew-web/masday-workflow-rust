import { createLogger } from '@mcp-rebuild/core';

const logger = createLogger('memory:working');

export interface SessionState {
  sessionId: string;
  currentSkills: string[];
  currentWorkflow: string | null;
  activeGoal: string | null;
  customState: Record<string, unknown>;
  createdAt: number;
  updatedAt: number;
}

/**
 * WorkingMemory - in-process RAM keyed by session ID.
 * Provides fast, ephemeral storage for active session state.
 * Not persisted - lost on process restart.
 */
export class WorkingMemory {
  private sessions: Map<string, SessionState> = new Map();

  /** Create a new session with default state. */
  create(sessionId: string): SessionState {
    if (this.sessions.has(sessionId)) {
      logger.warn({ sessionId }, 'Session already exists, returning existing');
      return this.sessions.get(sessionId)!;
    }

    const now = Date.now();
    const state: SessionState = {
      sessionId,
      currentSkills: [],
      currentWorkflow: null,
      activeGoal: null,
      customState: {},
      createdAt: now,
      updatedAt: now,
    };
    this.sessions.set(sessionId, state);
    logger.debug({ sessionId }, 'Created new session');
    return { ...state };
  }

  /** Get session state by ID. Returns undefined if not found. */
  get(sessionId: string): SessionState | undefined {
    const state = this.sessions.get(sessionId);
    return state ? { ...state, customState: { ...state.customState } } : undefined;
  }

  /** Get or create session state. */
  getOrCreate(sessionId: string): SessionState {
    const existing = this.get(sessionId);
    if (existing) {
      return existing;
    }
    return this.create(sessionId);
  }

  /** Update session state with partial updates (immutable). */
  update(sessionId: string, updates: Partial<Omit<SessionState, 'sessionId' | 'createdAt'>>): SessionState | undefined {
    const current = this.sessions.get(sessionId);
    if (!current) {
      logger.warn({ sessionId }, 'Cannot update non-existent session');
      return undefined;
    }

    const updated: SessionState = {
      ...current,
      ...updates,
      sessionId: current.sessionId,
      createdAt: current.createdAt,
      customState: updates.customState
        ? { ...updates.customState }
        : { ...current.customState },
      updatedAt: Date.now(),
    };
    this.sessions.set(sessionId, updated);
    logger.debug({ sessionId }, 'Updated session');
    return { ...updated, customState: { ...updated.customState } };
  }

  /** Delete a session. Returns true if it existed. */
  delete(sessionId: string): boolean {
    const existed = this.sessions.delete(sessionId);
    if (existed) {
      logger.debug({ sessionId }, 'Deleted session');
    }
    return existed;
  }

  /** Format session state for LLM prompt injection. */
  toPromptString(sessionId: string): string {
    const state = this.sessions.get(sessionId);
    if (!state) {
      return '';
    }

    const lines: string[] = [`## Current Session (${sessionId})`];

    if (state.activeGoal) {
      lines.push(`**Active Goal:** ${state.activeGoal}`);
    }

    if (state.currentWorkflow) {
      lines.push(`**Current Workflow:** ${state.currentWorkflow}`);
    }

    if (state.currentSkills.length > 0) {
      lines.push(`**Active Skills:** ${state.currentSkills.join(', ')}`);
    }

    const customKeys = Object.keys(state.customState);
    if (customKeys.length > 0) {
      lines.push('**Additional Context:**');
      for (const key of customKeys) {
        lines.push(`- ${key}: ${JSON.stringify(state.customState[key])}`);
      }
    }

    return lines.join('\n');
  }

  /** Get count of active sessions. */
  get size(): number {
    return this.sessions.size;
  }

  /** Check if a session exists. */
  has(sessionId: string): boolean {
    return this.sessions.has(sessionId);
  }

  /** Clear all sessions. */
  clear(): void {
    this.sessions.clear();
    logger.debug('Cleared all sessions');
  }
}
