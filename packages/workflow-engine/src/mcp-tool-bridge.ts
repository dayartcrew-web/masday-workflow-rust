/**
 * MCP Tool Bridge
 *
 * Captures tool handler references during server.registerTool() calls
 * and routes programmatic execution through them.
 */

import { createLogger } from "@mcp-rebuild/core";

const logger = createLogger("MCPToolBridge");

export type ToolHandler = (input: unknown) => Promise<unknown>;

export class MCPToolBridge {
  private handlers = new Map<string, ToolHandler>();

  register(name: string, handler: ToolHandler): void {
    this.handlers.set(name, handler);
    logger.info({ tool: name }, "Registered tool handler");
  }

  async execute(name: string, input: unknown): Promise<unknown> {
    const handler = this.handlers.get(name);
    if (!handler) {
      throw new Error(`Tool not found: ${name}`);
    }
    return handler(input);
  }

  has(name: string): boolean {
    return this.handlers.has(name);
  }

  getHandler(name: string): ToolHandler | undefined {
    return this.handlers.get(name);
  }

  getAll(): string[] {
    return Array.from(this.handlers.keys());
  }

  get size(): number {
    return this.handlers.size;
  }
}
