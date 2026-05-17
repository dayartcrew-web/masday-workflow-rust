/**
 * Task Queue - Priority-based task queue
 *
 * Ported from masday-workflow-reborn/packages/orchestrator/src/taskQueue.ts
 */

import type { Task } from "@mcp-rebuild/core";
import { createLogger } from "@mcp-rebuild/core";

const logger = createLogger("TaskQueue");

export interface QueueItem {
  task: Task;
  priority: number;
  addedAt: Date;
}

export interface TaskQueueConfig {
  maxSize?: number;
  defaultPriority?: number;
}

export class TaskQueue {
  private queue: QueueItem[] = [];
  private config: TaskQueueConfig;

  constructor(config: TaskQueueConfig = {}) {
    this.config = {
      maxSize: 1000,
      defaultPriority: 5,
      ...config,
    };
  }

  enqueue(task: Task, priority?: number): void {
    if (this.queue.length >= this.config.maxSize!) {
      throw new Error("Queue is full");
    }

    const item: QueueItem = {
      task,
      priority: priority ?? this.config.defaultPriority!,
      addedAt: new Date(),
    };

    let inserted = false;
    for (let i = 0; i < this.queue.length; i++) {
      if (item.priority > this.queue[i].priority) {
        this.queue.splice(i, 0, item);
        inserted = true;
        break;
      }
    }

    if (!inserted) {
      this.queue.push(item);
    }

    logger.debug(`Task ${task.id} enqueued with priority ${item.priority}`);
  }

  dequeue(): Task | null {
    if (this.queue.length === 0) {
      return null;
    }

    const item = this.queue.shift()!;
    logger.debug(`Task ${item.task.id} dequeued`);
    return item.task;
  }

  peek(): Task | null {
    if (this.queue.length === 0) {
      return null;
    }
    return this.queue[0].task;
  }

  size(): number {
    return this.queue.length;
  }

  isEmpty(): boolean {
    return this.queue.length === 0;
  }

  isFull(): boolean {
    return this.queue.length >= this.config.maxSize!;
  }

  clear(): void {
    const count = this.queue.length;
    this.queue = [];
    logger.info(`Queue cleared, removed ${count} tasks`);
  }

  getAll(): Task[] {
    return this.queue.map((item) => item.task);
  }

  getStats(): {
    size: number;
    maxSize: number;
    isEmpty: boolean;
    isFull: boolean;
    avgPriority: number;
    minPriority: number;
    maxPriority: number;
  } {
    if (this.queue.length === 0) {
      return {
        size: 0,
        maxSize: this.config.maxSize!,
        isEmpty: true,
        isFull: false,
        avgPriority: 0,
        minPriority: 0,
        maxPriority: 0,
      };
    }

    const priorities = this.queue.map((item) => item.priority);
    const sum = priorities.reduce((a, b) => a + b, 0);

    return {
      size: this.queue.length,
      maxSize: this.config.maxSize!,
      isEmpty: false,
      isFull: this.queue.length >= this.config.maxSize!,
      avgPriority: sum / this.queue.length,
      minPriority: Math.min(...priorities),
      maxPriority: Math.max(...priorities),
    };
  }

  remove(taskId: string): boolean {
    const index = this.queue.findIndex((item) => item.task.id === taskId);
    if (index === -1) {
      return false;
    }

    this.queue.splice(index, 1);
    logger.debug(`Task ${taskId} removed from queue`);
    return true;
  }

  updatePriority(taskId: string, newPriority: number): boolean {
    const index = this.queue.findIndex((item) => item.task.id === taskId);
    if (index === -1) {
      return false;
    }

    const item = this.queue.splice(index, 1)[0];
    item.priority = newPriority;

    let inserted = false;
    for (let i = 0; i < this.queue.length; i++) {
      if (item.priority > this.queue[i].priority) {
        this.queue.splice(i, 0, item);
        inserted = true;
        break;
      }
    }

    if (!inserted) {
      this.queue.push(item);
    }

    logger.debug(`Task ${taskId} priority updated to ${newPriority}`);
    return true;
  }
}
