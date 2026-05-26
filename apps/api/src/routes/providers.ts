// ============================================================
// Provider routes — list providers, test connection
// ============================================================

import type { ServerResponse, IncomingMessage } from 'http';
import { sendJson } from '../utils';
import type { RouteDefinition } from '../utils';

export interface ProviderServiceProvider {
  listProviders(): Promise<unknown>;
  testProvider(name: string, input: { model?: string; prompt?: string }): Promise<unknown>;
}

export function createProviderRoutes(provider: ProviderServiceProvider): RouteDefinition[] {
  return [
    // GET /api/providers — List available providers
    {
      method: 'GET',
      pattern: '/api/providers',
      authRequired: true,
      handler: async (_req: IncomingMessage, res: ServerResponse) => {
        const result = await provider.listProviders();
        sendJson(res, 200, result);
      },
    },
    // POST /api/providers/:name/test — Test provider
    {
      method: 'POST',
      pattern: '/api/providers/:name/test',
      authRequired: true,
      handler: async (_req: IncomingMessage, res: ServerResponse, params: Record<string, string>, body?: Record<string, unknown>) => {
        const input = body || {};
        const result = await provider.testProvider(params.name, {
          model: input.model as string | undefined,
          prompt: input.prompt as string | undefined,
        });
        sendJson(res, 200, result);
      },
    },
  ];
}
