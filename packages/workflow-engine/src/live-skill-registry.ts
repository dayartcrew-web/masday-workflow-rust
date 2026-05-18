/**
 * Live Skill Registry
 *
 * Implements ISkillRegistry by bridging skill definitions (SKILL.md)
 * with MCP tool handlers. Replaces the stub that threw "Not available".
 *
 * Input format for execute():
 * - Direct tool: { tool: "workflow.create", args: { name: "..." } }
 * - Skill-level: { action: "run", ...params } (maps to first allowed tool)
 */

import type { ISkillRegistry } from "./skill-executor.js";
import type { MCPToolBridge } from "./mcp-tool-bridge.js";
import type { SkillLoader, SkillDefinition } from "./skill-loader.js";
import { createLogger } from "@mcp-rebuild/core";

const logger = createLogger("LiveSkillRegistry");

export interface SkillExecuteInput {
  tool?: string;
  args?: Record<string, unknown>;
  action?: string;
  [key: string]: unknown;
}

export class LiveSkillRegistry implements ISkillRegistry {
  constructor(
    private loader: SkillLoader,
    private bridge: MCPToolBridge,
  ) {}

  async execute(skill: string, input: unknown): Promise<unknown> {
    const def = this.loader.load(skill);
    if (!def) {
      throw new Error(`Skill not found: ${skill}`);
    }

    const inp = (input ?? {}) as SkillExecuteInput;

    if (inp.tool) {
      return this.executeToolCall(def, inp.tool, inp.args ?? {});
    }

    return this.executeDefault(def, inp);
  }

  has(skill: string): boolean {
    return this.loader.has(skill);
  }

  getAll(): Array<{ name: string; description: string }> {
    return this.loader.getAll();
  }

  private async executeToolCall(
    def: SkillDefinition,
    toolName: string,
    args: Record<string, unknown>,
  ): Promise<unknown> {
    if (!def.allowedTools.includes(toolName)) {
      throw new Error(
        `Tool "${toolName}" not allowed for skill "${def.name}". ` +
          `Allowed: ${def.allowedTools.join(", ")}`,
      );
    }

    const handler = this.bridge.getHandler(toolName);
    if (!handler) {
      throw new Error(`Tool handler not registered: ${toolName}`);
    }

    logger.info({ skill: def.name, tool: toolName }, "Executing tool via skill");
    return handler(args);
  }

  private async executeDefault(
    def: SkillDefinition,
    input: SkillExecuteInput,
  ): Promise<unknown> {
    const { action, ...params } = input;
    const toolName = action ?? def.allowedTools[0];

    if (!toolName) {
      throw new Error(
        `No allowed tools defined for skill "${def.name}" and no action specified`,
      );
    }

    return this.executeToolCall(def, toolName, params);
  }
}
