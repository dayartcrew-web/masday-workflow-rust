// ============================================================
// Capability routes — agents, skills, templates, readiness, audit
// ============================================================

import type { ServerResponse, IncomingMessage } from 'http';
import { sendJson } from '../utils';
import type { RouteDefinition } from '../utils';

export interface CapabilityServiceProvider {
  createAgent(input: { name: string; role: string; projectRoot: string; description?: string }): Promise<unknown>;
  createSkill(input: { name: string; projectRoot: string; description?: string; agentName?: string }): Promise<unknown>;
  listAgents(projectRoot: string): Promise<unknown>;
  matchAgent(input: { taskType: string; requiredTools?: string[]; projectRoot: string }): Promise<unknown>;
  listSkills(projectRoot: string): Promise<unknown>;
  listTemplates(): Promise<unknown>;
  checkReadiness(projectRoot: string): Promise<unknown>;
  auditWorkflow(workflowId: string, projectRoot: string): Promise<unknown>;
}

export function createCapabilityRoutes(provider: CapabilityServiceProvider): RouteDefinition[] {
  return [
    // POST /api/capability/agent — Create agent
    {
      method: 'POST',
      pattern: '/api/capability/agent',
      authRequired: true,
      handler: async (_req: IncomingMessage, res: ServerResponse, _params, body?: Record<string, unknown>) => {
        const input = body!;
        const result = await provider.createAgent({
          name: input.name as string,
          role: input.role as string,
          projectRoot: input.projectRoot as string,
          description: input.description as string | undefined,
        });
        sendJson(res, 201, result);
      },
    },
    // POST /api/capability/skill — Create skill
    {
      method: 'POST',
      pattern: '/api/capability/skill',
      authRequired: true,
      handler: async (_req: IncomingMessage, res: ServerResponse, _params, body?: Record<string, unknown>) => {
        const input = body!;
        const result = await provider.createSkill({
          name: input.name as string,
          projectRoot: input.projectRoot as string,
          description: input.description as string | undefined,
          agentName: input.agentName as string | undefined,
        });
        sendJson(res, 201, result);
      },
    },
    // GET /api/capability/agents — List agents
    {
      method: 'GET',
      pattern: '/api/capability/agents',
      authRequired: true,
      handler: async (_req: IncomingMessage, res: ServerResponse, _params: Record<string, string>, _body?: Record<string, unknown>, query?: URLSearchParams) => {
        const projectRoot = query?.get('projectRoot') || process.cwd();
        const result = await provider.listAgents(projectRoot);
        sendJson(res, 200, result);
      },
    },
    // POST /api/capability/match — Match agent
    {
      method: 'POST',
      pattern: '/api/capability/match',
      authRequired: true,
      handler: async (_req: IncomingMessage, res: ServerResponse, _params, body?: Record<string, unknown>) => {
        const input = body!;
        const result = await provider.matchAgent({
          taskType: input.taskType as string,
          requiredTools: input.requiredTools as string[] | undefined,
          projectRoot: input.projectRoot as string,
        });
        sendJson(res, 200, result);
      },
    },
    // GET /api/capability/skills — List skills
    {
      method: 'GET',
      pattern: '/api/capability/skills',
      authRequired: true,
      handler: async (_req: IncomingMessage, res: ServerResponse, _params: Record<string, string>, _body?: Record<string, unknown>, query?: URLSearchParams) => {
        const projectRoot = query?.get('projectRoot') || process.cwd();
        const result = await provider.listSkills(projectRoot);
        sendJson(res, 200, result);
      },
    },
    // GET /api/capability/templates — List templates
    {
      method: 'GET',
      pattern: '/api/capability/templates',
      authRequired: true,
      handler: async (_req: IncomingMessage, res: ServerResponse) => {
        const result = await provider.listTemplates();
        sendJson(res, 200, result);
      },
    },
    // GET /api/capability/readiness — System readiness
    {
      method: 'GET',
      pattern: '/api/capability/readiness',
      authRequired: true,
      handler: async (_req: IncomingMessage, res: ServerResponse, _params: Record<string, string>, _body?: Record<string, unknown>, query?: URLSearchParams) => {
        const projectRoot = query?.get('projectRoot') || process.cwd();
        const result = await provider.checkReadiness(projectRoot);
        sendJson(res, 200, result);
      },
    },
    // GET /api/capability/audit — Workflow audit
    {
      method: 'GET',
      pattern: '/api/capability/audit',
      authRequired: true,
      handler: async (_req: IncomingMessage, res: ServerResponse, _params: Record<string, string>, _body?: Record<string, unknown>, query?: URLSearchParams) => {
        const workflowId = query?.get('workflowId') || '';
        const projectRoot = query?.get('projectRoot') || process.cwd();
        const result = await provider.auditWorkflow(workflowId, projectRoot);
        sendJson(res, 200, result);
      },
    },
  ];
}
