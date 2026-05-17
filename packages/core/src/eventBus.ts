import { EventEmitter } from "eventemitter3";
import { Event, EventType } from "./types.js";
import { createLogger } from "./logger.js";

const logger = createLogger("EventBus");

export class EventBus extends EventEmitter<EventType, Event> {
  private eventHistory: Event[] = [];

  emit(type: EventType, data: unknown): boolean {
    const event: Event = {
      type,
      timestamp: new Date(),
      data,
    };

    this.eventHistory.push(event);

    // Keep last 1000 events
    if (this.eventHistory.length > 1000) {
      this.eventHistory.shift();
    }

    logger.debug({ event }, `Event emitted: ${type}`);
    return super.emit(type, event);
  }

  on(type: EventType, listener: (event: Event) => void): this {
    return super.on(type, listener);
  }

  once(type: EventType, listener: (event: Event) => void): this {
    return super.once(type, listener);
  }

  off(type: EventType, listener: (event: Event) => void): this {
    return super.off(type, listener);
  }

  getHistory(limit?: number): Event[] {
    if (limit) {
      return this.eventHistory.slice(-limit);
    }
    return [...this.eventHistory];
  }

  getHistoryByType(type: EventType, limit?: number): Event[] {
    const filtered = this.eventHistory.filter((e) => e.type === type);
    return limit ? filtered.slice(-limit) : filtered;
  }

  clearHistory(): void {
    this.eventHistory = [];
  }
}
