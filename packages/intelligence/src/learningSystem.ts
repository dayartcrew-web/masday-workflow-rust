import { EventBus } from '@mcp-rebuild/core';
import type { LearningMetric, LearningData, CodeUsagePattern } from './types.js';
import { createLogger } from '@mcp-rebuild/core';
import { promises as fs } from 'fs';
import path from 'path';

const logger = createLogger('LearningSystem');

const LEARNING_FILE = 'data/learning.json';

export class LearningSystem {
  private _eventBus: EventBus;
  private data: LearningData;

  constructor(eventBus: EventBus) {
    this._eventBus = eventBus;
    this.data = {
      metrics: [],
      patterns: [],
      commonErrors: new Map(),
      bestPractices: new Map(),
    };

    // Load existing learning data
    this.loadLearningData();
  }

  private async loadLearningData(): Promise<void> {
    try {
      const filePath = path.join(process.cwd(), LEARNING_FILE);
      await fs.mkdir(path.dirname(filePath), { recursive: true });

      const content = await fs.readFile(filePath, 'utf-8');
      const data = JSON.parse(content) as LearningData;

      this.data = {
        metrics: data.metrics || [],
        patterns: data.patterns || [],
        commonErrors: new Map(Object.entries(data.commonErrors || {})),
        bestPractices: new Map(Object.entries(data.bestPractices || {})),
      };

      logger.info(`Loaded learning data: ${this.data.metrics.length} metrics, ${this.data.patterns.length} patterns`);
    } catch {
      logger.warn('No existing learning data, starting fresh');
    }
  }

  async recordMetric(metric: LearningMetric): Promise<void> {
    this.data = {
      ...this.data,
      metrics: [...this.data.metrics, metric],
    };
    await this.saveLearningData();
    logger.info(`Recorded metric: ${metric.skill} (success: ${metric.success})`);
  }

  async recordPattern(pattern: CodeUsagePattern): Promise<void> {
    this.data = {
      ...this.data,
      patterns: [...this.data.patterns, pattern],
    };
    await this.saveLearningData();
    logger.info(`Recorded pattern: ${pattern.pattern} (count: ${pattern.count})`);
  }

  async recordError(skill: string, error: string): Promise<void> {
    const key = `${skill}:${error}`;
    const count = (this.data.commonErrors.get(key) || 0) + 1;
    this.data = {
      ...this.data,
      commonErrors: new Map(this.data.commonErrors).set(key, count),
    };
    await this.saveLearningData();
    logger.info(`Recorded error: ${key} (count: ${count})`);
  }

  async recordBestPractice(skill: string, practice: unknown): Promise<void> {
    this.data = {
      ...this.data,
      bestPractices: new Map(this.data.bestPractices).set(skill, practice),
    };
    await this.saveLearningData();
    logger.info(`Recorded best practice: ${skill}`);
  }

  getOptimizationSuggestions(skill: string, _context: string): Array<{ type: string; message: string }> {
    const suggestions: Array<{ type: string; message: string }> = [];

    const skillMetrics = this.data.metrics.filter(m => m.skill === skill);

    if (skillMetrics.length === 0) return suggestions;

    const successRate = skillMetrics.filter(m => m.success).length / skillMetrics.length;
    const avgDuration = skillMetrics.reduce((sum, m) => sum + m.duration, 0) / skillMetrics.length;

    if (successRate < 0.8) {
      suggestions.push({
        type: 'retry_pattern',
        message: `Success rate is ${Math.round(successRate * 100)}%, consider enabling retry logic`,
      });
    }

    if (avgDuration > 5000) {
      suggestions.push({
        type: 'performance',
        message: `Average duration is ${Math.round(avgDuration)}ms, consider optimizing skill implementation`,
      });
    }

    const skillErrors = this.data.commonErrors.get(skill);
    if (skillErrors && skillErrors > 5) {
      suggestions.push({
        type: 'error_analysis',
        message: `Skill has ${skillErrors} recorded errors, review implementation`,
      });
    }

    return suggestions;
  }

  getSkillMetrics(skill: string): {
    total: number;
    success: number;
    failed: number;
    avgDuration: number;
    successRate: number;
  } {
    const skillMetrics = this.data.metrics.filter(m => m.skill === skill);

    if (skillMetrics.length === 0) {
      return { total: 0, success: 0, failed: 0, avgDuration: 0, successRate: 0 };
    }

    const success = skillMetrics.filter(m => m.success).length;
    const failed = skillMetrics.filter(m => !m.success).length;
    const avgDuration = skillMetrics.reduce((sum, m) => sum + m.duration, 0) / skillMetrics.length;
    const successRate = success / skillMetrics.length;

    return {
      total: skillMetrics.length,
      success,
      failed,
      avgDuration,
      successRate,
    };
  }

  getAllMetrics(): LearningMetric[] {
    return [...this.data.metrics];
  }

  getAllPatterns(): CodeUsagePattern[] {
    return [...this.data.patterns];
  }

  getCommonErrors(): Map<string, number> {
    return new Map(this.data.commonErrors);
  }

  getBestPractices(): Map<string, unknown> {
    return new Map(this.data.bestPractices);
  }

  private async saveLearningData(): Promise<void> {
    const filePath = path.join(process.cwd(), LEARNING_FILE);
    await fs.mkdir(path.dirname(filePath), { recursive: true });

    const serializable = {
      metrics: this.data.metrics,
      patterns: this.data.patterns,
      commonErrors: Object.fromEntries(this.data.commonErrors),
      bestPractices: Object.fromEntries(this.data.bestPractices),
    };

    const content = JSON.stringify(serializable, null, 2);
    await fs.writeFile(filePath, content, 'utf-8');

    logger.debug(`Saved learning data to ${filePath}`);
  }
}
