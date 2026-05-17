// ============================================================
// Monitoring routes — health, metrics, stats
// ============================================================

import type { ServerResponse, IncomingMessage } from 'http';
import { sendJson } from '../utils';
import type { RouteDefinition } from '../utils';

export interface MonitoringServiceProvider {
  getHealth(): Promise<unknown>;
  getMetrics(): unknown;
  getStats(): unknown;
}

export function createMonitoringRoutes(provider: MonitoringServiceProvider): RouteDefinition[] {
  return [
    // GET /api/health — Health check
    {
      method: 'GET',
      pattern: '/api/health',
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
    // GET /api/stats — System stats
    {
      method: 'GET',
      pattern: '/api/stats',
      authRequired: true,
      handler: async (_req: IncomingMessage, res: ServerResponse) => {
        const result = provider.getStats();
        sendJson(res, 200, result);
      },
    },
  ];
}
