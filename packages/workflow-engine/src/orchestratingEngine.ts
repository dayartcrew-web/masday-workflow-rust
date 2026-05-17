/**
 * Orchestrating Engine (full tier)
 *
 * Multi-agent orchestration with AgentCoordinator and SkillRouter.
 * Ported from masday-workflow-reborn/packages/orchestrator/src/orchestratingEngine.ts
 * Adapted to use inlined AgentCoordinator/SkillRouter and ISkillRegistry.
 */

import type { Workflow, Task } from "@mcp-rebuild/core";
import { EventBus } from "@mcp-rebuild/core";
import { TaskQueue } from "./taskQueue.js";
import type { ISkillRegistry, SkillExecutor } from "./skill-executor.js";
import {
  AgentCoordinator,
  SkillRouter,
} from "./agents.js";
import { createLogger } from "@mcp-rebuild/core";
import {
  EnhancedWorkflowEngine,
  EnhancedWorkflowEngineConfig,
} from "./enhancedWorkflowEngine.js";

const logger = createLogger("OrchestratingEngine");

export interface OrchestratingEngineConfig
  extends EnhancedWorkflowEngineConfig {
  coordinator?: boolean;
  enableSkillRouting?: boolean;
  workerConfigs?: Array<{
    type: string;
    tools: string[];
    description: string;
  }>;
}

const DEFAULT_WORKER_CONFIGS = [
  {
    type: "backend",
    tools: ["filesystem.*", "git.*"],
    description: "Backend agent for filesystem and git operations",
  },
  {
    type: "frontend",
    tools: ["code.modify", "code.generate"],
    description: "Frontend agent for code generation and modification",
  },
  {
    type: "qa",
    tools: ["tests.*"],
    description: "QA agent for testing operations",
  },
  {
    type: "general-purpose",
    tools: ["workflow.*", "code.*"],
    description:
      "General purpose agent for workflow and code operations",
  },
];

export class OrchestratingEngine extends EnhancedWorkflowEngine {
  private taskQueue: TaskQueue;
  private agentCoordinator?: AgentCoordinator;
  private skillRouter?: SkillRouter;
  private taskAgentMap: Map<string, string> = new Map();
  private engineConfig: OrchestratingEngineConfig;

  constructor(
    skillRegistry: ISkillRegistry,
    eventBus: EventBus,
    config: OrchestratingEngineConfig = {},
  ) {
    let agentSkillExecutor: SkillExecutor | undefined;
    let tempCoordinator: AgentCoordinator | undefined;
    let tempSkillRouter: SkillRouter | undefined;

    if (config.coordinator) {
      tempCoordinator = new AgentCoordinator(eventBus);
    }

    if (config.enableSkillRouting) {
      tempSkillRouter = new SkillRouter(eventBus);
    }

    if (tempCoordinator) {
      agentSkillExecutor = async (
        skill: string,
        input: unknown,
        task: Task,
      ) => {
        const agentId = this.taskAgentMap.get(task.name);
        if (agentId) {
          try {
            const result = await tempCoordinator!.dispatchTask(
              agentId,
              task,
            );
            return result;
          } catch (dispatchError: unknown) {
            logger.warn(
              {
                error: String(dispatchError),
                agentId,
                taskName: task.name,
              },
              `Agent dispatch failed, falling back to direct skill execution`,
            );
          }
        }
        return skillRegistry.execute(skill, input);
      };
    }

    super(skillRegistry, eventBus, {
      ...config,
      executor: {
        ...config.executor,
        _skillExecutor: agentSkillExecutor,
      } as never,
    });

    this.taskQueue = new TaskQueue();
    this.engineConfig = config;
    this.agentCoordinator = tempCoordinator;
    this.skillRouter = tempSkillRouter;

    if (this.agentCoordinator) {
      const workerConfigs = config.workerConfigs || DEFAULT_WORKER_CONFIGS;
      for (const wc of workerConfigs) {
        this.agentCoordinator.addWorker({
          name: `${wc.type}-default`,
          type: wc.type as never,
          tools: wc.tools,
          description: wc.description,
          skillRegistry: {
            execute: (skillName: string, input: unknown) =>
              skillRegistry.execute(skillName, input),
          },
        });
      }
      logger.info(`Registered ${workerConfigs.length} default workers`);
    }
  }

  async planAndExecute(
    name: string,
    requirements: string,
    context: Record<string, unknown> = {},
  ): Promise<Workflow> {
    logger.info(`Orchestrating planAndExecute: ${name}`);

    const workflow = this.createWorkflowObject(name, requirements, context);
    this.eventBus.emit("workflow.started", { workflow });

    try {
      this.stateMachine.transition(workflow, "ANALYZE");
      this.stateMachine.transition(workflow, "PLAN");

      const plan = await this.planner.analyze(requirements, context);

      const validation = this.planner.validatePlan(plan);
      if (!validation.valid) {
        throw new Error(`Invalid plan: ${validation.errors.join(", ")}`);
      }

      const optimizedPlan = this.planner.optimizePlan(plan);

      if (this.engineConfig.enableSkillRouting && this.skillRouter) {
        for (const task of optimizedPlan.tasks) {
          const recommendedAgent = this.skillRouter.routeTask(
            task as Task,
          );
          if (recommendedAgent) {
            this.taskAgentMap.set(
              task.name,
              `${recommendedAgent}-default`,
            );
          }
        }
      }

      const taskIds: string[] = [];
      for (const taskSpec of optimizedPlan.tasks) {
        const createdTask = this.addTask(workflow.id, taskSpec);
        taskIds.push(createdTask.id);

        const priority = this.getTaskPriority(createdTask);
        this.taskQueue.enqueue(createdTask, priority);
      }

      for (let i = 0; i < optimizedPlan.tasks.length; i++) {
        const planDeps = optimizedPlan.tasks[i].dependencies;
        if (planDeps.length > 0) {
          workflow.tasks[i].dependencies = planDeps
            .map((depIndex) => taskIds[Number(depIndex)])
            .filter((id): id is string => !!id);
        }
      }

      logger.info(`Created ${workflow.tasks.length} tasks from plan`);
      logger.info(
        {
          tasks: workflow.tasks.map((t) => ({
            name: t.name,
            agent: this.taskAgentMap.get(t.name) || "default",
            skill: t.skill,
          })),
        },
        `Tasks routed to agents:`,
      );
    } catch (error: unknown) {
      logger.error(
        { error: String(error) },
        `PlanAndExecute failed`,
      );
      throw error;
    }

    this.workflows.set(workflow.id, workflow);

    return await this.executeWorkflow(workflow.id);
  }

  getCoordinatorStats() {
    if (!this.agentCoordinator) {
      return { coordinatorEnabled: false };
    }

    return this.agentCoordinator.getStats();
  }

  getRoutingStats() {
    if (!this.skillRouter) {
      return { routingEnabled: false };
    }

    return this.skillRouter.getRoutingStats();
  }

  async cancel(id: string): Promise<void> {
    const workflow = this.workflows.get(id);
    if (!workflow) {
      throw new Error(`Workflow ${id} not found`);
    }

    logger.info(`Cancelling workflow: ${id}`);

    this.dagExecutor.cancel();
    this.taskQueue.clear();

    this.stateMachine.transition(workflow, "FIX");
  }

  getAllWorkflows(): Workflow[] {
    return this.listWorkflows();
  }

  private getTaskPriority(task: Task): number {
    const agentType = this.taskAgentMap.get(task.name);
    switch (agentType) {
      case "backend":
        return 8;
      case "frontend":
        return 7;
      case "qa":
        return 6;
      default:
        return 5;
    }
  }
}
