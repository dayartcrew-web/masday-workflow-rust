// ============================================================
// Provider routes — list providers, test connection
// ============================================================

import type { ServerResponse, IncomingMessage } from 'http';
import { sendJson } from '../utils';
import type { RouteDefinition } from '../utils';

export interface ProviderServiceProvider {
  listProviders(): Promise<unknown>;
  testProvider(name: string, input: { model?: string; prompt?: string }): Promise<unknown>;
  saveProvider(input: { providerName: string; providerType: string; baseUrl: string; apiKey: string; models: string[]; isDefault?: boolean }): Promise<unknown>;
  deleteProvider(providerName: string): Promise<unknown>;
  setDefaultProvider(providerName: string): Promise<unknown>;
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
    // POST /api/providers — Save/create provider config
    {
      method: 'POST',
      pattern: '/api/providers',
      authRequired: true,
      handler: async (_req: IncomingMessage, res: ServerResponse, _params, body?: Record<string, unknown>) => {
        const input = body!;
        const result = await provider.saveProvider({
          providerName: input.providerName as string,
          providerType: input.providerType as string,
          baseUrl: input.baseUrl as string,
          apiKey: input.apiKey as string,
          models: (input.models as string[]) || [],
          isDefault: input.isDefault as boolean | undefined,
        });
        sendJson(res, 200, result);
      },
    },
    // DELETE /api/providers/:name — Delete provider config
    {
      method: 'DELETE',
      pattern: '/api/providers/:name',
      authRequired: true,
      handler: async (_req: IncomingMessage, res: ServerResponse, params: Record<string, string>) => {
        const result = await provider.deleteProvider(params.name);
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
    // PATCH /api/providers/:name/default — Toggle default provider
    {
      method: 'PATCH',
      pattern: '/api/providers/:name/default',
      authRequired: true,
      handler: async (_req: IncomingMessage, res: ServerResponse, params: Record<string, string>) => {
        const result = await provider.setDefaultProvider(params.name);
        sendJson(res, 200, result);
      },
    },
  ];
}
