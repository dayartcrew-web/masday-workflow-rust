/**
 * @mcp-rebuild/intelligence - Semantic search, intelligence planning,
 * ReAct agent, and guardrails for the unified workflow platform.
 */

// Types
export type {
  FileMetadata,
  CodeSymbol,
  DependencyEdge,
  CodeUsagePattern,
  LearningMetric,
  IndexedChunk,
  IndexResult,
  IndexedRepository,
  SemanticQuery,
  SearchResult,
  SearchResultWithChunk,
  SearchConfig,
  CodeGraph,
  LearningData,
  ContextPackInput,
  ContextPackMemory,
  ContextPackDoc,
  ContextPackResult,
  HybridContextPackResult,
  PlanSuggestion,
  IntelligencePlanResult,
  PlannerConfig,
  ToolDefinition,
  ReActTrace,
  ReActResult,
  ReActConfig,
  GuardrailResult,
  ToolHandlerContext,
} from './types.js';

// Guardrails
export { Guardrails, createGuardrails } from './guardrails.js';

// Code Indexer
export { CodeIndexer } from './codeIndexer.js';
export type { IndexerEmbeddingProvider } from './codeIndexer.js';

// Semantic Searcher
export { SemanticSearcher } from './semanticSearcher.js';
export type { SearchEmbeddingProvider } from './semanticSearcher.js';

// Code Graph Analyzer
export { CodeGraphAnalyzer } from './codeGraphAnalyzer.js';

// Learning System
export { LearningSystem } from './learningSystem.js';

// Context Pack Builder
export {
  buildContextPack,
  buildHybridContextPack,
  computeFingerprint,
} from './context.js';
export type {
  MemoryProvider,
  DocumentProvider,
  EmbeddingProvider as ContextEmbeddingProvider,
  ChunkProvider,
} from './context.js';

// Intelligence Planner
export { IntelligencePlanner } from './intelligencePlanner.js';
export type { PlannerMemoryProvider } from './intelligencePlanner.js';

// ReAct Agent
export { ReActAgent } from './react.js';
export type { ReActMemoryProvider } from './react.js';

// MCP Tools
export {
  searchHybridContextPack,
  searchContextFingerprint,
  codeSearch,
  hybridContextPackInputSchema,
  contextFingerprintInputSchema,
  codeSearchInputSchema,
} from './tools.js';
export type {
  HybridContextPackInput,
  ContextFingerprintInput,
  CodeSearchInput,
  TaskMetadataProvider,
  ToolProviders,
} from './tools.js';
