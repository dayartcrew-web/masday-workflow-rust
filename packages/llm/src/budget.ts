/**
 * Budget-based execution control for LLM usage.
 * Tracks token consumption and cost to enforce session limits.
 */

import { createLogger } from '@mcp-rebuild/core';

const logger = createLogger('llm:budget');

export interface BudgetConfig {
  maxTokensPerRequest: number;
  maxTokensPerSession: number;
  maxCostPerSession: number; // in USD cents
}

const DEFAULT_CONFIG: BudgetConfig = {
  maxTokensPerRequest: 4096,
  maxTokensPerSession: 100000,
  maxCostPerSession: 100, // $1.00
};

export class BudgetManager {
  private readonly config: BudgetConfig;
  private tokensUsed = 0;
  private costUsedCents = 0;

  constructor(config?: Partial<BudgetConfig>) {
    this.config = { ...DEFAULT_CONFIG, ...config };
  }

  /** Check whether an estimated token usage is within budget. */
  canExecute(estimatedTokens: number): boolean {
    if (estimatedTokens > this.config.maxTokensPerRequest) {
      logger.debug(
        { estimated: estimatedTokens, max: this.config.maxTokensPerRequest },
        'Budget check: exceeds per-request token limit',
      );
      return false;
    }

    const remainingTokens = this.config.maxTokensPerSession - this.tokensUsed;
    if (estimatedTokens > remainingTokens) {
      logger.debug(
        { estimated: estimatedTokens, remaining: remainingTokens },
        'Budget check: exceeds remaining session tokens',
      );
      return false;
    }

    return true;
  }

  /** Record actual token usage and cost after an LLM call. */
  recordUsage(tokensUsed: number, costCents: number): void {
    this.tokensUsed += tokensUsed;
    this.costUsedCents += costCents;

    logger.debug(
      {
        tokensUsed,
        costCents,
        totalTokens: this.tokensUsed,
        totalCostCents: this.costUsedCents,
        maxTokens: this.config.maxTokensPerSession,
        maxCostCents: this.config.maxCostPerSession,
      },
      'Budget: usage recorded',
    );

    if (this.costUsedCents >= this.config.maxCostPerSession) {
      logger.warn(
        { totalCostCents: this.costUsedCents, maxCostCents: this.config.maxCostPerSession },
        'Budget: session cost limit reached',
      );
    }
  }

  /** Get remaining budget for the current session. */
  getRemaining(): { tokens: number; costCents: number } {
    return {
      tokens: Math.max(0, this.config.maxTokensPerSession - this.tokensUsed),
      costCents: Math.max(0, this.config.maxCostPerSession - this.costUsedCents),
    };
  }

  /** Reset all budget tracking for a new session. */
  reset(): void {
    this.tokensUsed = 0;
    this.costUsedCents = 0;
    logger.debug('Budget: reset');
  }

  /** Get current total tokens used. */
  getTokensUsed(): number {
    return this.tokensUsed;
  }

  /** Get current total cost in cents. */
  getCostUsedCents(): number {
    return this.costUsedCents;
  }
}
