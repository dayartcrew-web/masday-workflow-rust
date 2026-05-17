// ============================================================
// Chat routes — LLM chat completion, ReAct agent execution
// ============================================================

import type { ServerResponse, IncomingMessage } from 'http';
import { sendJson } from '../utils';
import type { RouteDefinition } from '../utils';

export interface ChatServiceProvider {
  complete(input: { message: string; sessionId?: string; model?: string; temperature?: number }): Promise<unknown>;
  react(input: { goal: string; maxIterations?: number; sessionId?: string }): Promise<unknown>;
}

export function createChatRoutes(provider: ChatServiceProvider): RouteDefinition[] {
  return [
    // POST /api/chat — Chat completion via LLM
    {
      method: 'POST',
      pattern: '/api/chat',
      authRequired: true,
      handler: async (_req: IncomingMessage, res: ServerResponse, _params, body?: Record<string, unknown>) => {
        const input = body!;
        const result = await provider.complete({
          message: input.message as string,
          sessionId: input.sessionId as string | undefined,
          model: input.model as string | undefined,
          temperature: input.temperature as number | undefined,
        });
        sendJson(res, 200, result);
      },
    },
    // POST /api/chat/react — ReAct agent execution
    {
      method: 'POST',
      pattern: '/api/chat/react',
      authRequired: true,
      handler: async (_req: IncomingMessage, res: ServerResponse, _params, body?: Record<string, unknown>) => {
        const input = body!;
        const result = await provider.react({
          goal: input.goal as string,
          maxIterations: input.maxIterations as number | undefined,
          sessionId: input.sessionId as string | undefined,
        });
        sendJson(res, 200, result);
      },
    },
  ];
}
