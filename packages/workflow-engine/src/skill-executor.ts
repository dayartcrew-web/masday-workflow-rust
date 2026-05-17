/**
 * Skill Executor Interface
 *
 * Abstraction over skill execution, replacing the direct dependency on
 * SkillRegistry from @masday-workflow-reborn/mcp-server.
 *
 * Consumers provide their own implementation via constructor injection.
 */

import type { Task } from "@mcp-rebuild/core";

/**
 * Minimal skill registry interface needed by the workflow engine.
 * Implement this in your application layer to connect to the actual
 * skill registry or MCP server.
 */
export interface ISkillRegistry {
  /** Execute a skill by name with the given input. */
  execute(skill: string, input: unknown): Promise<unknown>;

  /** Check whether a skill is registered. */
  has(skill: string): boolean;

  /** Get all registered skills. */
  getAll(): Array<{ name: string; description: string }>;
}

/**
 * Optional skill executor override for DAGExecutor.
 * When provided, the DAGExecutor calls this instead of ISkillRegistry.execute.
 * This allows OrchestratingEngine to route tasks through AgentCoordinator.
 */
export type SkillExecutor = (
  skill: string,
  input: unknown,
  task: Task,
) => Promise<unknown>;
