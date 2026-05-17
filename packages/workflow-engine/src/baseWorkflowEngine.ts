/**
 * Base Workflow Engine
 *
 * Shared logic for all engine tiers. Provides workflow storage,
 * retrieval, status reporting, and lifecycle management.
 *
 * Ported from masday-workflow-reborn/packages/orchestrator/src/baseWorkflowEngine.ts
 * Adapted to use ISkillRegistry interface instead of SkillRegistry from mcp-server.
 */

import type {
  Workflow,
  WorkflowState,
  Task,
} from "@mcp-rebuild/core";
import { EventBus, TaskManager } from "@mcp-rebuild/core";
import { StateMachine } from "./stateMachine.js";
import type { ISkillRegistry } from "./skill-executor.js";
import { createLogger } from "@mcp-rebuild/core";
import { randomUUID } from "crypto";
import type { IWorkflowStore } from "@mcp-rebuild/store";

const logger = createLogger("BaseWorkflowEngine");

export interface WorkflowStatusResult {
  state: WorkflowState;
  tasks: {
    total: number;
    pending: number;
    running: number;
    done: number;
    failed: number;
  };
}

export interface BaseWorkflowEngineConfig {
  /** Optional persistence store. When provided, workflows are persisted on every mutation. */
  store?: IWorkflowStore;
}

/**
 * BaseWorkflowEngine - Shared logic for all engine tiers
 *
 * Provides:
 * - Workflow storage and retrieval (with optional persistence)
 * - Deterministic ID generation using UUID
 * - Status reporting
 * - Common lifecycle management
 */
export abstract class BaseWorkflowEngine {
  protected workflows: Map<string, Workflow> = new Map();
  protected taskManager: TaskManager;
  protected stateMachine: StateMachine;
  protected skillRegistry: ISkillRegistry;
  protected eventBus: EventBus;
  protected _pausedWorkflows: Set<string> = new Set();
  protected store?: IWorkflowStore;

  constructor(
    skillRegistry: ISkillRegistry,
    eventBus: EventBus,
    config: BaseWorkflowEngineConfig = {},
  ) {
    this.skillRegistry = skillRegistry;
    this.eventBus = eventBus;
    this.taskManager = new TaskManager();
    this.stateMachine = new StateMachine(eventBus);
    this.store = config.store;
  }

  protected generateWorkflowId(): string {
    return `workflow_${randomUUID()}`;
  }

  protected createWorkflowObject(
    name: string,
    description: string,
    metadata: Record<string, unknown> = {},
  ): Workflow {
    const id = this.generateWorkflowId();
    const workflow: Workflow = {
      id,
      name,
      description,
      state: "INIT",
      tasks: [],
      metadata,
      traceId: `trace_${randomUUID()}`,
      createdAt: new Date(),
      updatedAt: new Date(),
    };

    this.workflows.set(workflow.id, workflow);
    this.persistWorkflow(workflow);
    logger.info(
      { workflowId: id, traceId: workflow.traceId },
      `Created workflow: ${workflow.id} - ${name}`,
    );
    return workflow;
  }

  addTask(
    workflowId: string,
    task: Omit<Task, "id" | "createdAt" | "state">,
  ): Task {
    const workflow = this.workflows.get(workflowId);
    if (!workflow) {
      throw new Error(`Workflow ${workflowId} not found`);
    }

    const newTask = this.taskManager.create(task);
    workflow.tasks.push(newTask);
    workflow.updatedAt = new Date();
    this.persistWorkflow(workflow);

    logger.info(`Added task ${newTask.id} to workflow ${workflowId}`);
    return newTask;
  }

  getWorkflow(id: string): Workflow | undefined {
    const cached = this.workflows.get(id);
    if (cached) return cached;

    if (this.store) {
      const loaded = this.store.load(id);
      if (loaded) {
        this.workflows.set(id, loaded);
        return loaded;
      }
    }

    return undefined;
  }

  listWorkflows(): Workflow[] {
    if (this.store) {
      const stored = this.store.loadAll();
      for (const wf of stored) {
        if (!this.workflows.has(wf.id)) {
          this.workflows.set(wf.id, wf);
        }
      }
    }
    return Array.from(this.workflows.values());
  }

  restoreWorkflow(workflow: Workflow): void {
    if (workflow.state === "INIT" && workflow.tasks.length === 0) {
      workflow.state = "DONE";
      workflow.updatedAt = new Date();
      logger.info(
        `Auto-resolved orphaned INIT workflow ${workflow.id} (${workflow.name}) to DONE`,
      );
    }
    this.workflows.set(workflow.id, workflow);
  }

  restoreWorkflows(workflows: Workflow[]): number {
    let orphans = 0;
    for (const wf of workflows) {
      if (wf.state === "INIT" && wf.tasks.length === 0) orphans++;
      this.restoreWorkflow(wf);
    }
    if (orphans > 0) {
      logger.info(
        `Restored ${workflows.length} workflows (${orphans} orphans resolved)`,
      );
    } else {
      logger.info(`Restored ${workflows.length} workflows`);
    }
    return orphans;
  }

  getStatus(id: string): WorkflowStatusResult {
    const workflow = this.workflows.get(id);
    if (!workflow) {
      throw new Error(`Workflow ${id} not found`);
    }

    const tasks = workflow.tasks;
    const tasksByState = tasks.reduce(
      (acc, task) => {
        acc[task.state]++;
        return acc;
      },
      { pending: 0, running: 0, done: 0, failed: 0 },
    );

    return {
      state: workflow.state,
      tasks: {
        total: tasks.length,
        ...tasksByState,
      },
    };
  }

  deleteWorkflow(id: string): boolean {
    const workflow = this.workflows.get(id);
    if (!workflow) {
      return false;
    }

    if (workflow.state === "EXECUTE" || workflow.state === "PAUSED") {
      throw new Error(
        `Cannot delete workflow ${id} in ${workflow.state} state. Cancel it first.`,
      );
    }

    this.workflows.delete(id);
    if (this.store) {
      this.store.delete(id);
    }
    this.eventBus.emit("workflow.deleted", { workflowId: id });
    logger.info(`Deleted workflow: ${id}`);
    return true;
  }

  pauseWorkflow(id: string): void {
    const workflow = this.workflows.get(id);
    if (!workflow) {
      throw new Error(`Workflow ${id} not found`);
    }

    this.stateMachine.transition(workflow, "PAUSED");
    this._pausedWorkflows.add(id);
    this.persistWorkflow(workflow);
    this.eventBus.emit("workflow.paused", {
      workflowId: id,
      pausedTasks: workflow.tasks.filter((t) => t.state === "pending").length,
    });
    logger.info(`Paused workflow: ${id}`);
  }

  resumeWorkflow(id: string): void {
    const workflow = this.workflows.get(id);
    if (!workflow) {
      throw new Error(`Workflow ${id} not found`);
    }

    this.stateMachine.transition(workflow, "EXECUTE");
    this._pausedWorkflows.delete(id);
    this.persistWorkflow(workflow);
    this.eventBus.emit("workflow.resumed", { workflowId: id });
    logger.info(`Resumed workflow: ${id}`);
  }

  isPaused(id: string): boolean {
    return this._pausedWorkflows.has(id);
  }

  protected persistWorkflow(workflow: Workflow): void {
    if (this.store) {
      try {
        this.store.save(workflow);
        logger.debug(
          { workflowId: workflow.id },
          `Workflow persisted to store`,
        );
      } catch (error) {
        logger.error(
          { error: String(error), workflowId: workflow.id },
          `Failed to persist workflow to store`,
        );
        this.eventBus.emit("store.error", {
          error: String(error),
          workflowId: workflow.id,
        });
      }
    }
  }
}
