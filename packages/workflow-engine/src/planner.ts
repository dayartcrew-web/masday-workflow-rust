/**
 * Planner
 *
 * Generates task plans from requirements using rule-based analysis.
 * Ported from masday-workflow-reborn/packages/orchestrator/src/planner.ts
 * Adapted to use ISkillRegistry interface.
 */

import type { Task } from "@mcp-rebuild/core";
import type { ISkillRegistry } from "./skill-executor.js";
import { createLogger } from "@mcp-rebuild/core";

const logger = createLogger("Planner");

export interface PlanResult {
  tasks: Omit<Task, "id" | "createdAt" | "state">[];
  estimatedSteps: number;
}

export interface PlannerConfig {
  maxRetries?: number;
  taskTimeout?: number;
}

export class Planner {
  private skillRegistry: ISkillRegistry;
  private config: PlannerConfig;

  constructor(skillRegistry: ISkillRegistry, config: PlannerConfig = {}) {
    this.skillRegistry = skillRegistry;
    this.config = {
      maxRetries: 3,
      taskTimeout: 300000,
      ...config,
    };
  }

  async analyze(
    requirements: string,
    context: Record<string, unknown>,
  ): Promise<PlanResult> {
    logger.info(
      { requirements: requirements.substring(0, 100) },
      "Analyzing requirements...",
    );

    const availableSkills = this.skillRegistry.getAll();
    logger.info(`Available skills: ${availableSkills.length}`);

    const plan = await this.generatePlan(
      requirements,
      context,
      availableSkills,
    );

    logger.info(
      `Plan generated with ${plan.tasks.length} tasks, estimated ${plan.estimatedSteps} steps`,
    );

    return plan;
  }

  private async generatePlan(
    requirements: string,
    context: Record<string, unknown>,
    availableSkills: Array<{ name: string; description: string }>,
  ): Promise<PlanResult> {
    const tasks: Omit<Task, "id" | "createdAt" | "state">[] = [];

    const lowerReqs = requirements.toLowerCase();

    if (lowerReqs.includes("write") || lowerReqs.includes("create")) {
      tasks.push(...this.detectWriteTasks(requirements, context));
    }

    if (lowerReqs.includes("read")) {
      tasks.push(...this.detectReadTasks(requirements, context));
    }

    if (lowerReqs.includes("list")) {
      tasks.push(...this.detectListTasks(requirements, context));
    }

    if (tasks.length === 0) {
      tasks.push({
        name: "Analyze requirements",
        agent: "system",
        skill: "filesystem.stat",
        dependencies: [],
        input: { path: process.cwd() },
      });
    }

    return {
      tasks,
      estimatedSteps: tasks.length,
    };
  }

  private detectWriteTasks(
    requirements: string,
    context: Record<string, unknown>,
  ): Omit<Task, "id" | "createdAt" | "state">[] {
    const pathMatch = requirements.match(/(?:[A-Za-z]:)?[/\\][^\s"'`,;]+/);
    const contentMatch = requirements.match(
      /(?:content|text)[:\s]+["']?([^"'\n]+)["']?/i,
    );

    const path = pathMatch
      ? pathMatch[0]
      : (context.path as string) || "/tmp/output.txt";
    const content = contentMatch ? contentMatch[1].trim() : "Generated content";

    return [
      {
        name: "Write file",
        agent: "backend",
        skill: "filesystem.write",
        dependencies: [],
        input: { path, content },
      },
    ];
  }

  private detectReadTasks(
    requirements: string,
    context: Record<string, unknown>,
  ): Omit<Task, "id" | "createdAt" | "state">[] {
    const pathMatch = requirements.match(/(?:read|from)\s+([^\s]+)/i);
    const path = pathMatch ? pathMatch[1] : context.path || "/tmp/output.txt";

    return [
      {
        name: "Read file",
        agent: "backend",
        skill: "filesystem.read",
        dependencies: [],
        input: { path },
      },
    ];
  }

  private detectListTasks(
    requirements: string,
    context: Record<string, unknown>,
  ): Omit<Task, "id" | "createdAt" | "state">[] {
    const pathMatch = requirements.match(/(?:list|dir)\s+([^\s]+)/i);
    const path = pathMatch ? pathMatch[1] : context.path || "/tmp";
    const recursive = requirements.toLowerCase().includes("recursive");

    return [
      {
        name: "List directory",
        agent: "backend",
        skill: "filesystem.list",
        dependencies: [],
        input: { path, recursive },
      },
    ];
  }

  estimateTaskDuration(
    task: Omit<Task, "id" | "createdAt" | "state">,
  ): number {
    const skillDurations: Record<string, number> = {
      "filesystem.read": 1000,
      "filesystem.write": 2000,
      "filesystem.list": 1500,
      "filesystem.delete": 1000,
      "filesystem.stat": 500,
    };

    return skillDurations[task.skill] || 5000;
  }

  validatePlan(plan: PlanResult): { valid: boolean; errors: string[] } {
    const errors: string[] = [];

    const visited = new Set<number>();
    const recursionStack = new Set<number>();

    const hasCycle = (taskIndex: number): boolean => {
      visited.add(taskIndex);
      recursionStack.add(taskIndex);

      const task = plan.tasks[taskIndex];
      if (!task) return false;

      for (const depId of task.dependencies) {
        const depIndex = Number(depId);
        if (isNaN(depIndex) || depIndex < 0 || depIndex >= plan.tasks.length) {
          continue;
        }
        if (!visited.has(depIndex) && hasCycle(depIndex)) {
          return true;
        } else if (recursionStack.has(depIndex)) {
          return true;
        }
      }

      recursionStack.delete(taskIndex);
      return false;
    };

    for (let i = 0; i < plan.tasks.length; i++) {
      if (!visited.has(i) && hasCycle(i)) {
        errors.push(`Circular dependency detected involving task ${i}`);
        break;
      }
    }

    for (let i = 0; i < plan.tasks.length; i++) {
      const task = plan.tasks[i];
      for (const dep of task.dependencies) {
        const depIndex = Number(dep);
        if (
          isNaN(depIndex) ||
          depIndex < 0 ||
          depIndex >= plan.tasks.length
        ) {
          errors.push(
            `Task ${i} (${task.name}) has invalid dependency: ${dep}`,
          );
        }
      }
    }

    for (const task of plan.tasks) {
      if (!this.skillRegistry.has(task.skill)) {
        errors.push(`Unknown skill: ${task.skill}`);
      }
    }

    return {
      valid: errors.length === 0,
      errors,
    };
  }

  optimizePlan(plan: PlanResult): PlanResult {
    if (plan.tasks.length <= 1) {
      return plan;
    }

    const tasks = plan.tasks;
    const n = tasks.length;

    const inDegree = new Array(n).fill(0) as number[];
    const dependents = new Array(n).fill(null).map(() => [] as number[]);

    for (let i = 0; i < n; i++) {
      for (const dep of tasks[i].dependencies) {
        const depIdx = Number(dep);
        if (!isNaN(depIdx) && depIdx >= 0 && depIdx < n) {
          inDegree[i]++;
          dependents[depIdx].push(i);
        }
      }
    }

    const longestPath = new Array(n).fill(0) as number[];
    const computed = new Set<number>();

    const computeLongestPath = (idx: number): number => {
      if (computed.has(idx)) return longestPath[idx];
      computed.add(idx);

      const task = tasks[idx];
      let maxDepPath = 0;
      for (const dep of task.dependencies) {
        const depIdx = Number(dep);
        if (!isNaN(depIdx) && depIdx >= 0 && depIdx < n) {
          maxDepPath = Math.max(maxDepPath, computeLongestPath(depIdx));
        }
      }

      const duration = this.estimateTaskDuration(task);
      longestPath[idx] = maxDepPath + duration;
      return longestPath[idx];
    };

    for (let i = 0; i < n; i++) {
      computeLongestPath(i);
    }

    const sorted: number[] = [];
    const queue: number[] = [];

    for (let i = 0; i < n; i++) {
      if (inDegree[i] === 0) {
        queue.push(i);
      }
    }

    queue.sort((a, b) => longestPath[b] - longestPath[a]);

    while (queue.length > 0) {
      const current = queue.shift()!;
      sorted.push(current);

      const newReady: number[] = [];
      for (const dep of dependents[current]) {
        inDegree[dep]--;
        if (inDegree[dep] === 0) {
          newReady.push(dep);
        }
      }

      for (const task of newReady) {
        let inserted = false;
        for (let i = 0; i < queue.length; i++) {
          if (longestPath[task] > longestPath[queue[i]]) {
            queue.splice(i, 0, task);
            inserted = true;
            break;
          }
        }
        if (!inserted) {
          queue.push(task);
        }
      }
    }

    if (sorted.length !== n) {
      logger.warn(
        "Plan optimization could not fully sort tasks; possible cycle",
      );
      return plan;
    }

    const indexMap = new Map<number, number>();
    for (let newPos = 0; newPos < sorted.length; newPos++) {
      indexMap.set(sorted[newPos], newPos);
    }

    const optimizedTasks = sorted.map((originalIdx) => {
      const task = tasks[originalIdx];
      return {
        ...task,
        dependencies: task.dependencies.map((dep) => {
          const depIdx = Number(dep);
          const newIdx = indexMap.get(depIdx);
          return newIdx !== undefined ? String(newIdx) : dep;
        }),
      };
    });

    logger.info(
      {
        originalOrder: tasks.map((t) => t.name),
        optimizedOrder: optimizedTasks.map((t) => t.name),
        criticalPathLength: Math.max(...longestPath),
      },
      "Plan optimized for parallelism and critical path",
    );

    return {
      tasks: optimizedTasks,
      estimatedSteps: optimizedTasks.length,
    };
  }
}
