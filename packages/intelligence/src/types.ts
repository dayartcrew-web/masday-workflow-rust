// Intelligence system types
//
// Covers: code indexing, semantic search, graph analysis,
// learning system, context packs, planning, ReAct agent, guardrails.

// --- File & Code Types ---

export type FileMetadata = {
  path: string;
  size: number;
  extension: string;
  lastModified: Date;
  isDirectory: boolean;
  lineCount?: number;
  functionCount?: number;
  classCount?: number;
  language?: string;
};

export type CodeSymbol = {
  name: string;
  type: 'function' | 'class' | 'variable' | 'interface' | 'type' | 'enum' | 'constant' | 'method' | 'property' | 'module' | 'import';
  filePath: string;
  line: number;
  column: number;
  exported: boolean;
};

export type DependencyEdge = {
  from: string;
  to: string;
  type: 'import' | 'require' | 'type' | 'call' | 'instantiation';
  strength: number;
};

export type CodeUsagePattern = {
  pattern: string;
  file: string;
  context: string;
  count: number;
};

export type LearningMetric = {
  workflowId: string;
  skill: string;
  success: boolean;
  duration: number;
  timestamp: Date;
};

// --- Indexing Types ---

export interface IndexedChunk {
  id: string;
  filePath: string;
  content: string;
  embedding: number[];
  language: string;
  startLine: number;
  endLine: number;
  indexedAt: number;
}

export interface IndexResult {
  totalChunks: number;
  totalTokens: number;
  durationMs: number;
}

export interface IndexedRepository {
  files: Map<string, FileMetadata>;
  symbols: Map<string, CodeSymbol[]>;
  dependencies: Map<string, DependencyEdge[]>;
  indexedAt: Date;
}

// --- Search Types ---

export interface SemanticQuery {
  query: string;
  patterns: string[];
  context?: string[];
  fileFilter?: {
    extensions?: string[];
    pathPattern?: string;
    maxSize?: number;
  };
}

export interface SearchResult {
  filePath: string;
  line: number;
  match: string;
  context: string;
  score: number;
}

export interface SearchResultWithChunk {
  chunk: IndexedChunk;
  score: number;
  source: 'bm25' | 'vector' | 'hybrid';
}

export interface SearchConfig {
  rrfK?: number;
  enableBM25?: boolean;
  enableVector?: boolean;
}

// --- Graph Types ---

export interface CodeGraph {
  nodes: Map<string, FileMetadata>;
  edges: Map<string, DependencyEdge[]>;
  entryPoints: string[];
}

// --- Learning Types ---

export interface LearningData {
  metrics: LearningMetric[];
  patterns: CodeUsagePattern[];
  commonErrors: Map<string, number>;
  bestPractices: Map<string, unknown>;
}

// --- Context Pack Types ---

export interface ContextPackInput {
  workflowId: string;
  planId: string;
  taskId: string;
  planSummary: string;
  taskTitle: string;
  acceptanceCriteria: string[];
  requiredContext: string[];
  recentProgress: string[];
}

export interface ContextPackMemory {
  id: string;
  summary: string;
  content: string;
  score?: number;
}

export interface ContextPackDoc {
  id: string;
  title: string | null;
  content: string;
  score?: number;
}

export interface ContextPackResult {
  workflowId: string;
  planId: string;
  taskId: string;
  planSummary: string;
  taskTitle: string;
  acceptanceCriteria: string[];
  requiredContext: string[];
  recentProgress: string[];
  semanticMemories: ContextPackMemory[];
  semanticDocs: ContextPackDoc[];
  fingerprint: string;
  contextSufficient: boolean;
}

export interface HybridContextPackResult extends ContextPackResult {
  executionMode?: 'sequential' | 'parallel';
}

// --- Planner Types ---

export interface PlanSuggestion {
  type: 'approach' | 'pattern' | 'risk' | 'dependency';
  content: string;
  confidence: number;
  source: string;
}

export interface IntelligencePlanResult {
  goal: string;
  suggestions: PlanSuggestion[];
  relevantMemories: Array<{
    memory: { id: string; content: string; type: string; importance: number };
    score: number;
  }>;
  relevantCode: SearchResult[];
  generatedAt: number;
}

export interface PlannerConfig {
  maxMemoryResults?: number;
  maxCodeResults?: number;
  minConfidence?: number;
  repositoryRoot?: string;
}

// --- ReAct Types ---

export interface ToolDefinition {
  name: string;
  description: string;
  parameters: Record<string, { type: string; description: string; required?: boolean }>;
}

export interface ReActTrace {
  step: number;
  thought: string;
  action: string | null;
  actionInput: Record<string, unknown> | null;
  observation: string | null;
  timestamp: number;
}

export interface ReActResult {
  answer: string;
  traces: ReActTrace[];
  toolCalls: number;
  iterations: number;
}

export interface ReActConfig {
  maxSteps?: number;
  temperature?: number;
  systemPrompt?: string;
}

// --- Guardrails Types ---

export interface GuardrailResult<T> {
  valid: boolean;
  data: T | null;
  errors: string[];
}

// --- Tool Handler Types ---

export interface ToolHandlerContext {
  workflowId: string;
  planId: string;
  taskId: string;
  cwd?: string;
}
