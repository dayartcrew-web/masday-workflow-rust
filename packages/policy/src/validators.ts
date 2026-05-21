/**
 * Policy Validators
 *
 * Validates execution, completion, and parallel completion
 * using SessionManager, ReviewManager, and DriftDetector.
 */

import type { StorageBackend } from '@mcp-rebuild/store';
import { SessionManager } from '@mcp-rebuild/workflow-engine';
import { detectScopeDrift } from '@mcp-rebuild/workflow-engine';
import { createLogger } from '@mcp-rebuild/core';
import { ReviewManager } from './review-manager.js';
import { ParallelExecutor } from './parallel-executor.js';

const logger = createLogger('PolicyValidator');

export interface ValidateExecutionInput {
  workflowId: string;
  taskId: string;
  sessionKey: string;
}

export interface ValidateCompletionInput {
  workflowId: string;
  taskId: string;
  outputText?: string;
  taskTitle?: string;
  acceptanceCriteria?: string[];
  requiredContext?: string[];
  testEvidence?: { testFiles: string[]; testsPassed: boolean; coveragePercent?: number };
}

export interface ValidateParallelCompletionInput {
  workflowId: string;
  taskId: string;
  sessionKey: string;
  outputText?: string;
  taskTitle?: string;
  acceptanceCriteria?: string[];
  requiredContext?: string[];
}

export interface PolicyResult {
  ok: boolean;
  reason?: string;
  missing?: string[];
  driftDetected?: boolean;
  driftScore?: number;
  tddRequired?: boolean;
  testsVerified?: boolean;
}

export class PolicyValidator {
  private sessionManager: SessionManager;
  private reviewManager: ReviewManager;
  private parallelExecutor: ParallelExecutor;

  constructor(storage: StorageBackend) {
    this.sessionManager = new SessionManager(storage);
    this.reviewManager = new ReviewManager(storage);
    this.parallelExecutor = new ParallelExecutor(storage);

    this.sessionManager.init();
    this.reviewManager.init();
    this.parallelExecutor.init();
  }

  /**
   * Validate that execution is allowed for the given session.
   *
   * Checks that the session has workflow, plan, task, and context loaded.
   */
  async validateExecution(input: ValidateExecutionInput): Promise<PolicyResult> {
    const { ready, missing } = await this.sessionManager.checkReadiness(input.sessionKey);

    if (!ready) {
      logger.warn(
        { sessionKey: input.sessionKey, missing },
        'Execution blocked: session not ready',
      );
      return {
        ok: false,
        reason: `Session not ready. Missing: ${missing.join(', ')}`,
        missing,
      };
    }

    logger.info(
      { sessionKey: input.sessionKey, workflowId: input.workflowId, taskId: input.taskId },
      'Execution validated: session ready',
    );

    return { ok: true };
  }

  /**
   * Validate that task completion is allowed.
   *
   * Checks:
   * 1. Latest review is APPROVED
   * 2. If task requires TDD: tests exist, tests passed, reviewer verified tests
   * 3. No scope drift in output text
   */
  async validateCompletion(input: ValidateCompletionInput): Promise<PolicyResult> {
    // Check review approval
    const latestReview = await this.reviewManager.getLatestReview(
      input.workflowId,
      input.taskId,
    );

    if (!latestReview) {
      logger.warn(
        { workflowId: input.workflowId, taskId: input.taskId },
        'Completion blocked: no review found',
      );
      return {
        ok: false,
        reason: 'No review found for this task. Submit a review before completion.',
      };
    }

    if (latestReview.decision !== 'APPROVED') {
      logger.warn(
        { workflowId: input.workflowId, taskId: input.taskId, decision: latestReview.decision },
        'Completion blocked: latest review not approved',
      );
      return {
        ok: false,
        reason: `Latest review decision is ${latestReview.decision}, not APPROVED.`,
      };
    }

    // TDD gate: if testEvidence is provided, validate test completeness
    if (input.testEvidence) {
      const hasTests = input.testEvidence.testFiles.length > 0;
      const testsPassed = input.testEvidence.testsPassed === true;

      if (!hasTests) {
        logger.warn(
          { workflowId: input.workflowId, taskId: input.taskId },
          'Completion blocked: no test files in testEvidence',
        );
        return {
          ok: false,
          reason: 'Task requires TDD but no test files found in testEvidence.',
          tddRequired: true,
          testsVerified: false,
        };
      }

      if (!testsPassed) {
        logger.warn(
          { workflowId: input.workflowId, taskId: input.taskId },
          'Completion blocked: tests not passing',
        );
        return {
          ok: false,
          reason: 'Task requires TDD but tests have not passed.',
          tddRequired: true,
          testsVerified: false,
        };
      }

      if (!((latestReview as unknown as Record<string, unknown>).testsVerified)) {
        logger.warn(
          { workflowId: input.workflowId, taskId: input.taskId },
          'Completion blocked: review did not verify tests',
        );
        return {
          ok: false,
          reason: 'Review must verify tests (testsVerified: true) before completing TDD task.',
          tddRequired: true,
          testsVerified: false,
        };
      }
    }

    // Check for scope drift if output text and task context are provided
    if (input.outputText && input.taskTitle) {
      const driftResult = detectScopeDrift({
        taskTitle: input.taskTitle,
        acceptanceCriteria: input.acceptanceCriteria ?? [],
        requiredContext: input.requiredContext ?? [],
        outputText: input.outputText,
      });

      if (driftResult.drift) {
        logger.warn(
          {
            workflowId: input.workflowId,
            taskId: input.taskId,
            outputScore: driftResult.outputScore,
            progressScore: driftResult.progressScore,
          },
          'Scope drift detected',
        );
        return {
          ok: false,
          reason: driftResult.reason,
          driftDetected: true,
          driftScore: driftResult.outputScore,
        };
      }
    }

    logger.info(
      { workflowId: input.workflowId, taskId: input.taskId },
      'Completion validated: review approved, TDD checks passed',
    );

    return { ok: true };
  }

  /**
   * Validate that parallel task completion is allowed.
   *
   * Checks:
   * 1. Execution mode is parallel in the session
   * 2. All parallel branches are completed
   * 3. Synthesis is ready
   * 4. Verification is ready
   * 5. Latest review is APPROVED
   */
  async validateParallelCompletion(
    input: ValidateParallelCompletionInput,
  ): Promise<PolicyResult> {
    // Check session readiness and mode
    const session = await this.sessionManager.getOrCreateState(input.sessionKey);

    if (session.executionMode !== 'parallel') {
      return {
        ok: false,
        reason: `Expected parallel execution mode, got: ${session.executionMode ?? 'none'}`,
      };
    }

    // Check synthesis and verification readiness
    if (!session.synthesisReady) {
      return {
        ok: false,
        reason: 'Synthesis is not ready. Complete synthesis before parallel completion.',
      };
    }

    if (!session.verificationReady) {
      return {
        ok: false,
        reason: 'Verification is not ready. Complete verification before parallel completion.',
      };
    }

    // Check all branches completed
    const allCompleted = await this.parallelExecutor.allBranchesCompleted(
      input.workflowId,
      input.taskId,
    );

    if (!allCompleted) {
      return {
        ok: false,
        reason: 'Not all parallel branches are completed.',
      };
    }

    // Check review approval
    const latestReview = await this.reviewManager.getLatestReview(
      input.workflowId,
      input.taskId,
    );

    if (!latestReview || latestReview.decision !== 'APPROVED') {
      const decision = latestReview?.decision ?? 'none';
      return {
        ok: false,
        reason: `Latest review decision is ${decision}, expected APPROVED.`,
      };
    }

    // Check for scope drift if output text and task context are provided
    if (input.outputText && input.taskTitle) {
      const driftResult = detectScopeDrift({
        taskTitle: input.taskTitle,
        acceptanceCriteria: input.acceptanceCriteria ?? [],
        requiredContext: input.requiredContext ?? [],
        outputText: input.outputText,
      });

      if (driftResult.drift) {
        logger.warn(
          {
            workflowId: input.workflowId,
            taskId: input.taskId,
            outputScore: driftResult.outputScore,
          },
          'Scope drift detected in parallel completion',
        );
        return {
          ok: false,
          reason: driftResult.reason,
          driftDetected: true,
          driftScore: driftResult.outputScore,
        };
      }
    }

    logger.info(
      { workflowId: input.workflowId, taskId: input.taskId },
      'Parallel completion validated',
    );

    return { ok: true };
  }
}
