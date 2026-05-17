// ============================================================
// Tests for server and routes — HTTP integration tests
// ============================================================

import { describe, it, expect, beforeAll, afterAll, vi } from 'vitest';
import { createServer, IncomingMessage, ServerResponse } from 'http';
import { APIServer } from '../index';
import { signToken, createUser } from '../auth/jwt';
import { EventBus } from '@mcp-rebuild/core';
import {
  sendJson, readBody, parseJsonBody, extractPathParams, matchRoute,
  APIError, getClientIp, str, num, optStr,
} from '../utils';

// --- Unit tests for utils ---

describe('Utils', () => {
  describe('sendJson', () => {
    it('should send JSON response', () => {
      const chunks: Buffer[] = [];
      const res = {
        writeHead: vi.fn(),
        end: vi.fn((data: string) => chunks.push(Buffer.from(data))),
      } as unknown as ServerResponse;

      sendJson(res, 200, { hello: 'world' });
      expect(res.writeHead).toHaveBeenCalledWith(200, expect.objectContaining({
        'Content-Type': 'application/json',
      }));
      expect(res.end).toHaveBeenCalled();
    });
  });

  describe('parseJsonBody', () => {
    it('should parse valid JSON', () => {
      const result = parseJsonBody('{"key": "value"}');
      expect(result).toEqual({ key: 'value' });
    });

    it('should parse empty body as empty object', () => {
      const result = parseJsonBody('');
      expect(result).toEqual({});
    });

    it('should throw APIError for invalid JSON', () => {
      expect(() => parseJsonBody('not json')).toThrow(APIError);
    });
  });

  describe('extractPathParams', () => {
    it('should extract params from matching pattern', () => {
      const params = extractPathParams('/api/workflows/wf_123', '/api/workflows/:id');
      expect(params).toEqual({ id: 'wf_123' });
    });

    it('should return null for non-matching pattern', () => {
      const params = extractPathParams('/api/workflows', '/api/workflows/:id');
      expect(params).toBeNull();
    });

    it('should extract multiple params', () => {
      const params = extractPathParams('/api/wf/a/tasks/t1', '/api/wf/:wfId/tasks/:taskId');
      expect(params).toEqual({ wfId: 'a', taskId: 't1' });
    });

    it('should handle URL-encoded values', () => {
      const params = extractPathParams('/api/memory/hello%20world', '/api/memory/:id');
      expect(params).toEqual({ id: 'hello world' });
    });
  });

  describe('matchRoute', () => {
    it('should match a valid route', () => {
      const params = matchRoute('GET', '/api/workflows/wf_1', 'GET', '/api/workflows/:id');
      expect(params).toEqual({ id: 'wf_1' });
    });

    it('should return null for method mismatch', () => {
      const params = matchRoute('POST', '/api/workflows/wf_1', 'GET', '/api/workflows/:id');
      expect(params).toBeNull();
    });

    it('should return null for path mismatch', () => {
      const params = matchRoute('GET', '/api/workflows', 'GET', '/api/workflows/:id');
      expect(params).toBeNull();
    });
  });

  describe('getClientIp', () => {
    it('should get IP from x-forwarded-for', () => {
      const req = { headers: { 'x-forwarded-for': '1.2.3.4, 5.6.7.8' }, socket: { remoteAddress: '127.0.0.1' } } as any;
      expect(getClientIp(req)).toBe('1.2.3.4');
    });

    it('should fall back to remoteAddress', () => {
      const req = { headers: {}, socket: { remoteAddress: '127.0.0.1' } } as any;
      expect(getClientIp(req)).toBe('127.0.0.1');
    });

    it('should return unknown when no IP available', () => {
      const req = { headers: {}, socket: {} } as any;
      expect(getClientIp(req)).toBe('unknown');
    });
  });

  describe('type helpers', () => {
    it('str() should return string or fallback', () => {
      expect(str('hello')).toBe('hello');
      expect(str(123)).toBe('');
      expect(str(undefined, 'default')).toBe('default');
    });

    it('num() should return number or fallback', () => {
      expect(num(42)).toBe(42);
      expect(num('not a number')).toBe(0);
      expect(num(NaN)).toBe(0);
      expect(num(undefined, 10)).toBe(10);
    });

    it('optStr() should return string or undefined', () => {
      expect(optStr('hello')).toBe('hello');
      expect(optStr(123)).toBeUndefined();
      expect(optStr(undefined)).toBeUndefined();
    });
  });
});

describe('Route registration', () => {
  it('should register 40+ routes', () => {
    const eventBus = new EventBus();

    // Create mock engine
    const mockEngine = {
      listWorkflows: vi.fn(() => []),
      createWorkflow: vi.fn(() => ({ id: 'wf_1', name: 'Test', state: 'INIT', tasks: [] })),
      getWorkflow: vi.fn(() => null),
      getStatus: vi.fn(() => null),
      executeWorkflow: vi.fn(),
      addTask: vi.fn(),
    } as any;

    const mockProvider = {
      store: vi.fn(),
      storeResearch: vi.fn(),
      recallDocuments: vi.fn(),
      recallRecent: vi.fn(),
      recallByType: vi.fn(),
      recallByTask: vi.fn(),
      update: vi.fn(),
      delete: vi.fn(),
      hybridContextPack: vi.fn(),
      contextFingerprint: vi.fn(),
      codeSearch: vi.fn(),
      checkReadiness: vi.fn(),
      validateExecution: vi.fn(),
      validateCompletion: vi.fn(),
      validateParallel: vi.fn(),
      detectDrift: vi.fn(),
      requireContextRefresh: vi.fn(),
      auditWorkflow: vi.fn(),
      createAgent: vi.fn(),
      createSkill: vi.fn(),
      listAgents: vi.fn(),
      matchAgent: vi.fn(),
      listSkills: vi.fn(),
      listTemplates: vi.fn(),
      checkSystemReadiness: vi.fn(),
      workflowAudit: vi.fn(),
      complete: vi.fn(),
      react: vi.fn(),
      listProviders: vi.fn(),
      testProvider: vi.fn(),
      getHealth: vi.fn(),
      getMetrics: vi.fn(),
      getStats: vi.fn(),
    };

    const apiServer = new APIServer(
      {
        eventBus,
        engine: mockEngine,
        memoryProvider: mockProvider as any,
        searchProvider: mockProvider as any,
        policyProvider: mockProvider as any,
        capabilityProvider: mockProvider as any,
        chatProvider: mockProvider as any,
        providerService: mockProvider as any,
        monitoringProvider: mockProvider as any,
      },
      { httpPort: 0, wsPort: 0 },
    );

    const routes = apiServer.getRoutes();
    expect(routes.length).toBeGreaterThanOrEqual(40);

    // Count endpoint categories
    const methods = routes.map((r) => `${r.method} ${r.pattern}`);
    expect(methods.some((m) => m.includes('/api/auth'))).toBe(true);
    expect(methods.some((m) => m.includes('/api/workflows'))).toBe(true);
    expect(methods.some((m) => m.includes('/api/memory'))).toBe(true);
    expect(methods.some((m) => m.includes('/api/search'))).toBe(true);
    expect(methods.some((m) => m.includes('/api/policy'))).toBe(true);
    expect(methods.some((m) => m.includes('/api/capability'))).toBe(true);
    expect(methods.some((m) => m.includes('/api/chat'))).toBe(true);
    expect(methods.some((m) => m.includes('/api/providers'))).toBe(true);
    expect(methods.some((m) => m.includes('/api/health'))).toBe(true);
    expect(methods.some((m) => m.includes('/api/metrics'))).toBe(true);
    expect(methods.some((m) => m.includes('/api/stats'))).toBe(true);
  });
});
