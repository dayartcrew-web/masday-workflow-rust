/**
 * Agent Coordinator and Skill Router
 *
 * Ported from @masday-workflow-reborn/agents and inlined into this package
 * so the OrchestratingEngine has no external agent dependency.
 *
 * These are self-contained implementations that depend only on @mcp-rebuild/core.
 */

import type { Task, EventType, Event } from "@mcp-rebuild/core";
import { EventBus } from "@mcp-rebuild/core";
import { createLogger } from "@mcp-rebuild/core";
import type { ISkillRegistry } from "./skill-executor.js";

const logger = createLogger("AgentWorker");

// ─── Agent Types ───

export type AgentStatus = "idle" | "busy" | "error" | "stopped";

export type AgentType = "backend" | "frontend" | "qa" | "general-purpose";

export interface AgentWorkerConfig {
  name: string;
  type: AgentType;
  model?: "sonnet" | "opus" | "haiku" | "inherit";
  tools: string[];
  description: string;
  maxTurns?: number;
  memory?: "user" | "project" | "local" | "none";
  hooks?: Record<string, unknown>;
  skillRegistry?: { execute: (name: string, input: unknown) => Promise<unknown> };
}

export interface AgentMessage {
  type: "task" | "status" | "result" | "error" | "coordinator";
  taskId?: string;
  agentId: string;
  content?: string;
  data?: unknown;
  timestamp: Date;
}

// ─── AgentWorker ───

export class AgentWorker {
  public status: AgentStatus = "idle";
  public currentTask: Task | null = null;
  public tasksCompleted = 0;
  public tasksFailed = 0;
  public startedAt?: Date;
  public lastActivity?: Date;

  constructor(
    public readonly id: string,
    public readonly config: AgentWorkerConfig,
  ) {
    this.startedAt = config.maxTurns ? new Date() : undefined;
    this.lastActivity = new Date();
  }

  async executeTask(task: Task): Promise<unknown> {
    this.currentTask = task;
    this.status = "busy";
    this.lastActivity = new Date();

    logger.info(`Agent ${this.id} executing task: ${task.name}`);

    try {
      const result = await this.config.skillRegistry!.execute(
        task.skill,
        task.input,
      );

      this.currentTask = null;
      this.tasksCompleted++;
      this.status = "idle";
      this.lastActivity = new Date();

      return result;
    } catch (error) {
      this.currentTask = null;
      this.tasksFailed++;
      this.status = "error";
      this.lastActivity = new Date();

      logger.error({ error }, `Agent ${this.id} task failed: ${task.name}`);

      throw error;
    }
  }

  getStatus(): AgentStatus {
    return this.status;
  }

  getStats() {
    return {
      id: this.id,
      status: this.status,
      config: this.config,
      currentTask: this.currentTask?.id || null,
      tasksCompleted: this.tasksCompleted,
      tasksFailed: this.tasksFailed,
      startedAt: this.startedAt,
      lastActivity: this.lastActivity,
      uptime: this.startedAt ? Date.now() - this.startedAt.getTime() : 0,
    };
  }

  stop(): void {
    this.status = "stopped";
    this.currentTask = null;
    this.lastActivity = new Date();
    logger.info(`Agent ${this.id} stopped`);
  }

  isIdle(): boolean {
    return this.status === "idle";
  }

  isBusy(): boolean {
    return this.status === "busy";
  }

  isError(): boolean {
    return this.status === "error";
  }
}

// ─── AgentCoordinator ───

/** Skill-to-agent default routing table */
const SKILL_TO_AGENT: Record<string, string> = {
  "filesystem.read": "backend",
  "filesystem.write": "backend",
  "filesystem.list": "backend",
  "filesystem.delete": "backend",
  "filesystem.stat": "backend",
  "git.status": "backend",
  "git.diff": "backend",
  "git.commit": "backend",
  "code.generate": "general-purpose",
  "code.modify": "frontend",
  "code.search": "general-purpose",
  "code.lint": "general-purpose",
  "code.format": "general-purpose",
  "tests.run": "qa",
  "tests.fix": "qa",
  "tests.coverage": "qa",
  "workflow.plan": "general-purpose",
  "workflow.execute": "general-purpose",
  "workflow.optimize": "general-purpose",
};

export class AgentCoordinator {
  public workers: Map<string, AgentWorker> = new Map();
  public activeWorkflows: Map<string, string[]> = new Map();
  private eventBus: EventBus;
  private pendingDispatches: Map<string, Task[]> = new Map();

  constructor(eventBus: EventBus) {
    this.eventBus = eventBus;
  }

  addWorker(workerConfig: AgentWorkerConfig & { id?: string }): void {
    const workerId = workerConfig.id || workerConfig.name;
    const worker = new AgentWorker(workerId, workerConfig);

    this.workers.set(workerId, worker);

    logger.info(`Added worker: ${workerId} (${workerConfig.type})`);
    this.eventBus.emit("agent.started" as EventType, {
      agentId: workerId,
      agentType: workerConfig.type,
    });

    this.flushPendingTasks(workerId);
  }

  getWorker(agentId: string): AgentWorker | undefined {
    return this.workers.get(agentId);
  }

  removeWorker(agentId: string): void {
    const worker = this.workers.get(agentId);
    if (!worker) return;

    worker.stop();
    this.workers.delete(agentId);
    logger.info(`Removed worker: ${agentId}`);
  }

  stopAll(): void {
    for (const [, worker] of this.workers) {
      worker.stop();
    }
    this.workers.clear();
    this.pendingDispatches.clear();
    logger.info("All workers stopped");
  }

  routeTask(agent: AgentWorker, task: Task): string | null {
    const bestAgentId = this.findBestAgent(task);

    if (bestAgentId) {
      const bestWorker = this.workers.get(bestAgentId);
      if (bestWorker && bestWorker.isIdle()) {
        return bestAgentId;
      }
      return null;
    }

    if (agent.isIdle()) {
      return agent.id;
    }

    if (agent.isError()) {
      logger.warn(
        `Agent ${agent.id} is in error state (failed: ${agent.getStats().tasksFailed}), removing`,
      );
      this.removeWorker(agent.id);
      return null;
    }

    logger.warn(`Agent ${agent.id} is busy, cannot route task: ${task.name}`);
    return null;
  }

  findBestAgent(task: Task): string | null {
    const targetAgentType = SKILL_TO_AGENT[task.skill];
    if (!targetAgentType) return null;

    for (const [workerId, worker] of this.workers) {
      if (worker.config.type === targetAgentType && worker.isIdle()) {
        return workerId;
      }
    }
    return null;
  }

  async dispatchTask(agentId: string, task: Task): Promise<unknown> {
    const worker = this.workers.get(agentId);
    if (!worker) {
      throw new Error(`Agent ${agentId} not found`);
    }

    if (worker.isBusy()) {
      throw new Error(
        `Agent ${agentId} is busy, cannot dispatch task ${task.name}`,
      );
    }

    try {
      const result = await worker.executeTask(task);

      this.eventBus.emit("agent.task.completed" as EventType, {
        agentId: worker.id,
        taskId: task.id,
        taskName: task.name,
        result: "completed",
      });

      return result;
    } catch (error: unknown) {
      this.eventBus.emit("agent.task.failed" as EventType, {
        agentId: worker.id,
        taskId: task.id,
        taskName: task.name,
        error: String(error),
      });
      throw error;
    }
  }

  async routeAndDispatch(task: Task): Promise<unknown> {
    const bestAgentId = this.findBestAgent(task);

    if (bestAgentId) {
      return this.dispatchTask(bestAgentId, task);
    }

    const agentType = SKILL_TO_AGENT[task.skill] || "general-purpose";
    const pending = this.pendingDispatches.get(agentType) || [];
    pending.push(task);
    this.pendingDispatches.set(agentType, pending);

    logger.info(`Queued task ${task.name} for agent type ${agentType}`);
    return undefined;
  }

  broadcast(message: AgentMessage): void {
    this.eventBus.emit("agent.message" as EventType, message);
    logger.info({ message }, `Broadcasting message:`);
  }

  getStats() {
    const workerStats = Array.from(this.workers.values()).map((worker) =>
      worker.getStats(),
    );

    return {
      workers: workerStats,
      totalWorkers: this.workers.size,
      activeWorkers: workerStats.filter(
        (w) => w.status !== "idle",
      ).length,
      totalTasksCompleted: workerStats.reduce(
        (sum, w) => sum + w.tasksCompleted,
        0,
      ),
      totalTasksFailed: workerStats.reduce(
        (sum, w) => sum + w.tasksFailed,
        0,
      ),
      pendingTasks: Array.from(this.pendingDispatches.values()).reduce(
        (sum, q) => sum + q.length,
        0,
      ),
    };
  }

  private flushPendingTasks(workerId: string): void {
    const worker = this.workers.get(workerId);
    if (!worker) return;

    const pending = this.pendingDispatches.get(worker.config.type);
    if (!pending || pending.length === 0) return;

    const task = pending.shift()!;
    if (pending.length === 0) {
      this.pendingDispatches.delete(worker.config.type);
    }

    this.dispatchTask(workerId, task).catch((error: unknown) => {
      logger.error(
        { error: String(error) },
        `Failed to dispatch queued task ${task.name}`,
      );
    });
  }
}

// ─── SkillRouter ───

const AGENT_CAPABILITIES: Record<string, string[]> = {
  backend: [
    "filesystem.read",
    "filesystem.write",
    "filesystem.list",
    "filesystem.delete",
    "filesystem.stat",
    "git.status",
    "git.diff",
    "git.commit",
  ],
  frontend: ["code.modify", "code.generate", "code.search"],
  qa: ["tests.run", "tests.fix", "tests.coverage"],
  "general-purpose": [
    "workflow.plan",
    "workflow.execute",
    "workflow.optimize",
    "code.generate",
    "code.modify",
    "tests.run",
  ],
};

export class SkillRouter {
  private preferredAgents: Map<string, string> = new Map();
  private eventBus: EventBus;

  constructor(eventBus: EventBus) {
    this.eventBus = eventBus;
  }

  registerSkill(skill: string, preferredAgent: string): void {
    this.preferredAgents.set(skill, preferredAgent);
    logger.info(`Registered skill ${skill} -> agent ${preferredAgent}`);
  }

  routeTask(task: Task): string | null {
    const preferredAgent = this.preferredAgents.get(task.skill);
    if (preferredAgent) {
      const capabilities = AGENT_CAPABILITIES[preferredAgent];
      if (capabilities && capabilities.includes(task.skill)) {
        logger.debug(
          `Routed task ${task.name} (${task.skill}) -> agent ${preferredAgent} (preferred)`,
        );
        return preferredAgent;
      }
      logger.warn(
        `Preferred agent ${preferredAgent} does not support skill ${task.skill}, falling back to default mapping`,
      );
    }

    const defaultAgent = SKILL_TO_AGENT[task.skill];
    if (defaultAgent) {
      logger.debug(
        `Routed task ${task.name} (${task.skill}) -> agent ${defaultAgent} (default)`,
      );
      return defaultAgent;
    }

    logger.warn(
      `No agent mapping found for skill ${task.skill}, falling back to general-purpose`,
    );
    return "general-purpose";
  }

  getSkillsForAgent(agentType: string): string[] {
    return AGENT_CAPABILITIES[agentType] || [];
  }

  getRoutingStats() {
    return {
      registeredSkills: this.preferredAgents.size,
      agentCapabilities: AGENT_CAPABILITIES,
    };
  }

  getAgentCapabilities(agentType: string): string[] {
    return AGENT_CAPABILITIES[agentType] || [];
  }
}
