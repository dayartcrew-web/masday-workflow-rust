// ============================================================
// HTTP API Server — main entry point
// Node.js native http server with router, auth, rate limiting
// ============================================================

import { createServer, IncomingMessage, ServerResponse } from 'http';
import { EventBus } from '@mcp-rebuild/core';
import type { OrchestratingEngine } from '@mcp-rebuild/workflow-engine';
import { RateLimiter } from './rate-limit';
import { authenticateRequest } from './auth/jwt';
import type { AuthUser } from './auth/jwt';
import { WebSocketAPIServer } from './websocket/server';
import {
  sendJson, readBody, parseJsonBody, getClientIp,
  matchRoute, APIError,
  type RouteDefinition,
} from './utils';
import {
  authRoutes,
  createWorkflowRoutes,
  createMemoryRoutes, createSearchRoutes,
  createPolicyRoutes, createCapabilityRoutes,
  createChatRoutes, createProviderRoutes,
  createMonitoringRoutes,
} from './routes';
import type {
  MemoryServiceProvider, SearchServiceProvider,
  PolicyServiceProvider, CapabilityServiceProvider,
  ChatServiceProvider, ProviderServiceProvider,
  MonitoringServiceProvider,
} from './routes';

export interface APIServerConfig {
  httpPort: number;
  wsPort: number;
  cors: boolean;
  maxBodySize: number;
  rateLimitWindow: number;
  rateLimitMax: number;
}

interface ServerDeps {
  eventBus: EventBus;
  engine: OrchestratingEngine;
  memoryProvider: MemoryServiceProvider;
  searchProvider: SearchServiceProvider;
  policyProvider: PolicyServiceProvider;
  capabilityProvider: CapabilityServiceProvider;
  chatProvider: ChatServiceProvider;
  providerService: ProviderServiceProvider;
  monitoringProvider: MonitoringServiceProvider;
}

interface RequestLog {
  method: string;
  path: string;
  status: number;
  latencyMs: number;
  error?: string;
}

export class APIServer {
  private config: APIServerConfig;
  private deps: ServerDeps;
  private rateLimiter: RateLimiter;
  private wsServer: WebSocketAPIServer;
  private httpServer: ReturnType<typeof createServer> | null = null;
  private routes: RouteDefinition[];
  private requestCount = 0;
  private errorCount = 0;
  private totalLatencyMs = 0;
  private startTime = Date.now();

  constructor(deps: ServerDeps, config?: Partial<APIServerConfig>) {
    this.config = {
      httpPort: 3000,
      wsPort: 3001,
      cors: true,
      maxBodySize: 1_000_000,
      rateLimitWindow: 60_000,
      rateLimitMax: 100,
      ...config,
    };
    this.deps = deps;
    this.rateLimiter = new RateLimiter({
      windowMs: this.config.rateLimitWindow,
      max: this.config.rateLimitMax,
    });
    this.wsServer = new WebSocketAPIServer({ port: this.config.wsPort });
    this.routes = this.buildRoutes();
  }

  /** Build all route definitions */
  private buildRoutes(): RouteDefinition[] {
    return [
      ...authRoutes,
      ...createWorkflowRoutes(this.deps.engine),
      ...createMemoryRoutes(this.deps.memoryProvider),
      ...createSearchRoutes(this.deps.searchProvider),
      ...createPolicyRoutes(this.deps.policyProvider),
      ...createCapabilityRoutes(this.deps.capabilityProvider),
      ...createChatRoutes(this.deps.chatProvider),
      ...createProviderRoutes(this.deps.providerService),
      ...createMonitoringRoutes(this.deps.monitoringProvider),
    ];
  }

  /** Get all registered routes */
  getRoutes(): ReadonlyArray<Readonly<RouteDefinition>> {
    return this.routes;
  }

  /** Start both HTTP and WebSocket servers */
  async start(): Promise<void> {
    // Start WebSocket server
    this.wsServer.subscribeToEventBus(this.deps.eventBus);
    await this.wsServer.start();

    // Start HTTP server
    await new Promise<void>((resolve) => {
      this.httpServer = createServer(async (req, res) => {
        await this.handleRequest(req, res);
      });

      this.httpServer.listen(this.config.httpPort, () => {
        resolve();
      });
    });
  }

  /** Stop both servers */
  async stop(): Promise<void> {
    await this.wsServer.stop();
    return new Promise((resolve) => {
      if (!this.httpServer) return resolve();
      this.httpServer.close(() => resolve());
    });
  }

  /** Get server stats */
  getStats(): Record<string, unknown> {
    return {
      uptimeMs: Date.now() - this.startTime,
      requestsTotal: this.requestCount,
      errorsTotal: this.errorCount,
      avgLatencyMs: this.requestCount > 0 ? Math.round(this.totalLatencyMs / this.requestCount) : 0,
      wsClients: this.wsServer.getClientCount(),
      routes: this.routes.length,
    };
  }

  private async handleRequest(req: IncomingMessage, res: ServerResponse): Promise<void> {
    const start = Date.now();
    const method = req.method || 'GET';
    const url = new URL(req.url || '/', `http://localhost:${this.config.httpPort}`);
    const pathname = url.pathname;

    // CORS headers
    if (this.config.cors) {
      res.setHeader('Access-Control-Allow-Origin', '*');
      res.setHeader('Access-Control-Allow-Methods', 'GET, POST, PUT, DELETE, OPTIONS');
      res.setHeader('Access-Control-Allow-Headers', 'Content-Type, Authorization');
      res.setHeader('Access-Control-Max-Age', '86400');
    }

    // Handle preflight
    if (method === 'OPTIONS') {
      res.writeHead(204);
      res.end();
      return;
    }

    // Rate limiting
    const clientIp = getClientIp(req);
    const rateResult = this.rateLimiter.check(clientIp, pathname);
    res.setHeader('X-RateLimit-Limit', String(this.config.rateLimitMax));
    res.setHeader('X-RateLimit-Remaining', String(rateResult.remaining));
    res.setHeader('X-RateLimit-Reset', String(Math.ceil(rateResult.resetAt / 1000)));

    if (!rateResult.allowed) {
      res.setHeader('Retry-After', String(rateResult.retryAfter));
      sendJson(res, 429, { error: 'Too many requests', code: 'RATE_LIMITED', retryAfter: rateResult.retryAfter });
      this.logRequest({ method, path: pathname, status: 429, latencyMs: Date.now() - start });
      return;
    }

    try {
      await this.dispatch(req, res, method, pathname, url);
      this.logRequest({ method, path: pathname, status: res.statusCode || 200, latencyMs: Date.now() - start });
    } catch (err: unknown) {
      const message = err instanceof Error ? err.message : 'Unknown error';
      const statusCode = err instanceof APIError ? err.statusCode : 500;
      const code = err instanceof APIError ? err.code : 'INTERNAL_ERROR';

      this.logRequest({ method, path: pathname, status: statusCode, latencyMs: Date.now() - start, error: message });
      sendJson(res, statusCode, { error: message, code });
    }
  }

  private async dispatch(
    req: IncomingMessage,
    res: ServerResponse,
    method: string,
    pathname: string,
    url: URL,
  ): Promise<void> {
    for (const route of this.routes) {
      const params = matchRoute(method, pathname, route.method, route.pattern);
      if (params === null) continue;

      // Auth check
      if (route.authRequired) {
        const authHeader = req.headers['authorization'] as string | undefined;
        const authResult = authenticateRequest(authHeader);
        if (!authResult) {
          throw new APIError(401, 'UNAUTHORIZED', 'Authentication required');
        }
        // Role check (if specified)
        if (route.roles && route.roles.length > 0) {
          if (!route.roles.includes(authResult.user.role)) {
            throw new APIError(403, 'FORBIDDEN', 'Insufficient permissions');
          }
        }
      }

      // Read body for non-GET requests
      let body: Record<string, unknown> | undefined;
      if (method === 'POST' || method === 'PUT' || method === 'PATCH') {
        const raw = await readBody(req, this.config.maxBodySize);
        body = parseJsonBody(raw);
      }

      // Route handler signature: (req, res, params, body?, query?)
      await route.handler(req, res, params, body, url.searchParams);
      return;
    }

    throw new APIError(404, 'NOT_FOUND', `Route not found: ${method} ${pathname}`);
  }

  private logRequest(log: RequestLog): void {
    this.requestCount++;
    this.totalLatencyMs += log.latencyMs;
    if (log.status >= 500) this.errorCount++;
  }
}
