// Core type definitions

/* eslint-disable @typescript-eslint/no-explicit-any -- Zod schemas are typed as any at boundaries */
type ZodTypeLike = { parse: (input: unknown) => unknown; safeParse: (input: unknown) => unknown };

export type TaskState = 'pending' | 'running' | 'done' | 'failed';

export interface Task {
  id: string;
  name: string;
  agent: string;
  skill: string;
  dependencies: string[];
  state: TaskState;
  input?: unknown;
  output?: unknown;
  error?: string;
  createdAt: Date;
  startedAt?: Date;
  completedAt?: Date;
}

export type WorkflowState = 'INIT' | 'ANALYZE' | 'PLAN' | 'EXECUTE' | 'VERIFY' | 'FIX' | 'DONE' | 'FAILED' | 'PAUSED';

export interface Workflow {
  id: string;
  name: string;
  description: string;
  state: WorkflowState;
  tasks: Task[];
  metadata: Record<string, unknown>;
  traceId?: string;
  createdAt: Date;
  updatedAt: Date;
}

export type EventType =
  | 'workflow.started'
  | 'workflow.completed'
  | 'workflow.failed'
  | 'workflow.fixing'
  | 'workflow.paused'
  | 'workflow.resumed'
  | 'workflow.deleted'
  | 'workflow.state.transition'
  | 'task.started'
  | 'task.completed'
  | 'task.failed'
  | 'skill.registered'
  | 'skill.executed'
  | 'agent.started'
  | 'agent.task.started'
  | 'agent.task.completed'
  | 'agent.task.failed'
  | 'agent.message'
  | 'graph.analyzed'
  | 'file.indexed'
  | 'repository.indexed'
  | 'search.completed'
  | 'learning.updated'
  | 'git.status.completed'
  | 'git.diff.completed'
  | 'git.commit.completed'
  | 'tests.run.completed'
  | 'code.search.completed'
  | 'store.connected'
  | 'store.error'
  | 'metrics.recorded'
  | 'trace.started'
  | 'trace.completed'
  | 'health.check.completed'
  | 'github.pr.created'
  | 'github.pr.listed'
  | 'github.issue.listed'
  | 'docker.build.completed'
  | 'docker.run.completed'
  | 'docker.ps.completed'
  | 'cicd.pipeline.status'
  | 'cicd.pipeline.triggered'
  | 'cicd.runs.viewed'
  | 'plugin.loaded'
  | 'plugin.unloaded';

export interface Event {
  type: EventType;
  timestamp: Date;
  data: unknown;
}

export interface Skill {
  name: string;
  description: string;
  inputSchema: ZodTypeLike;
  outputSchema: ZodTypeLike;
  execute(input: unknown): Promise<unknown>;
}

// ============================================================
// Unified types from msd-mcp shared-types + agentic-llm-mem types
// ============================================================

// --- MSD-MCP Workflow/Plan/Session Types ---

export type MsdWorkflowStatus =
  | 'planning'
  | 'researching'
  | 'ready'
  | 'executing'
  | 'reviewing'
  | 'blocked'
  | 'completed';

export type MsdTaskStatus =
  | 'todo'
  | 'in_progress'
  | 'reviewing'
  | 'blocked'
  | 'done';

export type MsdReviewDecision =
  | 'APPROVED'
  | 'REWORK_REQUIRED'
  | 'BLOCKED';

export interface ContextPack {
  workflowId: string;
  planId: string;
  taskId: string;
  planSummary: string;
  taskTitle: string;
  acceptanceCriteria: string[];
  requiredContext: string[];
  recentProgress: string[];
  semanticMemories: Array<{
    id: string;
    summary: string;
    content: string;
    score?: number;
  }>;
  semanticDocs: Array<{
    id: string;
    title: string | null;
    content: string;
    score?: number;
  }>;
  fingerprint: string;
  contextSufficient: boolean;
}

export interface HybridContextPack extends ContextPack {
  executionMode?: 'sequential' | 'parallel';
}

export interface SessionReadiness {
  sessionKey: string;
  workflowLoaded: boolean;
  planLoaded: boolean;
  taskLoaded: boolean;
  contextLoaded: boolean;
  reviewApproved: boolean;
  workflowId?: string;
  planId?: string;
  taskId?: string;
  contextFingerprint?: string;
  executionMode?: 'sequential' | 'parallel';
  synthesisReady?: boolean;
  verificationReady?: boolean;
}

export interface PlanContent {
  executionModeRecommendation?: 'sequential' | 'parallel';
  tasks: Array<{
    title: string;
    priority?: string;
    ownerAgent?: string;
    acceptanceCriteria?: string[];
    requiredContext?: string[];
    verificationSteps?: string[];
    mode?: 'sequential' | 'parallel-safe';
    parallelBranches?: Array<{
      branchKey: string;
      role: string;
      scope: string;
    }>;
  }>;
}

// --- Local State (.masday/) ---

export type ArtifactCategory =
  | 'context/codebase'
  | 'context/research'
  | 'context/intel'
  | 'plans'
  | 'reports'
  | 'artifacts/diagrams';

export interface LocalState {
  syncedAt: string;
  workflow: {
    id: string;
    name: string;
    status: string;
    createdAt: string;
    updatedAt: string;
  };
  plan: {
    id: string;
    version: number;
    summary: string | null;
  } | null;
  currentTask: {
    id: string;
    title: string;
    status: string;
    progressPercent: number | null;
  } | null;
  tasks: Array<{
    id: string;
    title: string;
    status: string;
    progressPercent: number | null;
  }>;
}

// --- Agentic-LLM-Mem Memory Types ---

export type MemoryType = 'fact' | 'preference' | 'skill' | 'experience' | 'strategy' | 'decision' | 'artifact' | 'learning' | 'blocker';

export interface MemoryRecord {
  id: string;
  content: string;
  type: MemoryType;
  importance: number;
  tags: string[];
  embedding?: number[];
  source: string;
  version: number;
  createdAt: number;
  updatedAt: number;
  accessedAt: number;
  accessCount: number;
}

export interface MemorySearchResult {
  memory: MemoryRecord;
  score: number;
}

export interface ReflectionResult {
  conflicts: MemoryConflict[];
  merges: MemoryMerge[];
  pruned: string[];
}

export interface MemoryConflict {
  memoryIds: string[];
  description: string;
  resolution: string;
}

export interface MemoryMerge {
  sourceIds: string[];
  mergedContent: string;
  newImportance: number;
}

// --- Knowledge Graph Types ---

export type GraphNodeType = 'user' | 'skill' | 'project' | 'concept' | 'tool' | 'memory' | 'workflow' | 'task' | 'research';

export interface GraphNodeRecord {
  id: string;
  type: GraphNodeType;
  label: string;
  properties: Record<string, unknown>;
}

export interface GraphEdgeRecord {
  id: string;
  from: string;
  to: string;
  relation: string;
  weight: number;
}

// --- LLM Types ---

export type AgentRole = 'planner' | 'executor' | 'critic' | 'reflector' | 'router';

export interface AgentConfig {
  name: string;
  role: AgentRole;
  model?: string;
  maxIterations: number;
  temperature?: number;
}

export interface LoopState {
  iteration: number;
  goal: string;
  plan: string | null;
  result: string | null;
  feedback: string | null;
  score: number;
  history: LoopStep[];
  status: 'running' | 'success' | 'failed' | 'max_iterations';
}

export interface LoopStep {
  iteration: number;
  phase: 'plan' | 'execute' | 'critique' | 'refine';
  input: string;
  output: string;
  timestamp: number;
  durationMs: number;
}

export interface AgentContext {
  sessionId: string;
  memories: MemorySearchResult[];
  loopState: LoopState;
  config: AgentConfig;
  metadata: Record<string, unknown>;
}

export interface LLMProvider {
  complete(prompt: string, options?: LLMOptions): Promise<LLMResponse>;
}

export interface LLMOptions {
  model?: string;
  temperature?: number;
  maxTokens?: number;
}

export interface LLMResponse {
  text: string;
  tokensUsed: number;
  latencyMs: number;
  model: string;
}

// --- Streaming / Evaluation Types ---

export type StreamEvent =
  | { event: 'start'; payload: { goal: string } }
  | { event: 'plan'; payload: { plan: string; iteration: number } }
  | { event: 'execute'; payload: { result: string; iteration: number } }
  | { event: 'critique'; payload: { feedback: string; score: number; iteration: number } }
  | { event: 'memory_hit'; payload: { memories: string[] } }
  | { event: 'refine'; payload: { refinedGoal: string; iteration: number } }
  | { event: 'done'; payload: { result: string; iterations: number } }
  | { event: 'error'; payload: { error: string } };

export interface EvalRun {
  id: string;
  query: string;
  iterationsUsed: number;
  memoryHits: number;
  finalScore: number;
  correct: boolean;
  latencyMs: number;
  costEstimate: number;
  timestamp: number;
}

export interface EvalMetrics {
  memoryHitRate: number;
  avgIterations: number;
  avgScore: number;
  successRate: number;
  avgLatencyMs: number;
  avgCost: number;
}

// --- Reward / RL Types ---

export type RewardSource = 'explicit' | 'implicit' | 'auto_eval';

export interface RewardSignal {
  memoryId?: string;
  strategyId?: string;
  source: RewardSource;
  value: number;
  context: string;
  timestamp: number;
}

export interface Strategy {
  id: string;
  queryType: string;
  strategy: string;
  avgReward: number;
  usesCount: number;
  lastUsed: number;
}
