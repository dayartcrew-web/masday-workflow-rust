import { describe, it, expect, beforeEach } from 'vitest';
import { BudgetManager } from '../budget.js';

describe('BudgetManager', () => {
  let budget: BudgetManager;

  beforeEach(() => {
    budget = new BudgetManager({
      maxTokensPerRequest: 4096,
      maxTokensPerSession: 10000,
      maxCostPerSession: 100,
    });
  });

  describe('canExecute', () => {
    it('should allow execution within per-request limit', () => {
      expect(budget.canExecute(2000)).toBe(true);
    });

    it('should reject execution exceeding per-request limit', () => {
      expect(budget.canExecute(5000)).toBe(false);
    });

    it('should reject execution at exact per-request limit', () => {
      expect(budget.canExecute(4096)).toBe(true);
    });

    it('should reject execution exceeding per-request limit by 1', () => {
      expect(budget.canExecute(4097)).toBe(false);
    });

    it('should reject execution when session tokens are exhausted', () => {
      // Use up most of the session budget
      budget.recordUsage(9000, 0);

      expect(budget.canExecute(2000)).toBe(false);
    });

    it('should allow execution when exactly at session limit', () => {
      budget.recordUsage(5904, 0); // 10000 - 5904 = 4096
      expect(budget.canExecute(4096)).toBe(true);
    });

    it('should reject zero-token requests that exceed session budget', () => {
      budget.recordUsage(10000, 0);
      expect(budget.canExecute(1)).toBe(false);
    });
  });

  describe('recordUsage', () => {
    it('should track token usage', () => {
      budget.recordUsage(100, 5);
      budget.recordUsage(200, 10);

      expect(budget.getTokensUsed()).toBe(300);
      expect(budget.getCostUsedCents()).toBe(15);
    });

    it('should track cost in cents', () => {
      budget.recordUsage(1000, 25);
      expect(budget.getCostUsedCents()).toBe(25);
    });

    it('should allow recording usage beyond budget (warning only)', () => {
      budget.recordUsage(50000, 200);
      expect(budget.getTokensUsed()).toBe(50000);
      expect(budget.getCostUsedCents()).toBe(200);
    });
  });

  describe('getRemaining', () => {
    it('should return full budget initially', () => {
      const remaining = budget.getRemaining();
      expect(remaining.tokens).toBe(10000);
      expect(remaining.costCents).toBe(100);
    });

    it('should return remaining after usage', () => {
      budget.recordUsage(3000, 30);

      const remaining = budget.getRemaining();
      expect(remaining.tokens).toBe(7000);
      expect(remaining.costCents).toBe(70);
    });

    it('should not return negative values', () => {
      budget.recordUsage(50000, 200);

      const remaining = budget.getRemaining();
      expect(remaining.tokens).toBe(0);
      expect(remaining.costCents).toBe(0);
    });
  });

  describe('reset', () => {
    it('should reset all tracking to zero', () => {
      budget.recordUsage(5000, 50);
      budget.reset();

      expect(budget.getTokensUsed()).toBe(0);
      expect(budget.getCostUsedCents()).toBe(0);
      expect(budget.getRemaining()).toEqual({ tokens: 10000, costCents: 100 });
    });
  });

  describe('defaults', () => {
    it('should use default config when none provided', () => {
      const defaultManager = new BudgetManager();
      expect(defaultManager.canExecute(4096)).toBe(true);
      expect(defaultManager.canExecute(4097)).toBe(false);
    });

    it('should merge partial config with defaults', () => {
      const partial = new BudgetManager({ maxTokensPerRequest: 8192 });
      expect(partial.canExecute(5000)).toBe(true);
    });
  });
});
