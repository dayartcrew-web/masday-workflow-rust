// ============================================================
// Token tracking — persists to PostgreSQL via Prisma, in-memory fallback
// ============================================================

import { estimateTokens } from './token-estimator.js';
import { getMetrics } from './metrics.js';
import { MetricNames } from './metrics.js';
import { createLogger } from './logger.js';

const logger = createLogger('token-tracker');

interface TokenRecord {
  source: string;
  route: string;
  model?: string;
  promptTokens: number;
  completionTokens: number;
  totalTokens: number;
  latencyMs: number;
  metadata?: Record<string, unknown>;
  createdAt: Date;
}

// In-memory buffer for when DB is unavailable
const memoryBuffer: TokenRecord[] = [];
const routeTokenMap: Map<string, number> = new Map();
// eslint-disable-next-line @typescript-eslint/no-explicit-any
let prismaClient: any = null;

export function setPrismaClient(client: unknown): void {
  prismaClient = client;
}

export function trackTokens(route: string, requestBody: unknown, responseBody: unknown): number {
  const inputStr = typeof requestBody === 'string' ? requestBody : JSON.stringify(requestBody ?? '');
  const outputStr = typeof responseBody === 'string' ? responseBody : JSON.stringify(responseBody ?? '');

  const inputTokens = estimateTokens(inputStr);
  const outputTokens = estimateTokens(outputStr);
  const total = inputTokens + outputTokens;

  getMetrics().increment(MetricNames.LLM_TOKENS_USED, total, { route });

  const prev = routeTokenMap.get(route) ?? 0;
  routeTokenMap.set(route, prev + total);

  persistRecord({
    source: 'estimated',
    route,
    promptTokens: inputTokens,
    completionTokens: outputTokens,
    totalTokens: total,
    latencyMs: 0,
    createdAt: new Date(),
  });

  return total;
}

export interface LLMTokenData {
  route: string;
  model: string;
  promptTokens: number;
  completionTokens: number;
  totalTokens: number;
  latencyMs: number;
}

export function trackLLMTokens(data: LLMTokenData): void {
  getMetrics().increment(MetricNames.LLM_TOKENS_USED, data.totalTokens, { route: data.route, model: data.model });

  const prev = routeTokenMap.get(data.route) ?? 0;
  routeTokenMap.set(data.route, prev + data.totalTokens);

  persistRecord({
    source: 'llm',
    route: data.route,
    model: data.model,
    promptTokens: data.promptTokens,
    completionTokens: data.completionTokens,
    totalTokens: data.totalTokens,
    latencyMs: data.latencyMs,
    createdAt: new Date(),
  });
}

function persistRecord(record: TokenRecord): void {
  if (prismaClient?.tokenUsage) {
    prismaClient.tokenUsage.create({
      data: {
        source: record.source,
        route: record.route,
        model: record.model,
        promptTokens: record.promptTokens,
        completionTokens: record.completionTokens,
        totalTokens: record.totalTokens,
        latencyMs: record.latencyMs,
        metadata: record.metadata ?? {},
        createdAt: record.createdAt,
      },
    }).catch((err: unknown) => {
      logger.warn({ err: String(err) }, 'Failed to persist token record to DB, buffering in memory');
      memoryBuffer.push(record);
    });
  } else {
    memoryBuffer.push(record);
  }
}

export function getRouteTokenBreakdown(): ReadonlyMap<string, number> {
  return new Map(routeTokenMap);
}

export function resetTokenTracking(): void {
  routeTokenMap.clear();
  memoryBuffer.length = 0;
}

export interface TokenUsageAggregation {
  byRoute: Record<string, { totalTokens: number; promptTokens: number; completionTokens: number; count: number }>;
  byModel: Record<string, { totalTokens: number; promptTokens: number; completionTokens: number; count: number }>;
  bySource: Record<string, { totalTokens: number; count: number }>;
  totalTokens: number;
  totalRequests: number;
}

export function getMemoryBufferSnapshot(): TokenRecord[] {
  return [...memoryBuffer];
}
