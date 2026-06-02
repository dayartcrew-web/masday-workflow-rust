// ============================================================
// API Client — HTTP + WebSocket with JWT auth, auto-reconnect
// ============================================================

import type {
  LoginResponse,
  AuthUser,
  Workflow,
  Task,
  MemoryEntry,
  ProviderInfo,
  HealthStatus,
  SystemStats,
  Metrics,
  ReActStep,
  PolicyValidation,
  DriftResult,
  AuditResult,
  WSEvent,
} from './types';

const API_BASE = process.env.NEXT_PUBLIC_API_URL || 'http://localhost:30101';
const WS_BASE = process.env.NEXT_PUBLIC_WS_URL || 'ws://localhost:30101';

// ============================================================
// HTTP Client
// ============================================================

function getToken(): string | null {
  if (typeof window === 'undefined') return null;
  return localStorage.getItem('auth_token');
}

async function request<T>(
  method: string,
  path: string,
  body?: unknown,
): Promise<T> {
  const token = getToken();
  const headers: Record<string, string> = {};
  if (token) {
    headers['Authorization'] = `Bearer ${token}`;
  }
  if (body !== undefined) {
    headers['Content-Type'] = 'application/json';
  }

  const res = await fetch(`${API_BASE}${path}`, {
    method,
    headers,
    body: body ? JSON.stringify(body) : undefined,
  });

  if (res.status === 401) {
    localStorage.removeItem('auth_token');
    if (typeof window !== 'undefined' && !window.location.pathname.includes('/login')) {
      window.location.href = '/login';
    }
    throw new Error('Session expired. Please log in again.');
  }


  // 204 No Content is a valid successful response with an empty body.
  if (res.status === 204) {
    return undefined as unknown as T;
  }

  const contentType = res.headers.get('content-type') ?? '';
  const rawText = await res.text();
  const hasBody = rawText.trim().length > 0;

  const isJson = contentType.includes('application/json') || contentType.includes('+json');
  const parsedBody: unknown = (() => {
    if (!hasBody) return undefined;
    if (!isJson) return rawText;
    try {
      return JSON.parse(rawText);
    } catch {
      // Some APIs send invalid JSON or mismatched content-type.
      return rawText;
    }
  })();

  if (!res.ok) {
    const detail = (() => {
      if (typeof parsedBody === 'string') return parsedBody;
      if (parsedBody && typeof parsedBody === 'object') {
        const obj = parsedBody as Record<string, unknown>;
        const msg = obj.error ?? obj.message;
        if (typeof msg === 'string' && msg.trim().length > 0) return msg;
      }
      return undefined;
    })();

    const statusText = res.statusText ? ` ${res.statusText}` : '';
    throw new Error(
      `API ${method.toUpperCase()} ${path} failed: ${res.status}${statusText}${detail ? ` - ${detail}` : ''}`,
    );
  }

  return parsedBody as T;
}

// ============================================================
// Auth API
// ============================================================

export const authApi = {
  login(email: string, name: string): Promise<LoginResponse> {
    return request<LoginResponse>('POST', '/api/auth/login', { email, name });
  },
  verifyToken(token: string): Promise<{ valid: boolean; payload?: unknown }> {
    return request('POST', '/api/auth/token', { token });
  },
  getMe(): Promise<{ user: AuthUser }> {
    return request('GET', '/api/auth/me');
  },
};

// ============================================================
// Workflow API
// ============================================================

export const workflowApi = {
  list(): Promise<{ workflows: Workflow[] }> {
    return request('GET', '/api/workflows');
  },
  get(id: string): Promise<{ workflow: Workflow }> {
    return request('GET', `/api/workflows/${id}`);
  },
  getActive(): Promise<{ workflow: Workflow | null }> {
    return request('GET', '/api/workflows/active');
  },
  getStatus(id: string): Promise<{ status: unknown }> {
    return request('GET', `/api/workflows/${id}/status`);
  },
  create(name: string, description: string, metadata?: Record<string, unknown>): Promise<{ workflow: Workflow }> {
    return request('POST', '/api/workflows', { name, description, metadata });
  },
  execute(id: string): Promise<{ workflow: Workflow }> {
    return request('POST', `/api/workflows/${id}/execute`);
  },
  createPlan(id: string, tasks: Array<Record<string, unknown>>): Promise<{ plan: { tasks: Task[]; taskCount: number } }> {
    return request('POST', `/api/workflows/${id}/plan`, { tasks });
  },
  getTasks(workflowId: string): Promise<{ tasks: Task[] }> {
    return request('GET', `/api/workflows/${workflowId}/tasks`);
  },
  addTask(workflowId: string, task: { name: string; agent: string; skill: string; dependencies?: string[]; input?: unknown }): Promise<{ task: Task }> {
    return request('POST', `/api/workflows/${workflowId}/tasks`, task);
  },
  startTask(workflowId: string, taskId: string): Promise<{ task: Task }> {
    return request('POST', `/api/workflows/${workflowId}/tasks/${taskId}/start`);
  },
  completeTask(workflowId: string, taskId: string, output?: unknown): Promise<{ task: Task }> {
    return request('POST', `/api/workflows/${workflowId}/tasks/${taskId}/complete`, { output });
  },
  saveProgress(workflowId: string, taskId: string, progressNote: string): Promise<{ saved: boolean }> {
    return request('POST', `/api/workflows/${workflowId}/tasks/${taskId}/progress`, { progressNote });
  },
};

// ============================================================
// Memory API
// ============================================================

export const memoryApi = {
  store(entry: { memoryType: string; summary: string; content: string; importance?: number; taskId?: string; workflowId?: string }): Promise<{ id: string }> {
    return request('POST', '/api/memory', entry);
  },
  storeResearch(entry: { workflowId: string; query: string; findings: string; sources: string[] }): Promise<{ id: string }> {
    return request('POST', '/api/memory/research', entry);
  },
  recallDocuments(workflowId: string, limit?: number): Promise<{ documents: MemoryEntry[] }> {
    const params = limit ? `?limit=${limit}` : '';
    return request('GET', `/api/memory/${workflowId}${params}`);
  },
  recallRecent(workflowId: string, limit?: number): Promise<{ memories: MemoryEntry[] }> {
    return request('GET', `/api/memory/${workflowId}/recent?limit=${limit || 10}`);
  },
  recallByType(workflowId: string, type: string, limit?: number): Promise<{ memories: MemoryEntry[] }> {
    const params = limit ? `?limit=${limit}` : '';
    return request('GET', `/api/memory/${workflowId}/by-type/${type}${params}`);
  },
  recallByTask(taskId: string, limit?: number): Promise<{ memories: MemoryEntry[] }> {
    const params = limit ? `?limit=${limit}` : '';
    return request('GET', `/api/memory/task/${taskId}${params}`);
  },
  update(id: string, updates: Record<string, unknown>): Promise<{ updated: boolean }> {
    return request('PUT', `/api/memory/${id}`, updates);
  },
  delete(id: string): Promise<{ deleted: boolean }> {
    return request('DELETE', `/api/memory/${id}`);
  },
};

// ============================================================
// Search API
// ============================================================

export const searchApi = {
  hybridContextPack(input: { workflowId: string; planId: string; taskId: string; cwd?: string }): Promise<unknown> {
    return request('POST', '/api/search/hybrid', input);
  },
  contextFingerprint(input: { workflowId: string; planId: string; taskId: string }): Promise<unknown> {
    return request('POST', '/api/search/fingerprint', input);
  },
  codeSearch(input: { query: string; glob?: string; type?: string; limit?: number }): Promise<unknown> {
    return request('POST', '/api/search/code', input);
  },
};

// ============================================================
// Chat API
// ============================================================

export const chatApi = {
  complete(input: { message: string; sessionId?: string; model?: string; temperature?: number }): Promise<unknown> {
    return request('POST', '/api/chat', input);
  },
  async react(input: { goal: string; maxIterations?: number; sessionId?: string }): Promise<{ steps: ReActStep[]; result: string }> {
    const raw = await request('POST', '/api/chat/react', input) as Record<string, unknown>;
    if (raw.ok === false) {
      throw new Error(String(raw.error ?? 'ReAct execution failed'));
    }
    return {
      steps: Array.isArray(raw.steps) ? raw.steps as ReActStep[] : [],
      result: String(raw.result ?? ''),
    };
  },
};

// ============================================================
// Provider API
// ============================================================

export const providerApi = {
  list(): Promise<{ providers: ProviderInfo[] }> {
    return request('GET', '/api/providers');
  },
  test(name: string, input?: { model?: string; prompt?: string }): Promise<unknown> {
    return request('POST', `/api/providers/${name}/test`, input || {});
  },
};

// ============================================================
// Policy API
// ============================================================

export const policyApi = {
  checkReadiness(sessionKey: string): Promise<unknown> {
    return request('GET', `/api/policy/session/${sessionKey}`);
  },
  validateExecution(input: { workflowId: string; taskId: string; sessionKey: string }): Promise<PolicyValidation> {
    return request('POST', '/api/policy/validate/execution', input);
  },
  validateCompletion(input: { workflowId: string; taskId: string; acceptanceCriteria: string[]; evidence: string[] }): Promise<PolicyValidation> {
    return request('POST', '/api/policy/validate/completion', input);
  },
  validateParallel(input: { workflowId: string; branchResults: Array<Record<string, unknown>>; mergeStrategy?: string }): Promise<PolicyValidation> {
    return request('POST', '/api/policy/validate/parallel', input);
  },
  detectDrift(input: { workflowId: string; originalScope: string; currentInput: string; threshold?: number }): Promise<DriftResult> {
    return request('POST', '/api/policy/drift', input);
  },
  requireContextRefresh(input: { workflowId: string; planId: string; taskId: string }): Promise<unknown> {
    return request('POST', '/api/policy/fingerprint', input);
  },
  async auditWorkflow(workflowId: string): Promise<AuditResult> {
    const raw = await request('GET', `/api/policy/audit/${workflowId}`) as Record<string, unknown>;
    const issues: Array<Record<string, unknown>> = Array.isArray(raw.issues) ? raw.issues as Array<Record<string, unknown>> : [];
    const stuck = issues.filter((i) => i.type === 'stuck_task') as unknown as AuditResult['stuckTasks'];
    const missing = issues.filter((i) => i.type === 'missing_review').map((i) => String(i.message ?? i));
    const incomplete = issues.filter((i) => i.type === 'incomplete_progress').map((i) => String(i.message ?? i));
    return {
      workflowId: String(raw.workflowId ?? workflowId),
      stuckTasks: Array.isArray(raw.stuckTasks) ? raw.stuckTasks as AuditResult['stuckTasks'] : stuck,
      missingReviews: Array.isArray(raw.missingReviews) ? raw.missingReviews as string[] : missing,
      incompleteProgress: Array.isArray(raw.incompleteProgress) ? raw.incompleteProgress as string[] : incomplete,
      totalIssues: typeof raw.totalIssues === 'number' ? raw.totalIssues : issues.length,
    };
  },
};

// ============================================================
// Capability API
// ============================================================

export const capabilityApi = {
  createAgent(input: { name: string; role: string; projectRoot: string; description?: string }): Promise<unknown> {
    return request('POST', '/api/capability/agent', input);
  },
  createSkill(input: { name: string; projectRoot: string; description?: string; agentName?: string }): Promise<unknown> {
    return request('POST', '/api/capability/skill', input);
  },
  listAgents(projectRoot?: string): Promise<unknown> {
    const params = projectRoot ? `?projectRoot=${encodeURIComponent(projectRoot)}` : '';
    return request('GET', `/api/capability/agents${params}`);
  },
  matchAgent(input: { taskType: string; requiredTools?: string[]; projectRoot: string }): Promise<unknown> {
    return request('POST', '/api/capability/match', input);
  },
  listSkills(projectRoot?: string): Promise<unknown> {
    const params = projectRoot ? `?projectRoot=${encodeURIComponent(projectRoot)}` : '';
    return request('GET', `/api/capability/skills${params}`);
  },
  listTemplates(): Promise<unknown> {
    return request('GET', '/api/capability/templates');
  },
  checkReadiness(projectRoot?: string): Promise<unknown> {
    const params = projectRoot ? `?projectRoot=${encodeURIComponent(projectRoot)}` : '';
    return request('GET', `/api/capability/readiness${params}`);
  },
  auditWorkflow(workflowId: string, projectRoot?: string): Promise<unknown> {
    const params = `?workflowId=${workflowId}${projectRoot ? `&projectRoot=${encodeURIComponent(projectRoot)}` : ''}`;
    return request('GET', `/api/capability/audit${params}`);
  },
};

// ============================================================
// Monitoring API
// ============================================================

export const monitoringApi = {
  async getHealth(): Promise<HealthStatus> {
    const raw = await request('GET', '/api/health') as Record<string, unknown>;
    const checks: Record<string, { status: string; latencyMs: number }> = {};
    if (Array.isArray(raw.checks)) {
      for (const c of raw.checks as Array<Record<string, unknown>>) {
        checks[String(c.name ?? 'unknown')] = {
          status: String(c.status ?? 'unknown'),
          latencyMs: typeof c.duration === 'number' ? c.duration : 0,
        };
      }
    }
    return {
      status: (raw.status as HealthStatus['status']) ?? 'healthy',
      uptimeMs: typeof raw.uptimeMs === 'number' ? raw.uptimeMs : 0,
      version: String(raw.version ?? 'unknown'),
      checks,
    };
  },
  async getMetrics(): Promise<Metrics> {
    const raw = await request('GET', '/api/stats') as Record<string, unknown>;
    const engine = (raw.engine ?? {}) as Record<string, unknown>;
    return {
      workflowsTotal: typeof engine.workflows === 'number' ? engine.workflows : 0,
      workflowsActive: typeof engine.workflowsActive === 'number' ? engine.workflowsActive : (typeof engine.workflows === 'number' ? engine.workflows : 0),
      workflowsCompleted: typeof engine.workflowsCompleted === 'number' ? engine.workflowsCompleted : 0,
      workflowsFailed: typeof engine.workflowsFailed === 'number' ? engine.workflowsFailed : 0,
      tasksTotal: typeof engine.tasksTotal === 'number' ? engine.tasksTotal : 0,
      tasksCompleted: typeof engine.tasksCompleted === 'number' ? engine.tasksCompleted : 0,
      tasksFailed: typeof engine.tasksFailed === 'number' ? engine.tasksFailed : 0,
      memoriesTotal: typeof engine.memoriesTotal === 'number' ? engine.memoriesTotal : 0,
      tokensUsed: typeof engine.tokensUsed === 'number' ? engine.tokensUsed : 0,
      tokenBreakdown: engine.tokenBreakdown && typeof engine.tokenBreakdown === 'object'
        ? engine.tokenBreakdown as Record<string, number>
        : undefined,
    };
  },
  async getStats(): Promise<SystemStats> {
    const raw = await request('GET', '/api/stats') as Record<string, unknown>;
    const api = (raw.api ?? {}) as Record<string, unknown>;
    return {
      uptimeMs: typeof api.uptimeMs === 'number' ? api.uptimeMs : 0,
      requestsTotal: typeof api.requestsTotal === 'number' ? api.requestsTotal : 0,
      errorsTotal: typeof api.errorsTotal === 'number' ? api.errorsTotal : 0,
      avgLatencyMs: typeof api.avgLatencyMs === 'number' ? api.avgLatencyMs : 0,
      wsClients: typeof api.wsClients === 'number' ? api.wsClients : 0,
      routes: typeof api.routes === 'number' ? api.routes : 0,
    };
  },
};

// ============================================================
// WebSocket Client
// ============================================================

type WSMessageHandler = (event: WSEvent) => void;

export class WebSocketClient {
  private ws: WebSocket | null = null;
  private token: string | null = null;
  private handlers: Set<WSMessageHandler> = new Set();
  private reconnectTimer: ReturnType<typeof setTimeout> | null = null;
  private reconnectAttempts = 0;
  private maxReconnectAttempts = 10;
  private channels: Set<string> = new Set(['workflow', 'task', 'memory', 'system']);
  private _connected = false;
  private shouldReconnect = true;

  get connected(): boolean {
    return this._connected;
  }

  connect(token: string): void {
    this.token = token;
    this.reconnectAttempts = 0;
    this.shouldReconnect = true;
    this.doConnect();
  }

  disconnect(): void {
    this.shouldReconnect = false;
    if (this.reconnectTimer) {
      clearTimeout(this.reconnectTimer);
      this.reconnectTimer = null;
    }
    if (this.ws) {
      this.ws.close(1000, 'Client disconnect');
      this.ws = null;
    }
    this._connected = false;
  }

  subscribe(channel: string): void {
    this.channels.add(channel);
    if (this.ws && this.ws.readyState === WebSocket.OPEN) {
      this.ws.send(JSON.stringify({ action: 'subscribe', channel }));
    }
  }

  unsubscribe(channel: string): void {
    this.channels.delete(channel);
    if (this.ws && this.ws.readyState === WebSocket.OPEN) {
      this.ws.send(JSON.stringify({ action: 'unsubscribe', channel }));
    }
  }

  onEvent(handler: WSMessageHandler): () => void {
    this.handlers.add(handler);
    return () => { this.handlers.delete(handler); };
  }

  private doConnect(): void {
    if (!this.token) return;

    const url = `${WS_BASE}?token=${this.token}`;
    this.ws = new WebSocket(url);

    this.ws.onopen = () => {
      this._connected = true;
      this.reconnectAttempts = 0;
      // Resubscribe channels
      Array.from(this.channels).forEach((channel) => {
        this.ws?.send(JSON.stringify({ action: 'subscribe', channel }));
      });
    };

    this.ws.onmessage = (event: MessageEvent) => {
      try {
        const parsed = JSON.parse(event.data as string) as WSEvent;
        Array.from(this.handlers).forEach((handler) => {
          handler(parsed);
        });
      } catch {
        // Ignore malformed messages
      }
    };

    this.ws.onclose = () => {
      this._connected = false;
      if (!this.shouldReconnect) {
        return;
      }
      this.tryReconnect();
    };

    this.ws.onerror = () => {
      this._connected = false;
    };
  }

  private tryReconnect(): void {
    if (!this.shouldReconnect) return;
    if (this.reconnectAttempts >= this.maxReconnectAttempts) return;
    if (this.reconnectTimer) return;

    const delay = Math.min(1000 * Math.pow(2, this.reconnectAttempts), 30000);
    this.reconnectTimer = setTimeout(() => {
      this.reconnectTimer = null;
      this.reconnectAttempts++;
      this.doConnect();
    }, delay);
  }
}

export const wsClient = new WebSocketClient();
