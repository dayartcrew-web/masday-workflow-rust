// ============================================================
// Policy routes — session readiness, validation, drift, audit
// ============================================================

import type { ServerResponse, IncomingMessage } from 'http';
import { sendJson } from '../utils';
import type { RouteDefinition } from '../utils';

export interface PolicyServiceProvider {
  checkReadiness(sessionKey: string): Promise<unknown>;
  validateExecution(input: { workflowId: string; taskId: string; sessionKey: string }): Promise<unknown>;
  validateCompletion(input: { workflowId: string; taskId: string; acceptanceCriteria: string[]; evidence: string[] }): Promise<unknown>;
  validateParallel(input: { workflowId: string; branchResults: Array<Record<string, unknown>>; mergeStrategy?: string }): Promise<unknown>;
  detectDrift(input: { workflowId: string; originalScope: string; currentInput: string; threshold?: number }): Promise<unknown>;
  requireContextRefresh(input: { workflowId: string; planId: string; taskId: string }): Promise<unknown>;
  auditWorkflow(workflowId: string): Promise<unknown>;
}

export function createPolicyRoutes(provider: PolicyServiceProvider): RouteDefinition[] {
  return [
    // GET /api/policy/session/:key — Check readiness
    {
      method: 'GET',
      pattern: '/api/policy/session/:key',
      authRequired: true,
      handler: async (_req: IncomingMessage, res: ServerResponse, params: Record<string, string>) => {
        const result = await provider.checkReadiness(params.key);
        sendJson(res, 200, result);
      },
    },
    // POST /api/policy/validate/execution — Validate execution
    {
      method: 'POST',
      pattern: '/api/policy/validate/execution',
      authRequired: true,
      handler: async (_req: IncomingMessage, res: ServerResponse, _params, body?: Record<string, unknown>) => {
        const input = body!;
        const result = await provider.validateExecution({
          workflowId: input.workflowId as string,
          taskId: input.taskId as string,
          sessionKey: input.sessionKey as string,
        });
        sendJson(res, 200, result);
      },
    },
    // POST /api/policy/validate/completion — Validate completion
    {
      method: 'POST',
      pattern: '/api/policy/validate/completion',
      authRequired: true,
      handler: async (_req: IncomingMessage, res: ServerResponse, _params, body?: Record<string, unknown>) => {
        const input = body!;
        const result = await provider.validateCompletion({
          workflowId: input.workflowId as string,
          taskId: input.taskId as string,
          acceptanceCriteria: input.acceptanceCriteria as string[],
          evidence: input.evidence as string[],
        });
        sendJson(res, 200, result);
      },
    },
    // POST /api/policy/validate/parallel — Validate parallel
    {
      method: 'POST',
      pattern: '/api/policy/validate/parallel',
      authRequired: true,
      handler: async (_req: IncomingMessage, res: ServerResponse, _params, body?: Record<string, unknown>) => {
        const input = body!;
        const result = await provider.validateParallel({
          workflowId: input.workflowId as string,
          branchResults: input.branchResults as Array<Record<string, unknown>>,
          mergeStrategy: input.mergeStrategy as string | undefined,
        });
        sendJson(res, 200, result);
      },
    },
    // POST /api/policy/drift — Detect scope drift
    {
      method: 'POST',
      pattern: '/api/policy/drift',
      authRequired: true,
      handler: async (_req: IncomingMessage, res: ServerResponse, _params, body?: Record<string, unknown>) => {
        const input = body!;
        const result = await provider.detectDrift({
          workflowId: input.workflowId as string,
          originalScope: input.originalScope as string,
          currentInput: input.currentInput as string,
          threshold: input.threshold as number | undefined,
        });
        sendJson(res, 200, result);
      },
    },
    // POST /api/policy/fingerprint — Require context refresh
    {
      method: 'POST',
      pattern: '/api/policy/fingerprint',
      authRequired: true,
      handler: async (_req: IncomingMessage, res: ServerResponse, _params, body?: Record<string, unknown>) => {
        const input = body!;
        const result = await provider.requireContextRefresh({
          workflowId: input.workflowId as string,
          planId: input.planId as string,
          taskId: input.taskId as string,
        });
        sendJson(res, 200, result);
      },
    },
    // GET /api/policy/audit/:workflowId — Audit workflow
    {
      method: 'GET',
      pattern: '/api/policy/audit/:workflowId',
      authRequired: true,
      handler: async (_req: IncomingMessage, res: ServerResponse, params: Record<string, string>) => {
        const result = await provider.auditWorkflow(params.workflowId);
        sendJson(res, 200, result);
      },
    },
  ];
}
