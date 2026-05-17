/**
 * Guardrails - Input/output validation using Zod schemas.
 *
 * Provides structured validation for agent inputs and outputs,
 * with configurable length limits and descriptive error messages.
 */

import { z } from 'zod';
import { createLogger } from '@mcp-rebuild/core';
import type { GuardrailResult } from './types.js';

const logger = createLogger('intelligence:guardrails');

const DEFAULT_MAX_INPUT_LENGTH = 100_000;
const DEFAULT_MAX_OUTPUT_LENGTH = 100_000;

/**
 * Guardrails validates agent inputs and outputs against Zod schemas.
 *
 * Usage:
 *   const guardrails = new Guardrails(inputSchema, outputSchema);
 *   const inputResult = guardrails.validateInput(rawInput);
 *   if (!inputResult.valid) { throw new Error(inputResult.errors.join(', ')); }
 */
export class Guardrails<TInput = unknown, TOutput = unknown> {
  private readonly inputSchema: z.ZodType<TInput>;
  private readonly outputSchema: z.ZodType<TOutput>;
  private readonly maxInputLength: number;
  private readonly maxOutputLength: number;

  constructor(
    inputSchema: z.ZodType<TInput>,
    outputSchema: z.ZodType<TOutput>,
    options?: { maxInputLength?: number; maxOutputLength?: number },
  ) {
    this.inputSchema = inputSchema;
    this.outputSchema = outputSchema;
    this.maxInputLength = options?.maxInputLength ?? DEFAULT_MAX_INPUT_LENGTH;
    this.maxOutputLength = options?.maxOutputLength ?? DEFAULT_MAX_OUTPUT_LENGTH;
  }

  /** Validate agent input against the input schema. */
  validateInput(data: unknown): GuardrailResult<TInput> {
    // Length check for string inputs
    if (typeof data === 'string' && data.length > this.maxInputLength) {
      const error = `Input exceeds maximum length of ${this.maxInputLength} characters`;
      logger.warn({ length: data.length, max: this.maxInputLength }, error);
      return { valid: false, data: null, errors: [error] };
    }

    const result = this.inputSchema.safeParse(data);

    if (result.success) {
      return { valid: true, data: result.data, errors: [] };
    }

    const errors = result.error.issues.map(
      issue => `${issue.path.join('.')}: ${issue.message}`,
    );

    logger.warn({ errors }, 'Input validation failed');
    return { valid: false, data: null, errors };
  }

  /** Validate agent output against the output schema. */
  validateOutput(data: unknown): GuardrailResult<TOutput> {
    // Length check for string outputs
    if (typeof data === 'string' && data.length > this.maxOutputLength) {
      const error = `Output exceeds maximum length of ${this.maxOutputLength} characters`;
      logger.warn({ length: data.length, max: this.maxOutputLength }, error);
      return { valid: false, data: null, errors: [error] };
    }

    const result = this.outputSchema.safeParse(data);

    if (result.success) {
      return { valid: true, data: result.data, errors: [] };
    }

    const errors = result.error.issues.map(
      issue => `${issue.path.join('.')}: ${issue.message}`,
    );

    logger.warn({ errors }, 'Output validation failed');
    return { valid: false, data: null, errors };
  }

  /** Validate both input and output in a single call. */
  validate(input: unknown, output: unknown): {
    input: GuardrailResult<TInput>;
    output: GuardrailResult<TOutput>;
  } {
    return {
      input: this.validateInput(input),
      output: this.validateOutput(output),
    };
  }

  /** Get the input Zod schema. */
  getInputSchema(): z.ZodType<TInput> {
    return this.inputSchema;
  }

  /** Get the output Zod schema. */
  getOutputSchema(): z.ZodType<TOutput> {
    return this.outputSchema;
  }
}

/**
 * Create a guardrail from separate input/output Zod schemas.
 *
 * Convenience factory function for quick guardrail creation.
 */
export function createGuardrails<TInput, TOutput>(
  inputSchema: z.ZodType<TInput>,
  outputSchema: z.ZodType<TOutput>,
  options?: { maxInputLength?: number; maxOutputLength?: number },
): Guardrails<TInput, TOutput> {
  return new Guardrails(inputSchema, outputSchema, options);
}
