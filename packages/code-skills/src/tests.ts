import { z } from "zod";
import { createLogger } from "@mcp-rebuild/core";
import { runCommand } from "./run-command.js";

const logger = createLogger("TestSkills");

// ---------------------------------------------------------------------------
// Schemas
// ---------------------------------------------------------------------------

export const testsRunSchema = z.object({
  repoPath: z.string().default("."),
  testPattern: z.string().optional(),
  coverage: z.boolean().default(false),
  config: z.string().optional(),
});

export type TestsRunInput = z.infer<typeof testsRunSchema>;

export const testsRunOutputSchema = z.object({
  success: z.boolean(),
  exitCode: z.number(),
  stdout: z.string(),
  stderr: z.string(),
  testsRun: z.number(),
  testsPassed: z.number(),
  testsFailed: z.number(),
});

export type TestsRunOutput = z.infer<typeof testsRunOutputSchema>;

// ---------------------------------------------------------------------------
// Functions
// ---------------------------------------------------------------------------

export async function runTests(
  input: unknown,
): Promise<TestsRunOutput> {
  const { repoPath, testPattern, coverage, config } = testsRunSchema.parse(input);
  const args = ["vitest", "run"];

  if (coverage) args.push("--coverage");
  if (config) args.push("--config", config);
  if (testPattern) args.push(testPattern);

  try {
    const { stdout, stderr } = await runCommand("npx", args, {
      cwd: repoPath,
      maxBuffer: 10 * 1024 * 1024,
    });

    const parsed = parseVitestOutput(stdout);

    logger.info(`Tests run: ${parsed.testsRun}, passed: ${parsed.testsPassed}, failed: ${parsed.testsFailed}`);
    return {
      success: true,
      exitCode: 0,
      stdout,
      stderr,
      ...parsed,
    };
  } catch (error: unknown) {
    const execError = error as { stdout?: string; stderr?: string; code?: number };
    const stdout = execError.stdout || "";
    const stderr = execError.stderr || "";
    const parsed = parseVitestOutput(stdout + "\n" + stderr);

    logger.error(`Tests failed: ${parsed.testsFailed} failures`);
    return {
      success: false,
      exitCode: execError.code ?? 1,
      stdout,
      stderr,
      ...parsed,
    };
  }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function parseVitestOutput(output: string): {
  testsRun: number;
  testsPassed: number;
  testsFailed: number;
} {
  const passedMatch = output.match(/(\d+)\s+passed/);
  const failedMatch = output.match(/(\d+)\s+failed/);

  const testsPassed = passedMatch ? parseInt(passedMatch[1], 10) : 0;
  const testsFailed = failedMatch ? parseInt(failedMatch[1], 10) : 0;

  return {
    testsRun: testsPassed + testsFailed,
    testsPassed,
    testsFailed,
  };
}
