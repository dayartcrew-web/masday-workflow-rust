/**
 * Context fingerprinting
 *
 * Generates a deterministic SHA-256 hash fingerprint from structured
 * context data. Used to detect when context has changed between
 * sessions or execution steps.
 *
 * Unified from both msd-mcp and reborn versions.
 */

import { createHash } from "crypto";

export interface FingerprintInput {
  workflowId: string;
  planId: string;
  taskId: string;
  acceptanceCriteria: string[];
  requiredContext: string[];
  documentIds: string[];
  memoryIds: string[];
}

/**
 * Create a deterministic SHA-256 fingerprint from context input.
 *
 * All array fields are sorted before hashing to ensure stable output
 * regardless of insertion order.
 */
export function makeFingerprint(input: FingerprintInput): string {
  const sortable: Record<string, string[]> = {
    acceptanceCriteria: [...input.acceptanceCriteria].sort(),
    requiredContext: [...input.requiredContext].sort(),
    documentIds: [...input.documentIds].sort(),
    memoryIds: [...input.memoryIds].sort(),
  };

  const payload = JSON.stringify({
    workflowId: input.workflowId,
    planId: input.planId,
    taskId: input.taskId,
    ...sortable,
  });

  return createHash("sha256").update(payload).digest("hex");
}
