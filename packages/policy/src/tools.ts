/**
 * Policy MCP Tool Business Logic
 *
 * Contains the core logic for 6 policy MCP tools.
 * These functions return plain objects; Phase 7 wraps them in MCP protocol.
 */

import type { StorageBackend } from '@mcp-rebuild/store';
import { SessionManager } from '@mcp-rebuild/workflow-engine';
import { makeFingerprint } from '@mcp-rebuild/workflow-engine';
import { detectScopeDrift } from '@mcp-rebuild/workflow-engine';
import { PolicyValidator } from './validators.js';
import type {
  PolicyResult,
  ValidateExecutionInput,
  ValidateCompletionInput,
  ValidateParallelCompletionInput,
} from './validators.js';

// --- Tool 1: policy.check_session_readiness ---

export interface CheckSessionReadinessInput {
  sessionKey: string;
}

export interface CheckSessionReadinessResult {
  ready: boolean;
  missing: string[];
  state: {
    sessionKey: string;
    workflowLoaded: boolean;
    planLoaded: boolean;
    taskLoaded: boolean;
    contextLoaded: boolean;
    reviewApproved: boolean;
    executionMode?: string;
  };
}

export function checkSessionReadiness(
  storage: StorageBackend,
  input: CheckSessionReadinessInput,
): Promise<CheckSessionReadinessResult> {
  const sessionManager = new SessionManager(storage);
  return sessionManager.getOrCreateState(input.sessionKey).then(async (state) => {
    const { ready, missing } = await sessionManager.checkReadiness(input.sessionKey);
    return {
      ready,
      missing,
      state: {
        sessionKey: state.sessionKey,
        workflowLoaded: state.workflowLoaded,
        planLoaded: state.planLoaded,
        taskLoaded: state.taskLoaded,
        contextLoaded: state.contextLoaded,
        reviewApproved: state.reviewApproved,
        executionMode: state.executionMode,
      },
    };
  });
}

// --- Tool 2: policy.validate_execution ---

export function validateExecution(
  storage: StorageBackend,
  input: ValidateExecutionInput,
): Promise<PolicyResult> {
  const validator = new PolicyValidator(storage);
  return validator.validateExecution(input);
}

// --- Tool 3: policy.validate_completion ---

export function validateCompletion(
  storage: StorageBackend,
  input: ValidateCompletionInput,
): Promise<PolicyResult> {
  const validator = new PolicyValidator(storage);
  return validator.validateCompletion(input);
}

// --- Tool 4: policy.require_context_refresh ---

export interface RequireContextRefreshInput {
  sessionKey: string;
  currentFingerprint: {
    workflowId: string;
    planId: string;
    taskId: string;
    acceptanceCriteria: string[];
    requiredContext: string[];
    documentIds: string[];
    memoryIds: string[];
  };
}

export interface RequireContextRefreshResult {
  refreshRequired: boolean;
  reason: string;
  previousFingerprint?: string;
  currentFingerprint: string;
}

export async function requireContextRefresh(
  storage: StorageBackend,
  input: RequireContextRefreshInput,
): Promise<RequireContextRefreshResult> {
  const sessionManager = new SessionManager(storage);
  const state = await sessionManager.getOrCreateState(input.sessionKey);

  const newFingerprint = makeFingerprint(input.currentFingerprint);
  const previousFingerprint = state.contextFingerprint;

  if (!previousFingerprint) {
    return {
      refreshRequired: true,
      reason: 'No previous fingerprint stored for this session',
      currentFingerprint: newFingerprint,
    };
  }

  if (previousFingerprint !== newFingerprint) {
    return {
      refreshRequired: true,
      reason: 'Context fingerprint has changed since last load',
      previousFingerprint,
      currentFingerprint: newFingerprint,
    };
  }

  return {
    refreshRequired: false,
    reason: 'Context fingerprint matches; no refresh needed',
    previousFingerprint,
    currentFingerprint: newFingerprint,
  };
}

// --- Tool 5: policy.detect_scope_drift ---

export interface DetectScopeDriftInput {
  taskTitle: string;
  acceptanceCriteria: string[];
  requiredContext: string[];
  outputText: string;
  progressNote?: string;
}

export interface DetectScopeDriftResult {
  drift: boolean;
  outputScore: number;
  progressScore: number;
  reason: string;
}

export function detectScopeDriftTool(
  _storage: StorageBackend,
  input: DetectScopeDriftInput,
): DetectScopeDriftResult {
  const result = detectScopeDrift({
    taskTitle: input.taskTitle,
    acceptanceCriteria: input.acceptanceCriteria,
    requiredContext: input.requiredContext,
    outputText: input.outputText,
    progressNote: input.progressNote,
  });

  return {
    drift: result.drift,
    outputScore: result.outputScore,
    progressScore: result.progressScore,
    reason: result.reason,
  };
}

// --- Tool 6: policy.validate_parallel_completion ---

export function validateParallelCompletion(
  storage: StorageBackend,
  input: ValidateParallelCompletionInput,
): Promise<PolicyResult> {
  const validator = new PolicyValidator(storage);
  return validator.validateParallelCompletion(input);
}
