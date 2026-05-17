// ============================================================
// Zod request validation middleware
// ============================================================

import { z, ZodSchema } from 'zod';
import { readBody, parseJsonBody, APIError } from './utils';
import type { IncomingMessage } from 'http';

export interface ValidatedRequest {
  body: Record<string, unknown>;
}

/** Validate request body against a Zod schema */
export async function validateBody<T>(
  req: IncomingMessage,
  schema: ZodSchema<T>,
): Promise<T> {
  const raw = await readBody(req);
  const parsed = parseJsonBody(raw);

  const result = schema.safeParse(parsed);
  if (!result.success) {
    const errors = result.error.issues.map(
      (issue) => `${issue.path.join('.')}: ${issue.message}`,
    );
    throw new APIError(422, 'VALIDATION_ERROR', `Validation failed: ${errors.join('; ')}`);
  }

  return result.data;
}

/** Validate query parameters against a Zod schema */
export function validateQuery<T>(
  url: URL,
  schema: ZodSchema<T>,
): T {
  const query: Record<string, unknown> = {};
  for (const [key, value] of url.searchParams) {
    query[key] = value;
  }

  const result = schema.safeParse(query);
  if (!result.success) {
    const errors = result.error.issues.map(
      (issue) => `${issue.path.join('.')}: ${issue.message}`,
    );
    throw new APIError(422, 'VALIDATION_ERROR', `Query validation failed: ${errors.join('; ')}`);
  }

  return result.data;
}

// --- Common reusable schemas ---

export const paginationSchema = z.object({
  page: z.coerce.number().int().min(1).default(1),
  limit: z.coerce.number().int().min(1).max(100).default(20),
});

export const idSchema = z.string().min(1);

export const emailSchema = z.string().email();

// --- Auth schemas ---

export const loginSchema = z.object({
  email: emailSchema,
  name: z.string().min(1).max(200),
});

export const tokenSchema = z.object({
  token: z.string().min(1),
});

// --- Workflow schemas ---

export const createWorkflowSchema = z.object({
  name: z.string().min(1).max(500),
  description: z.string().max(2000).default(''),
  metadata: z.record(z.unknown()).optional(),
});

export const addTaskSchema = z.object({
  name: z.string().min(1).max(500),
  agent: z.string().min(1).max(200),
  skill: z.string().min(1).max(200),
  dependencies: z.array(z.string()).default([]),
  input: z.unknown().optional(),
});

export const saveProgressSchema = z.object({
  agentName: z.string().min(1).max(200),
  progressNote: z.string().min(1).max(5000),
  evidence: z.array(z.string()).optional(),
  statusBefore: z.string().optional(),
  statusAfter: z.string().optional(),
});

export const createPlanSchema = z.object({
  tasks: z.array(z.object({
    title: z.string().min(1).max(500),
    priority: z.enum(['CRITICAL', 'HIGH', 'MEDIUM', 'LOW']).optional(),
    ownerAgent: z.string().max(200).optional(),
    acceptanceCriteria: z.array(z.string()).optional(),
    requiredContext: z.array(z.string()).optional(),
  })).min(1),
});

// --- Memory schemas ---

export const storeMemorySchema = z.object({
  memoryType: z.enum(['decision', 'artifact', 'learning', 'blocker']),
  summary: z.string().min(1).max(500),
  content: z.string().min(1).max(50000),
  importance: z.number().min(0).max(1).optional(),
  taskId: z.string().optional(),
  workflowId: z.string().optional(),
});

export const updateMemorySchema = z.object({
  summary: z.string().min(1).max(500).optional(),
  content: z.string().min(1).max(50000).optional(),
  importance: z.number().min(0).max(1).optional(),
});

// --- Search schemas ---

export const hybridSearchSchema = z.object({
  workflowId: z.string().min(1),
  planId: z.string().min(1),
  taskId: z.string().min(1),
  cwd: z.string().optional(),
});

export const fingerprintSchema = z.object({
  workflowId: z.string().min(1),
  planId: z.string().min(1),
  taskId: z.string().min(1),
});

export const codeSearchSchema = z.object({
  query: z.string().min(1).max(500),
  glob: z.string().optional(),
  type: z.string().optional(),
  limit: z.number().int().min(1).max(100).optional(),
});

// --- Policy schemas ---

export const checkReadinessSchema = z.object({
  sessionKey: z.string().min(1),
});

export const validateExecutionSchema = z.object({
  workflowId: z.string().min(1),
  taskId: z.string().min(1),
  sessionKey: z.string().min(1),
});

export const validateCompletionSchema = z.object({
  workflowId: z.string().min(1),
  taskId: z.string().min(1),
  acceptanceCriteria: z.array(z.string()),
  evidence: z.array(z.string()),
});

export const validateParallelSchema = z.object({
  workflowId: z.string().min(1),
  branchResults: z.array(z.object({
    branchKey: z.string(),
    status: z.enum(['done', 'failed']),
    output: z.unknown().optional(),
  })),
  mergeStrategy: z.enum(['sequential', 'interleave', 'vote']).optional(),
});

export const detectDriftSchema = z.object({
  workflowId: z.string().min(1),
  originalScope: z.string().min(1),
  currentInput: z.string().min(1),
  threshold: z.number().min(0).max(1).optional(),
});

// --- Capability schemas ---

export const createAgentSchema = z.object({
  name: z.string().min(1).max(200),
  role: z.enum(['planner', 'executor', 'critic', 'reflector', 'router']),
  projectRoot: z.string().min(1),
  description: z.string().max(1000).optional(),
});

export const createSkillSchema = z.object({
  name: z.string().min(1).max(200),
  projectRoot: z.string().min(1),
  description: z.string().max(1000).optional(),
  agentName: z.string().optional(),
});

export const matchAgentSchema = z.object({
  taskType: z.string().min(1),
  requiredTools: z.array(z.string()).optional(),
  projectRoot: z.string().min(1),
});

// --- Chat schemas ---

export const chatSchema = z.object({
  message: z.string().min(1).max(50000),
  sessionId: z.string().optional(),
  model: z.string().optional(),
  temperature: z.number().min(0).max(2).optional(),
});

export const reactSchema = z.object({
  goal: z.string().min(1).max(50000),
  maxIterations: z.number().int().min(1).max(20).optional(),
  sessionId: z.string().optional(),
});

// --- Provider schemas ---

export const testProviderSchema = z.object({
  model: z.string().optional(),
  prompt: z.string().max(1000).optional(),
});
