// ============================================================
// Shared types for the dashboard — mirrors API server types
// ============================================================

export interface ApiResponse<T> {
  success: boolean;
  data?: T;
  error?: string;
  meta?: { total: number; page: number; limit: number };
}

export interface AuthUser {
  id: string;
  email: string;
  name: string;
  role: 'admin' | 'user' | 'readonly';
}

export interface LoginResponse {
  token: string;
  user: AuthUser;
}

export interface Workflow {
  id: string;
  name: string;
  description: string;
  state: string;
  tasks: Task[];
  metadata: Record<string, unknown>;
  createdAt: string;
  updatedAt: string;
}

export interface Task {
  id: string;
  name: string;
  agent: string;
  skill: string;
  state: string;
  dependencies: string[];
  input?: unknown;
  output?: unknown;
  startedAt?: string;
  completedAt?: string;
  progressLog?: TaskProgressEntry[];
}

export interface TaskProgressEntry {
  timestamp: string;
  agentName: string;
  progressNote: string;
  evidence: string[];
}

export interface MemoryEntry {
  id: string;
  memoryType: string;
  summary: string;
  content: string;
  importance: number;
  taskId?: string;
  workflowId?: string;
  createdAt: string;
  updatedAt: string;
  score?: ScoreBreakdown;
}

export interface ScoreBreakdown {
  similarity: number;
  recency: number;
  importance: number;
  usage: number;
  total: number;
}

export interface GraphNode {
  id: string;
  label: string;
  type: string;
  properties: Record<string, unknown>;
}

export interface GraphEdge {
  id: string;
  source: string;
  target: string;
  type: string;
  weight: number;
}

export interface ProviderInfo {
  name: string;
  type: string;
  models: string[];
  status: 'available' | 'unavailable' | 'error';
  circuitState: 'closed' | 'open' | 'half-open';
}

export interface HealthStatus {
  status: 'healthy' | 'degraded' | 'unhealthy';
  uptimeMs: number;
  version: string;
  checks: Record<string, { status: string; latencyMs: number }>;
}

export interface SystemStats {
  uptimeMs: number;
  requestsTotal: number;
  errorsTotal: number;
  avgLatencyMs: number;
  wsClients: number;
  routes: number;
}

export interface Metrics {
  workflowsTotal: number;
  workflowsActive: number;
  workflowsCompleted: number;
  workflowsFailed: number;
  tasksTotal: number;
  tasksCompleted: number;
  tasksFailed: number;
  memoriesTotal: number;
  tokensUsed: number;
}

export interface ChatMessage {
  role: 'user' | 'assistant' | 'system';
  content: string;
  timestamp: string;
  memoryContext?: MemoryEntry[];
}

export interface ReActStep {
  iteration: number;
  thought: string;
  action: string;
  observation: string;
}

export interface PolicyValidation {
  valid: boolean;
  errors: string[];
  warnings: string[];
}

export interface DriftResult {
  drifted: boolean;
  score: number;
  threshold: number;
  recommendation: string;
}

export interface AuditResult {
  workflowId: string;
  stuckTasks: Task[];
  missingReviews: string[];
  incompleteProgress: string[];
  totalIssues: number;
}

export interface WSEvent {
  type: string;
  data: unknown;
  timestamp: string;
}
