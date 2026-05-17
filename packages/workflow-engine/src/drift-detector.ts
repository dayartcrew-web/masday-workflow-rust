/**
 * Scope Drift Detection (unified)
 *
 * Detects when task output or progress notes drift away from
 * the original task scope defined by title, acceptance criteria,
 * and required context.
 *
 * Uses Jaccard-like overlap scoring from the reborn version
 * with the same public interface from msd-mcp.
 */

export interface DriftResult {
  drift: boolean;
  outputScore: number;
  progressScore: number;
  reason: string;
}

/** Tokenize text into lowercase word tokens for comparison. */
function tokenize(text: string): Set<string> {
  return new Set(
    text
      .toLowerCase()
      .replace(/[^a-z0-9\s]/g, " ")
      .split(/\s+/)
      .filter((t) => t.length > 2),
  );
}

/** Compute Jaccard-like overlap score between two token sets (0..1). */
function overlapScore(a: Set<string>, b: Set<string>): number {
  if (a.size === 0 && b.size === 0) return 1;
  if (a.size === 0 || b.size === 0) return 0;

  let matches = 0;
  for (const token of a) {
    if (b.has(token)) matches++;
  }
  return matches / a.size;
}

const DRIFT_THRESHOLD = 0.2;

/**
 * Detect scope drift by comparing output and optional progress note
 * against the combined context tokens from title, acceptance criteria,
 * and required context.
 */
export function detectScopeDrift(input: {
  taskTitle: string;
  acceptanceCriteria: string[];
  requiredContext: string[];
  outputText: string;
  progressNote?: string;
}): DriftResult {
  const contextText = [
    input.taskTitle,
    ...input.acceptanceCriteria,
    ...input.requiredContext,
  ].join(" ");

  const contextTokens = tokenize(contextText);
  const outputTokens = tokenize(input.outputText);

  const outputScore = overlapScore(contextTokens, outputTokens);

  let progressScore = 1;
  if (input.progressNote) {
    const progressTokens = tokenize(input.progressNote);
    progressScore = overlapScore(contextTokens, progressTokens);
  }

  const drift =
    outputScore < DRIFT_THRESHOLD && progressScore < DRIFT_THRESHOLD;

  let reason: string;
  if (drift) {
    reason = `Both output (${outputScore.toFixed(2)}) and progress (${progressScore.toFixed(2)}) scores are below threshold (${DRIFT_THRESHOLD})`;
  } else if (outputScore < DRIFT_THRESHOLD) {
    reason = `Output score (${outputScore.toFixed(2)}) is low but progress score (${progressScore.toFixed(2)}) is acceptable`;
  } else {
    reason = `Output score (${outputScore.toFixed(2)}) is above threshold (${DRIFT_THRESHOLD})`;
  }

  return { drift, outputScore, progressScore, reason };
}
