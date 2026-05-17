/**
 * Enhanced Workflow Engine
 *
 * Adds Planner, DAGExecutor, VERIFY, FIX retry to the base engine.
 * Ported from masday-workflow-reborn/packages/orchestrator/src/enhancedWorkflowEngine.ts
 * Adapted to use ISkillRegistry interface.
 */

import type { Workflow, WorkflowState, Task } from "@mcp-rebuild/core";
import { EventBus } from "@mcp-rebuild/core";
import { Planner, PlannerConfig } from "./planner.js";
import {
  DAGExecutor,
} from "./dagExecutor.js";
import type { DAGExecutorConfig } from "./dagExecutor.js";
import type { ISkillRegistry, SkillExecutor } from "./skill-executor.js";
import { createLogger } from "@mcp-rebuild/core";
import {
  BaseWorkflowEngine,
  BaseWorkflowEngineConfig,
} from "./baseWorkflowEngine.js";

const logger = createLogger("EnhancedWorkflowEngine");

export interface EnhancedWorkflowEngineConfig extends BaseWorkflowEngineConfig {
  planner?: PlannerConfig;
  executor?: DAGExecutorConfig & { _skillExecutor?: SkillExecutor };
  maxFixRetries?: number;
}

export class EnhancedWorkflowEngine extends BaseWorkflowEngine {
  protected planner: Planner;
  protected dagExecutor: DAGExecutor;
  private enhancedConfig: EnhancedWorkflowEngineConfig;

  constructor(
    skillRegistry: ISkillRegistry,
    eventBus: EventBus,
    config: EnhancedWorkflowEngineConfig = {},
  ) {
    super(skillRegistry, eventBus, config);
    this.planner = new Planner(skillRegistry, config.planner);
    const { _skillExecutor, ...dagConfig } = config.executor || {};
    this.dagExecutor = new DAGExecutor(
      skillRegistry,
      eventBus,
      dagConfig,
      _skillExecutor,
    );
    this.enhancedConfig = {
      maxFixRetries: 1,
      ...config,
    };
  }

  createWorkflow(
    name: string,
    description: string,
    metadata: Record<string, unknown> = {},
  ): Workflow {
    const workflow = this.createWorkflowObject(name, description, metadata);
    this.eventBus.emit("workflow.started", { workflow });
    return workflow;
  }

  async planAndExecute(
    name: string,
    requirements: string,
    context: Record<string, unknown> = {},
  ): Promise<Workflow> {
    logger.info(`Planning workflow: ${name}`);

    const workflow = this.createWorkflow(name, requirements, context);

    try {
      this.stateMachine.transition(workflow, "ANALYZE");
      this.stateMachine.transition(workflow, "PLAN");

      const plan = await this.planner.analyze(requirements, context);

      const validation = this.planner.validatePlan(plan);
      if (!validation.valid) {
        throw new Error(`Invalid plan: ${validation.errors.join(", ")}`);
      }

      const optimizedPlan = this.planner.optimizePlan(plan);

      const taskIds: string[] = [];
      for (const taskSpec of optimizedPlan.tasks) {
        const task = this.addTask(workflow.id, {
          name: taskSpec.name,
          agent: taskSpec.agent,
          skill: taskSpec.skill,
          dependencies: [],
          input: taskSpec.input,
        });
        taskIds.push(task.id);
      }

      for (let i = 0; i < optimizedPlan.tasks.length; i++) {
        const planDeps = optimizedPlan.tasks[i].dependencies;
        if (planDeps.length > 0) {
          workflow.tasks[i].dependencies = planDeps
            .map((depIndex) => {
              const index = Number(depIndex);
              return taskIds[index];
            })
            .filter((id): id is string => !!id);
        }
      }

      logger.info(`Created ${workflow.tasks.length} tasks from plan`);

      logger.info(`About to execute workflow ${workflow.id}`);
      const result = await this.executeWorkflow(workflow.id);
      logger.info(`Workflow executed successfully`);
      return result;
    } catch (error: unknown) {
      const errMsg =
        error instanceof Error ? error.message : String(error);
      logger.error({ error: errMsg }, `Error in planAndExecute:`);
      logger.error({ state: workflow.state }, `Current workflow state:`);

      try {
        this.stateMachine.transition(workflow, "FIX");
      } catch (transitionError: unknown) {
        const transErrMsg =
          transitionError instanceof Error
            ? transitionError.message
            : String(transitionError);
        logger.error(
          { error: transErrMsg },
          `Failed to transition to FIX:`,
        );
      }
      this.eventBus.emit("workflow.failed", {
        workflow,
        error: String(error),
      });

      logger.error(
        { error },
        `Workflow ${workflow.id} failed during planning`,
      );
      throw error;
    }
  }

  async executeWorkflow(id: string): Promise<Workflow> {
    const workflow = this.workflows.get(id);
    if (!workflow) {
      throw new Error(`Workflow ${id} not found`);
    }

    logger.info(`Starting workflow execution: ${id}`);

    if (workflow.tasks.length === 0) {
      this.stateMachine.transition(workflow, "DONE");
      this.persistWorkflow(workflow);
      this.eventBus.emit("workflow.completed", { workflow });
      logger.info(`Workflow ${id} completed (no tasks)`);
      return workflow;
    }

    try {
      if (workflow.state === "INIT") {
        this.stateMachine.transition(workflow, "ANALYZE");
        this.stateMachine.transition(workflow, "PLAN");
      }

      this.stateMachine.transition(workflow, "EXECUTE");

      this.workflows.set(id, workflow);
      this.persistWorkflow(workflow);

      const result = await this.dagExecutor.execute(workflow.tasks);

      logger.info(result, `Workflow execution completed`);

      this.stateMachine.transition(workflow, "VERIFY");
      this.persistWorkflow(workflow);
      await this.verifyWorkflow(workflow);

      this.stateMachine.transition(workflow, "DONE");
      this.persistWorkflow(workflow);
      this.eventBus.emit("workflow.completed", { workflow });

      logger.info(`Workflow completed: ${id}`);
      return workflow;
    } catch (error) {
      const fixResult = await this.attemptFix(workflow, error);
      if (fixResult === "recovered") {
        return workflow;
      }

      try {
        this.stateMachine.transition(workflow, "FAILED");
      } catch {
        this.stateMachine.transition(workflow, "FIX");
      }
      this.persistWorkflow(workflow);
      this.eventBus.emit("workflow.failed", {
        workflow,
        error: String(error),
      });

      logger.error({ error }, `Workflow failed: ${id}`);
      throw error;
    }
  }

  protected async verifyWorkflow(workflow: Workflow): Promise<void> {
    const failedTasks = workflow.tasks.filter((t) => t.state === "failed");
    if (failedTasks.length > 0) {
      const failedNames = failedTasks.map((t) => t.name).join(", ");
      throw new Error(
        `Verification failed: ${failedTasks.length} task(s) failed: ${failedNames}`,
      );
    }

    logger.info(
      `Workflow ${workflow.id} verification passed: all ${workflow.tasks.length} tasks completed`,
    );
  }

  protected async attemptFix(
    workflow: Workflow,
    error: unknown,
  ): Promise<"recovered" | "failed"> {
    const maxFixRetries = this.enhancedConfig.maxFixRetries ?? 1;
    let fixAttempts = 0;

    while (fixAttempts < maxFixRetries) {
      fixAttempts++;

      try {
        logger.info(
          `FIX attempt ${fixAttempts}/${maxFixRetries} for workflow ${workflow.id}`,
        );
        this.stateMachine.transition(workflow, "FIX");

        for (const task of workflow.tasks) {
          if (task.state === "failed") {
            task.state = "pending";
            task.error = undefined;
            task.completedAt = undefined;
            task.startedAt = undefined;
          }
        }

        this.eventBus.emit("workflow.fixing", {
          workflow,
          attempt: fixAttempts,
          error: String(error),
        });

        this.stateMachine.transition(workflow, "EXECUTE");
        const result = await this.dagExecutor.execute(workflow.tasks);

        logger.info(result, `FIX attempt ${fixAttempts} succeeded`);

        this.stateMachine.transition(workflow, "VERIFY");
        await this.verifyWorkflow(workflow);

        this.stateMachine.transition(workflow, "DONE");
        this.persistWorkflow(workflow);
        this.eventBus.emit("workflow.completed", { workflow });

        logger.info(
          `Workflow ${workflow.id} recovered after ${fixAttempts} fix attempt(s)`,
        );
        return "recovered";
      } catch (fixError) {
        logger.error(
          { error: String(fixError) },
          `FIX attempt ${fixAttempts} failed for workflow ${workflow.id}`,
        );
        error = fixError;
      }
    }

    return "failed";
  }

  getCriticalPath(id: string): Task[] {
    const workflow = this.workflows.get(id);
    if (!workflow) {
      throw new Error(`Workflow ${id} not found`);
    }

    return this.dagExecutor.getCriticalPath(workflow.tasks);
  }

  async cancel(id: string): Promise<void> {
    logger.info(`Cancelling workflow: ${id}`);
    await this.dagExecutor.cancel();

    const workflow = this.workflows.get(id);
    if (workflow) {
      workflow.state = "FIX";
      workflow.updatedAt = new Date();
    }
  }
}
