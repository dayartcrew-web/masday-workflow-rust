/**
 * Utility functions for the LLM module.
 */

/**
 * Estimate token count for a text string.
 * Accounts for CJK characters which typically use more tokens.
 */
export function estimateTokens(text: string): number {
  const cjkChars = (text.match(/[一-鿿぀-ゟ゠-ヿ]/g) || []).length;
  const otherChars = text.length - cjkChars;
  return Math.ceil(otherChars / 4 + cjkChars / 2);
}

/**
 * Execute a function with exponential backoff retry.
 * Does not retry on 4xx errors (except 429 rate limiting).
 */
export async function withRetry<T>(
  fn: () => Promise<T>,
  maxRetries = 2,
  baseDelayMs = 1000,
): Promise<T> {
  let lastError: unknown;

  for (let attempt = 0; attempt <= maxRetries; attempt++) {
    try {
      return await fn();
    } catch (error: unknown) {
      lastError = error;

      if (attempt < maxRetries && isRetryable(error)) {
        const delay = calculateDelay(baseDelayMs, attempt);
        await sleep(delay);
        continue;
      }

      throw error;
    }
  }

  // Should be unreachable, but satisfy the type checker
  throw lastError;
}

/**
 * Determine if an error is retryable.
 * 4xx errors (except 429) are not retryable.
 */
function isRetryable(error: unknown): boolean {
  if (error instanceof Error && 'statusCode' in error) {
    const status = (error as { statusCode: number }).statusCode;
    if (status >= 400 && status < 500 && status !== 429) {
      return false;
    }
  }
  return true;
}

/**
 * Calculate exponential backoff delay with jitter.
 */
function calculateDelay(baseDelayMs: number, attempt: number): number {
  const exponentialDelay = baseDelayMs * Math.pow(2, attempt);
  const jitter = Math.random() * baseDelayMs;
  return exponentialDelay + jitter;
}

/**
 * Sleep for a given number of milliseconds.
 */
function sleep(ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms));
}
