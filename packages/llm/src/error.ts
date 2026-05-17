/**
 * Custom error class for LLM operations.
 */

export class LLMError extends Error {
  public readonly provider: string;
  public readonly statusCode?: number;
  public readonly retryable: boolean;

  constructor(options: {
    message: string;
    provider: string;
    statusCode?: number;
    retryable?: boolean;
    cause?: Error;
  }) {
    super(options.message);
    this.name = 'LLMError';
    this.provider = options.provider;
    this.statusCode = options.statusCode;
    this.retryable = options.retryable ?? false;
    if (options.cause) {
      this.cause = options.cause;
    }
  }
}
