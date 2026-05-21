// ============================================================
// Search routes — hybrid context pack, fingerprint, code search
// ============================================================

import type { ServerResponse, IncomingMessage } from 'http';
import { sendJson } from '../utils';
import type { RouteDefinition } from '../utils';

export interface SearchServiceProvider {
  hybridContextPack(input: { workflowId: string; planId: string; taskId: string; cwd?: string }): Promise<unknown>;
  contextFingerprint(input: { workflowId: string; planId: string; taskId: string }): Promise<unknown>;
  codeSearch(input: { query: string; glob?: string; type?: string; limit?: number }): Promise<unknown>;
}

export function createSearchRoutes(provider: SearchServiceProvider): RouteDefinition[] {
  return [
    // POST /api/search/hybrid — Hybrid context pack
    {
      method: 'POST',
      pattern: '/api/search/hybrid',
      authRequired: true,
      handler: async (_req: IncomingMessage, res: ServerResponse, _params, body?: Record<string, unknown>) => {
        const input = body!;
        const result = await provider.hybridContextPack({
          workflowId: input.workflowId as string,
          planId: input.planId as string,
          taskId: input.taskId as string,
          cwd: input.cwd as string | undefined,
        });
        sendJson(res, 200, result);
      },
    },
    // POST /api/search/context-pack — Alias for hybrid (e2e compat)
    {
      method: 'POST',
      pattern: '/api/search/context-pack',
      authRequired: true,
      handler: async (_req: IncomingMessage, res: ServerResponse, _params, body?: Record<string, unknown>) => {
        const input = body!;
        const result = await provider.hybridContextPack({
          workflowId: input.workflowId as string,
          planId: input.planId as string,
          taskId: input.taskId as string,
        });
        sendJson(res, 200, result);
      },
    },
    // POST /api/search/fingerprint — Context fingerprint
    {
      method: 'POST',
      pattern: '/api/search/fingerprint',
      authRequired: true,
      handler: async (_req: IncomingMessage, res: ServerResponse, _params, body?: Record<string, unknown>) => {
        const input = body!;
        const result = await provider.contextFingerprint({
          workflowId: input.workflowId as string,
          planId: input.planId as string,
          taskId: input.taskId as string,
        });
        sendJson(res, 200, result);
      },
    },
    // POST /api/search/code — Code search
    {
      method: 'POST',
      pattern: '/api/search/code',
      authRequired: true,
      handler: async (_req: IncomingMessage, res: ServerResponse, _params, body?: Record<string, unknown>) => {
        const input = body!;
        const result = await provider.codeSearch({
          query: input.query as string,
          glob: input.glob as string | undefined,
          type: input.type as string | undefined,
          limit: input.limit as number | undefined,
        });
        sendJson(res, 200, result);
      },
    },
  ];
}
