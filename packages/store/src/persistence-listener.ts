import type {
  EventBus,
  Event,
  EventType,
  Workflow,
  Task,
} from "@mcp-rebuild/core";
import type { IWorkflowStore, ITaskResultStore } from "./types.js";
import { createLogger } from "@mcp-rebuild/core";

const logger = createLogger("PersistenceListener");

interface WorkflowEventData {
  workflow?: {
    id: string;
    state: string;
    tasks: unknown[];
    [key: string]: unknown;
  };
}

interface TaskEventData {
  task?: { id: string; state: string; [key: string]: unknown };
  workflowId?: string;
}

export class PersistenceListener {
  private eventBus: EventBus;
  private workflowStore: IWorkflowStore;
  private taskResultStore: ITaskResultStore;
  private started = false;
  private handlerEntries: [EventType, (event: Event) => void][];

  constructor(
    eventBus: EventBus,
    workflowStore: IWorkflowStore,
    taskResultStore: ITaskResultStore,
  ) {
    this.eventBus = eventBus;
    this.workflowStore = workflowStore;
    this.taskResultStore = taskResultStore;

    this.handlerEntries = [
      ["workflow.started", (e) => this.onWorkflowEvent(e)],
      ["workflow.completed", (e) => this.onWorkflowEvent(e)],
      ["workflow.failed", (e) => this.onWorkflowEvent(e)],
      ["task.started", (e) => this.onTaskEvent(e)],
      ["task.completed", (e) => this.onTaskEvent(e)],
      ["task.failed", (e) => this.onTaskEvent(e)],
    ];
  }

  start(): void {
    if (this.started) return;
    this.started = true;

    for (const [type, handler] of this.handlerEntries) {
      this.eventBus.on(type, handler);
    }

    this.eventBus.emit("store.connected", {});
    logger.info("Persistence listener started");
  }

  stop(): void {
    if (!this.started) return;
    this.started = false;

    for (const [type, handler] of this.handlerEntries) {
      this.eventBus.off(type, handler);
    }

    logger.info("Persistence listener stopped");
  }

  private onWorkflowEvent(event: Event): void {
    try {
      const data = event.data as WorkflowEventData;
      if (data?.workflow) {
        this.workflowStore.save(data.workflow as unknown as Workflow);
      }
    } catch (error) {
      logger.error({ error: String(error) }, "Failed to persist workflow");
      this.eventBus.emit("store.error", { error: String(error) });
    }
  }

  private onTaskEvent(event: Event): void {
    try {
      const data = event.data as TaskEventData;
      if (data?.task && data?.workflowId) {
        this.taskResultStore.saveTask(
          data.workflowId,
          data.task as unknown as Task,
        );
      }
    } catch (error) {
      logger.error({ error: String(error) }, "Failed to persist task");
      this.eventBus.emit("store.error", { error: String(error) });
    }
  }
}
