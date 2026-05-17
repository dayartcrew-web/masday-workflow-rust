import { z } from "zod";
import { createLogger } from "@mcp-rebuild/core";
import { runCommand } from "./run-command.js";

const logger = createLogger("NpmSkills");

// ---------------------------------------------------------------------------
// Schemas
// ---------------------------------------------------------------------------

export const npmInstallSchema = z.object({
  repoPath: z.string().default("."),
  packages: z.array(z.string()).optional(),
  dev: z.boolean().default(false),
  exact: z.boolean().default(false),
  save: z.boolean().default(false),
  silent: z.boolean().default(false),
  force: z.boolean().default(false),
  dryRun: z.boolean().default(false),
  global: z.boolean().default(false),
});

export type NpmInstallInput = z.infer<typeof npmInstallSchema>;

export const npmInstallOutputSchema = z.object({
  success: z.boolean(),
  exitCode: z.number(),
  stdout: z.string(),
  stderr: z.string(),
  packagesInstalled: z.number(),
  durationMs: z.number(),
});

export type NpmInstallOutput = z.infer<typeof npmInstallOutputSchema>;

export const npmRunSchema = z.object({
  repoPath: z.string().default("."),
  script: z.string(),
  args: z.array(z.string()).optional(),
  env: z.record(z.string()).optional(),
});

export type NpmRunInput = z.infer<typeof npmRunSchema>;

export const npmRunOutputSchema = z.object({
  success: z.boolean(),
  exitCode: z.number(),
  stdout: z.string(),
  stderr: z.string(),
  scriptName: z.string(),
  durationMs: z.number(),
});

export type NpmRunOutput = z.infer<typeof npmRunOutputSchema>;

// ---------------------------------------------------------------------------
// Functions
// ---------------------------------------------------------------------------

export async function runNpmInstall(
  input: unknown,
): Promise<NpmInstallOutput> {
  const {
    repoPath,
    packages,
    dev,
    exact,
    save,
    silent,
    force,
    dryRun,
    global: npmGlobal,
  } = npmInstallSchema.parse(input);

  const args = ["install"];
  if (packages && packages.length > 0) {
    args.push(...packages);
  }
  if (dev) args.push("--dev");
  if (exact) args.push("--exact");
  if (save) args.push("--save");
  if (silent) args.push("--silent");
  if (force) args.push("--force");
  if (dryRun) args.push("--dry-run");
  if (npmGlobal) args.push("--global");

  const startTime = Date.now();

  try {
    const { stdout, stderr } = await runCommand("npm", args, { cwd: repoPath });
    const installedMatch = stdout.match(/added (\d+) package/);
    const packagesInstalled = installedMatch ? parseInt(installedMatch[1], 10) : 0;

    logger.info(`npm install: ${packagesInstalled} packages, exitCode: 0`);
    return {
      success: true,
      exitCode: 0,
      stdout,
      stderr,
      packagesInstalled,
      durationMs: Date.now() - startTime,
    };
  } catch (error: unknown) {
    const execError = error as { stdout?: string; stderr?: string; code?: number };
    const stdout = execError.stdout || "";
    const stderr = execError.stderr || "";

    logger.error(`npm install failed: ${execError.code || 1}`);
    return {
      success: false,
      exitCode: execError.code || 1,
      stdout,
      stderr,
      packagesInstalled: 0,
      durationMs: Date.now() - startTime,
    };
  }
}

export async function runNpmRun(
  input: unknown,
): Promise<NpmRunOutput> {
  const { repoPath, script, args: runArgs, env: npmEnv } = npmRunSchema.parse(input);

  const cmdArgs = ["run", script];
  if (runArgs && runArgs.length > 0) {
    cmdArgs.push("--", ...runArgs);
  }

  const startTime = Date.now();

  try {
    const envVars = npmEnv
      ? { ...(process.env as Record<string, string>), ...npmEnv }
      : undefined;
    const { stdout, stderr } = await runCommand("npm", cmdArgs, {
      cwd: repoPath,
      ...(envVars && { env: envVars }),
    });

    logger.info(`npm run ${script}: exitCode: 0`);
    return {
      success: true,
      exitCode: 0,
      stdout,
      stderr,
      scriptName: script,
      durationMs: Date.now() - startTime,
    };
  } catch (error: unknown) {
    const execError = error as { stdout?: string; stderr?: string; code?: number };
    const stdout = execError.stdout || "";
    const stderr = execError.stderr || "";

    logger.error(`npm run ${script} failed: ${execError.code || 1}`);
    return {
      success: false,
      exitCode: execError.code || 1,
      stdout,
      stderr,
      scriptName: script,
      durationMs: Date.now() - startTime,
    };
  }
}
