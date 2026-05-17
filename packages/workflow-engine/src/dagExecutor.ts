/**
 * DAG Executor
 *
 * Executes tasks with dependency resolution and parallel execution.
 * Ported from masday-workflow-reborn/packages/orchestrator/src/dagExecutor.ts
 * Adapted to use ISkillRegistry interface.
 */

import type { Task } from "@mcp-rebuild/core";
import { EventBus } from "@mcp-rebuild/core";
import { TaskManager } from "@mcp-rebuild/core";
import type { ISkillRegistry, SkillExecutor } from "./skill-executor.js";
import { createLogger } from "@mcp-rebuild/core";

const logger = createLogger("DAGExecutor");

export interface DAGExecutorConfig {
  maxRetries?: number;
  retryDelay?: number;
  maxConcurrentTasks?: number;
  taskTimeout?: number;
}

export interface ExecutionResult {
  successful: number;
  failed: number;
  skipped: number;
  duration: number;
}

export class DAGExecutor {
  private skillRegistry: ISkillRegistry;
  private eventBus: EventBus;
  private taskManager: TaskManager;
  private config: DAGExecutorConfig;
  private runningTasks: Map<string, Promise<void>> = new Map();
  private abortControllers: Map<string, AbortController> = new Map();
  private skillExecutor?: SkillExecutor;
  private cancelled = false;
  private cancelReject?: (reason: Error) => void;

  constructor(
    skillRegistry: ISkillRegistry,
    eventBus: EventBus,
    config: DAGExecutorConfig = {},
    skillExecutor?: SkillExecutor,
  ) {
    this.skillRegistry = skillRegistry;
    this.eventBus = eventBus;
    this.taskManager = new TaskManager();
    this.config = {
      maxRetries: 3,
      retryDelay: 1000,
      maxConcurrentTasks: 10,
      taskTimeout: 300000,
      ...config,
    };
    this.skillExecutor = skillExecutor;
  }

  async execute(tasks: Task[]): Promise<ExecutionResult> {
    const startTime = Date.now();

    logger.info(`Starting DAG execution with ${tasks.length} tasks`);

    this.cancelled = false;
    this.cancelReject = undefined;

    let completedTasks = 0;
    let failedTasks = 0;
    let skippedTasks = 0;

    const taskMap = new Map<string, Task>();
    for (const task of tasks) {
      if (!task.id) {
        task.id = `task_${Date.now()}_${Math.random().toString(36).substring(7)}`;
      }
      taskMap.set(task.id, task);
    }

    try {
      while (completedTasks + failedTasks < taskMap.size) {
        if (this.cancelled) {
          logger.info("DAG execution cancelled, stopping");
          break;
        }

        const readyTasks = Array.from(taskMap.values()).filter((task) => {
          if (task.state !== "pending") return false;

          const allDepsDone = task.dependencies.every((depId) => {
            const depTask = taskMap.get(depId);
            return depTask && depTask.state === "done";
          });

          return allDepsDone && !this.runningTasks.has(task.id);
        });

        if (readyTasks.length === 0) {
          const pendingTasks = Array.from(taskMap.values()).filter(
            (t) => t.state === "pending",
          );
          if (pendingTasks.length > 0) {
            throw new Error(
              "DAG executor stuck: no ready tasks but workflow not complete",
            );
          }
          break;
        }

        const tasksToExecute = readyTasks.slice(
          0,
          this.config.maxConcurrentTasks!,
        );
        for (const task of tasksToExecute) {
          const executionPromise = this.executeTask(task, taskMap);
          this.runningTasks.set(task.id, executionPromise);
        }

        const cancelPromise = new Promise<never>((_, reject) => {
          this.cancelReject = reject;
        });

        try {
          await Promise.race([
            Promise.race(this.runningTasks.values()),
            cancelPromise,
          ]);
        } catch (error) {
          if (this.cancelled) {
            break;
          }
          throw error;
        } finally {
          this.cancelReject = undefined;
        }

        for (const [taskId, promise] of this.runningTasks.entries()) {
          const task = taskMap.get(taskId);
          if (
            task &&
            (task.state === "done" || task.state === "failed")
          ) {
            this.runningTasks.delete(taskId);

            if (task.state === "done") {
              completedTasks++;
            } else {
              failedTasks++;
            }
          }
        }
      }
    } catch (error) {
      if (this.cancelled) {
        logger.info("DAG execution interrupted by cancellation");
      } else {
        throw error;
      }
    }

    const duration = Date.now() - startTime;

    if (this.cancelled) {
      const pendingCount = Array.from(taskMap.values()).filter(
        (t) => t.state === "pending",
      ).length;
      logger.info(
        {
          successful: completedTasks,
          failed: failedTasks,
          skipped: skippedTasks + pendingCount,
          duration,
        },
        "DAG execution cancelled",
      );
      return {
        successful: completedTasks,
        failed: failedTasks,
        skipped: skippedTasks + pendingCount,
        duration,
      };
    }

    if (failedTasks > 0) {
      const failedTaskDetails = Array.from(taskMap.values())
        .filter((t) => t.state === "failed")
        .map((t) => ({ id: t.id, name: t.name, error: t.error }));

      throw new Error(
        `DAG execution failed: ${failedTasks} task(s) failed\n` +
          JSON.stringify(failedTaskDetails, null, 2),
      );
    }

    logger.info(
      {
        successful: completedTasks,
        failed: failedTasks,
        skipped: skippedTasks,
        duration,
      },
      `DAG execution completed`,
    );

    return {
      successful: completedTasks,
      failed: failedTasks,
      skipped: skippedTasks,
      duration,
    };
  }

  private async executeTask(
    task: Task,
    taskMap: Map<string, Task>,
  ): Promise<void> {
    const taskId = task.id;
    logger.info(`Executing task: ${taskId} (${task.name})`);

    let attempt = 0;
    let lastError: string | undefined;

    while (attempt <= this.config.maxRetries!) {
      attempt++;
      const abortController = new AbortController();
      this.abortControllers.set(taskId, abortController);
      const timeoutId = setTimeout(
        () => abortController.abort(),
        this.config.taskTimeout!,
      );

      try {
        task.state = "running";
        task.startedAt = new Date();
        this.eventBus.emit("task.started", { taskId, attempt });

        logger.debug(`Task ${taskId} attempt ${attempt}`);

        const enrichedInput = this.buildTaskInput(task, taskMap);

        const executeSkill = this.skillExecutor
          ? () => this.skillExecutor!(task.skill, enrichedInput, task)
          : () => this.skillRegistry.execute(task.skill, enrichedInput);

        const result = await Promise.race([
          executeSkill(),
          new Promise<never>((_, reject) =>
            abortController.signal.addEventListener("abort", () =>
              reject(
                new Error(
                  `Task ${taskId} timed out after ${this.config.taskTimeout}ms`,
                ),
              ),
            ),
          ),
        ]);

        task.output = result;
        task.state = "done";
        task.completedAt = new Date();
        this.eventBus.emit("task.completed", { taskId, attempt, result });

        logger.info(`Task ${taskId} completed successfully`);
        clearTimeout(timeoutId);
        this.abortControllers.delete(taskId);
        return;
      } catch (error) {
        clearTimeout(timeoutId);
        lastError = String(error);
        logger.error(
          { error: lastError },
          `Task ${taskId} attempt ${attempt} failed`,
        );

        if (attempt < this.config.maxRetries!) {
          const delay = this.calculateRetryDelay(attempt);
          logger.info(`Retrying task ${taskId} in ${delay}ms...`);
          await this.sleep(delay);
        }
      }
    }

    task.error = lastError!;
    task.state = "failed";
    task.completedAt = new Date();
    this.abortControllers.delete(taskId);
    this.eventBus.emit("task.failed", {
      taskId,
      error: lastError,
      attempts: attempt,
    });

    logger.error(`Task ${taskId} failed after ${attempt} attempts`);
    throw new Error(`Task ${task.name} failed: ${lastError}`);
  }

  private calculateRetryDelay(attempt: number): number {
    const baseDelay = this.config.retryDelay || 1000;
    return baseDelay * Math.pow(2, attempt - 1);
  }

  private sleep(ms: number): Promise<void> {
    return new Promise((resolve) => setTimeout(resolve, ms));
  }

  private buildTaskInput(
    task: Task,
    taskMap: Map<string, Task>,
  ): unknown {
    const depOutputs: Record<string, unknown> = {};

    if (task.dependencies.length > 0) {
      for (const depId of task.dependencies) {
        const depTask = taskMap.get(depId);
        if (depTask && depTask.output !== undefined) {
          depOutputs[depId] = depTask.output;
        }
      }
    }

    if (Object.keys(depOutputs).length === 0) {
      return task.input;
    }

    return {
      ...(typeof task.input === "object" && task.input !== null
        ? (task.input as Record<string, unknown>)
        : { input: task.input }),
      dependencyOutputs: depOutputs,
    };
  }

  async cancel(): Promise<void> {
    logger.info("Cancelling all running tasks...");

    this.cancelled = true;

    for (const [taskId, controller] of this.abortControllers.entries()) {
      controller.abort();
      this.taskManager.setError(taskId, "Task cancelled");
      this.taskManager.updateState(taskId, "failed");
    }

    this.abortControllers.clear();
    this.runningTasks.clear();

    if (this.cancelReject) {
      this.cancelReject(new Error("DAG execution cancelled"));
    }
  }

  getStatus(tasks: Task[]): {
    total: number;
    pending: number;
    running: number;
    done: number;
    failed: number;
  } {
    const status = {
      total: tasks.length,
      pending: 0,
      running: 0,
      done: 0,
      failed: 0,
    };

    for (const task of tasks) {
      status[task.state]++;
    }

    return status;
  }

  getCriticalPath(tasks: Task[]): Task[] {
    const taskMap = new Map(tasks.map((t) => [t.id, t]));

    const longestPath = new Map<string, Task[]>();

    const calculatePath = (taskId: string): Task[] => {
      if (longestPath.has(taskId)) {
        return longestPath.get(taskId)!;
      }

      const task = taskMap.get(taskId);
      if (!task) return [];

      if (task.dependencies.length === 0) {
        return [task];
      }

      let maxPath: Task[] = [];
      for (const depId of task.dependencies) {
        const depPath = calculatePath(depId);
        if (depPath.length > maxPath.length) {
          maxPath = depPath;
        }
      }

      const path = [...maxPath, task];
      longestPath.set(taskId, path);
      return path;
    };

    let criticalPath: Task[] = [];
    for (const task of tasks) {
      const path = calculatePath(task.id);
      if (path.length > criticalPath.length) {
        criticalPath = path;
      }
    }

    return criticalPath;
  }
}
