// ============================================================
// Monitoring routes — health, metrics, stats
// ============================================================

import type { ServerResponse, IncomingMessage } from 'http';
import { sendJson } from '../utils';
import type { RouteDefinition } from '../utils';

export interface MonitoringServiceProvider {
  getHealth(): Promise<unknown>;
  getMetrics(): unknown;
  getStats(): Promise<unknown>;
  getTokenUsage?(params: { groupBy?: string; from?: string; to?: string; route?: string; model?: string }): Promise<unknown>;
}

export function createMonitoringRoutes(provider: MonitoringServiceProvider): RouteDefinition[] {
  return [
    // GET /health — Top-level health check (no auth)
    {
      method: 'GET',
      pattern: '/health',
      handler: async (_req: IncomingMessage, res: ServerResponse) => {
        const result = await provider.getHealth();
        sendJson(res, 200, result);
      },
    },
    // GET /api/health — Health check
    {
      method: 'GET',
      pattern: '/api/health',
      handler: async (_req: IncomingMessage, res: ServerResponse) => {
        const result = await provider.getHealth();
        sendJson(res, 200, result);
      },
    },
    // GET /api/monitoring/health — Monitoring health alias
    {
      method: 'GET',
      pattern: '/api/monitoring/health',
      handler: async (_req: IncomingMessage, res: ServerResponse) => {
        const result = await provider.getHealth();
        sendJson(res, 200, result);
      },
    },
    // GET /api/metrics — Prometheus-style metrics
    {
      method: 'GET',
      pattern: '/api/metrics',
      authRequired: true,
      handler: async (_req: IncomingMessage, res: ServerResponse) => {
        const result = provider.getMetrics();
        sendJson(res, 200, result);
      },
    },
    // GET /api/monitoring/metrics — Monitoring metrics alias
    {
      method: 'GET',
      pattern: '/api/monitoring/metrics',
      authRequired: true,
      handler: async (_req: IncomingMessage, res: ServerResponse) => {
        const result = provider.getMetrics();
        sendJson(res, 200, result);
      },
    },
    // GET /api/stats — System stats
    {
      method: 'GET',
      pattern: '/api/stats',
      authRequired: true,
      handler: async (_req: IncomingMessage, res: ServerResponse) => {
        const result = await provider.getStats();
        sendJson(res, 200, result);
      },
    },
    // GET /api/monitoring/stats — Monitoring stats alias
    {
      method: 'GET',
      pattern: '/api/monitoring/stats',
      authRequired: true,
      handler: async (_req: IncomingMessage, res: ServerResponse) => {
        const result = await provider.getStats();
        sendJson(res, 200, result);
      },
    },
    // GET /api/token-usage — Token usage aggregation
    {
      method: 'GET',
      pattern: '/api/token-usage',
      authRequired: true,
      handler: async (req: IncomingMessage, res: ServerResponse) => {
        if (!provider.getTokenUsage) {
          sendJson(res, 501, { error: 'Token usage aggregation not available' });
          return;
        }
        const url = new URL(req.url || '', `http://localhost`);
        const result = await provider.getTokenUsage({
          groupBy: url.searchParams.get('groupBy') ?? undefined,
          from: url.searchParams.get('from') ?? undefined,
          to: url.searchParams.get('to') ?? undefined,
          route: url.searchParams.get('route') ?? undefined,
          model: url.searchParams.get('model') ?? undefined,
        });
        sendJson(res, 200, result);
      },
    },
  ];
}
