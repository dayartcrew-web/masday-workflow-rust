/**
 * Workflow Engine (basic tier)
 *
 * Linear state machine with sequential task execution.
 * Ported from masday-workflow-reborn/packages/orchestrator/src/workflowEngine.ts
 */

import type { Workflow, WorkflowState } from "@mcp-rebuild/core";
import { EventBus } from "@mcp-rebuild/core";
import type { ISkillRegistry } from "./skill-executor.js";
import { createLogger } from "@mcp-rebuild/core";
import {
  BaseWorkflowEngine,
  BaseWorkflowEngineConfig,
} from "./baseWorkflowEngine.js";

const logger = createLogger("WorkflowEngine");

export class WorkflowEngine extends BaseWorkflowEngine {
  constructor(
    skillRegistry: ISkillRegistry,
    eventBus: EventBus,
    config: BaseWorkflowEngineConfig = {},
  ) {
    super(skillRegistry, eventBus, config);
  }

  createWorkflow(
    name: string,
    description: string,
    metadata: Record<string, unknown> = {},
    projectPath?: string,
  ): Workflow {
    const workflow = this.createWorkflowObject(name, description, metadata, projectPath);
    this.eventBus.emit("workflow.started", { workflow });
    return workflow;
  }

  async executeWorkflow(id: string): Promise<Workflow> {
    const workflow = this.workflows.get(id);
    if (!workflow) {
      throw new Error(`Workflow ${id} not found`);
    }

    logger.info(`Starting workflow execution: ${id}`);

    try {
      for (const state of [
        "ANALYZE",
        "PLAN",
        "EXECUTE",
        "VERIFY",
      ] as WorkflowState[]) {
        this.stateMachine.transition(workflow, state);

        if (state === "EXECUTE") {
          await this.executeTasks(workflow);
        }
      }

      this.stateMachine.transition(workflow, "DONE");
      this.eventBus.emit("workflow.completed", { workflow });

      logger.info(`Workflow completed: ${id}`);
      return workflow;
    } catch (error) {
      this.stateMachine.transition(workflow, "FIX");
      this.eventBus.emit("workflow.failed", {
        workflow,
        error: String(error),
      });

      logger.error({ error }, `Workflow failed: ${id}`);
      throw error;
    }
  }

  private async executeTasks(workflow: Workflow): Promise<void> {
    const tasks = workflow.tasks;
    let completedTasks = 0;
    let failedTasks = 0;

    while (completedTasks + failedTasks < tasks.length) {
      const readyTasks = this.taskManager.getReadyTasks(tasks);

      if (
        readyTasks.length === 0 &&
        completedTasks + failedTasks < tasks.length
      ) {
        throw new Error(
          "Workflow stuck: circular dependency or unresolvable task state",
        );
      }

      for (const task of readyTasks) {
        this.taskManager.updateState(task.id, "running");
        this.eventBus.emit("task.started", { task, workflowId: workflow.id });

        try {
          const result = await this.skillRegistry.execute(
            task.skill,
            task.input,
          );
          this.taskManager.setOutput(task.id, result);
          this.taskManager.updateState(task.id, "done");
          this.eventBus.emit("task.completed", {
            task,
            workflowId: workflow.id,
          });
          completedTasks++;

          logger.info(`Task completed: ${task.id}`);
        } catch (error) {
          this.taskManager.setError(task.id, String(error));
          this.taskManager.updateState(task.id, "failed");
          this.eventBus.emit("task.failed", {
            task,
            workflowId: workflow.id,
            error: String(error),
          });
          failedTasks++;

          logger.error({ error }, `Task failed: ${task.id}`);
          throw error;
        }
      }
    }
  }
}
