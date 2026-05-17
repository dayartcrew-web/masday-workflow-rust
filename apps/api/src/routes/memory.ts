// ============================================================
// Memory routes — store, recall, search, CRUD
// ============================================================

import type { ServerResponse, IncomingMessage } from 'http';
import { sendJson, str, num } from '../utils';
import type { RouteDefinition } from '../utils';

/** Memory store interface for the API to depend on */
export interface MemoryServiceProvider {
  store(entry: { memoryType: string; summary: string; content: string; importance?: number; taskId?: string; workflowId?: string }): Promise<{ id: string }>;
  storeResearch(entry: { workflowId: string; query: string; findings: string; sources: string[] }): Promise<{ id: string }>;
  recallDocuments(workflowId: string, limit?: number): Promise<unknown[]>;
  recallRecent(workflowId: string, limit?: number): Promise<unknown[]>;
  recallByType(workflowId: string, type: string, limit?: number): Promise<unknown[]>;
  recallByTask(taskId: string, limit?: number): Promise<unknown[]>;
  update(id: string, updates: Record<string, unknown>): Promise<{ updated: boolean }>;
  delete(id: string): Promise<{ deleted: boolean }>;
}

export function createMemoryRoutes(provider: MemoryServiceProvider): RouteDefinition[] {
  return [
    // POST /api/memory — Store memory
    {
      method: 'POST',
      pattern: '/api/memory',
      authRequired: true,
      handler: async (_req: IncomingMessage, res: ServerResponse, _params, body?: Record<string, unknown>) => {
        const input = body!;
        const result = await provider.store({
          memoryType: input.memoryType as string,
          summary: input.summary as string,
          content: input.content as string,
          importance: input.importance as number | undefined,
          taskId: input.taskId as string | undefined,
          workflowId: input.workflowId as string | undefined,
        });
        sendJson(res, 201, result);
      },
    },
    // POST /api/memory/research — Store research
    {
      method: 'POST',
      pattern: '/api/memory/research',
      authRequired: true,
      handler: async (_req: IncomingMessage, res: ServerResponse, _params, body?: Record<string, unknown>) => {
        const input = body!;
        const result = await provider.storeResearch({
          workflowId: input.workflowId as string,
          query: input.query as string,
          findings: input.findings as string,
          sources: (input.sources as string[]) || [],
        });
        sendJson(res, 201, result);
      },
    },
    // GET /api/memory/:workflowId — Recall documents
    {
      method: 'GET',
      pattern: '/api/memory/:workflowId',
      authRequired: true,
      handler: async (_req: IncomingMessage, res: ServerResponse, params: Record<string, string>, _body?: Record<string, unknown>, query?: URLSearchParams) => {
        const limit = query?.get('limit') ? parseInt(query.get('limit')!) : undefined;
        const docs = await provider.recallDocuments(params.workflowId, limit);
        sendJson(res, 200, { documents: docs });
      },
    },
    // GET /api/memory/:workflowId/recent — Recent memories
    {
      method: 'GET',
      pattern: '/api/memory/:workflowId/recent',
      authRequired: true,
      handler: async (_req: IncomingMessage, res: ServerResponse, params: Record<string, string>, _body?: Record<string, unknown>, query?: URLSearchParams) => {
        const limit = query?.get('limit') ? parseInt(query.get('limit')!) : 10;
        const memories = await provider.recallRecent(params.workflowId, limit);
        sendJson(res, 200, { memories });
      },
    },
    // GET /api/memory/:workflowId/by-type/:type — By type
    {
      method: 'GET',
      pattern: '/api/memory/:workflowId/by-type/:type',
      authRequired: true,
      handler: async (_req: IncomingMessage, res: ServerResponse, params: Record<string, string>, _body?: Record<string, unknown>, query?: URLSearchParams) => {
        const limit = query?.get('limit') ? parseInt(query.get('limit')!) : undefined;
        const memories = await provider.recallByType(params.workflowId, params.type, limit);
        sendJson(res, 200, { memories });
      },
    },
    // GET /api/memory/task/:taskId — By task
    {
      method: 'GET',
      pattern: '/api/memory/task/:taskId',
      authRequired: true,
      handler: async (_req: IncomingMessage, res: ServerResponse, params: Record<string, string>, _body?: Record<string, unknown>, query?: URLSearchParams) => {
        const limit = query?.get('limit') ? parseInt(query.get('limit')!) : undefined;
        const memories = await provider.recallByTask(params.taskId, limit);
        sendJson(res, 200, { memories });
      },
    },
    // PUT /api/memory/:id — Update
    {
      method: 'PUT',
      pattern: '/api/memory/:id',
      authRequired: true,
      handler: async (_req: IncomingMessage, res: ServerResponse, params: Record<string, string>, body?: Record<string, unknown>) => {
        const input = body!;
        const result = await provider.update(params.id, input);
        sendJson(res, 200, result);
      },
    },
    // DELETE /api/memory/:id — Delete
    {
      method: 'DELETE',
      pattern: '/api/memory/:id',
      authRequired: true,
      handler: async (_req: IncomingMessage, res: ServerResponse, params: Record<string, string>) => {
        const result = await provider.delete(params.id);
        sendJson(res, 200, result);
      },
    },
  ];
}
