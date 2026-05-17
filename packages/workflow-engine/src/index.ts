/**
 * @mcp-rebuild/workflow-engine
 *
 * Unified workflow engine combining:
 *
 * 1. MSD-MCP Business Logic (Primary API) - Pure functions for MCP tool handlers
 *    that use Prisma for database access.
 *
 * 2. Reborn 3-Tier Engine (Advanced API) - In-memory workflow engine with
 *    state machine, DAG executor, planner, and multi-agent orchestration.
 *    Uses StorageBackend for persistence.
 */

// ─── Skill Executor Interface ───
export type { ISkillRegistry, SkillExecutor } from "./skill-executor.js";

// ─── MSD-MCP Business Logic (Primary API / Prisma-backed) ───

export {
  getActiveWorkflow,
  getPlan,
  getCurrentTask,
  listWorkflows as listWorkflowsDb,
  getResumeSuggestion,
} from "./workflow.js";

export { createWorkflow } from "./workflow-create.js";

export { createPlan } from "./plan.js";

export { startTask, listTasks, completeTask } from "./task.js";

export { saveProgress } from "./progress.js";

export { submitReview, getLatestReview } from "./review.js";

export {
  validateCompletion,
  validateExecution,
  validateParallelCompletion,
} from "./policy.js";

export {
  getOrCreateSessionState,
  patchSessionState,
} from "./session.js";

export { buildContextPack, buildHybridContextPack } from "./retrieval.js";

export { makeFingerprint } from "./fingerprint.js";
export type { FingerprintInput } from "./fingerprint.js";

export { detectScopeDrift } from "./drift-detector.js";
export type { DriftResult } from "./drift-detector.js";

export {
  vectorSearchMemories,
  vectorSearchContext,
} from "./vector-search.js";

export {
  MockEmbeddingProvider,
  OpenAIEmbeddingProvider,
  createEmbeddingProvider,
} from "./embedding.js";
export type { EmbeddingProvider } from "./embedding.js";

export { logRetrieval } from "./audit.js";

export {
  initMsdDir,
  writeLocalState,
  readLocalState,
  writeArtifact,
  listArtifacts,
  syncToDb,
  syncFromDb,
} from "./local-state.js";
export type { SyncResult } from "./local-state.js";

export {
  setExecutionMode,
  createParallelBranches,
  listParallelBranches,
  completeParallelBranch,
  markSynthesisReady,
  markVerificationReady,
} from "./parallel.js";

// ─── Reborn 3-Tier Engine (Advanced API / StorageBackend-backed) ───

export { StateMachine } from "./stateMachine.js";

export {
  BaseWorkflowEngine,
} from "./baseWorkflowEngine.js";
export type {
  WorkflowStatusResult,
  BaseWorkflowEngineConfig,
} from "./baseWorkflowEngine.js";

export { WorkflowEngine } from "./workflowEngine.js";

export {
  EnhancedWorkflowEngine,
} from "./enhancedWorkflowEngine.js";
export type { EnhancedWorkflowEngineConfig } from "./enhancedWorkflowEngine.js";

export { OrchestratingEngine } from "./orchestratingEngine.js";
export type { OrchestratingEngineConfig } from "./orchestratingEngine.js";

export { Planner } from "./planner.js";
export type { PlanResult, PlannerConfig } from "./planner.js";

export { DAGExecutor } from "./dagExecutor.js";
export type {
  DAGExecutorConfig,
  ExecutionResult,
} from "./dagExecutor.js";

export { TaskQueue } from "./taskQueue.js";
export type { QueueItem, TaskQueueConfig } from "./taskQueue.js";

export { SessionManager } from "./session-manager.js";

// ─── Agent Coordination (inlined from reborn agents package) ───

export {
  AgentWorker,
  AgentCoordinator,
  SkillRouter,
} from "./agents.js";
export type {
  AgentStatus,
  AgentType,
  AgentWorkerConfig,
  AgentMessage,
} from "./agents.js";
