import { execFile as execFileCb } from "node:child_process";

export interface RunCommandResult {
  stdout: string;
  stderr: string;
}

export interface RunCommandOptions {
  cwd: string;
  env?: Record<string, string>;
  maxBuffer?: number;
}

/**
 * Shared helper to execute a command and capture stdout/stderr.
 * Rejects with an enriched Error on non-zero exit codes.
 */
export function runCommand(
  cmd: string,
  args: string[],
  options: RunCommandOptions,
): Promise<RunCommandResult> {
  return new Promise((resolve, reject) => {
    const execOptions: {
      cwd: string;
      env?: Record<string, string>;
      maxBuffer?: number;
    } = { cwd: options.cwd };

    if (options.env) {
      execOptions.env = {
        ...process.env as Record<string, string>,
        ...options.env,
      };
    }

    if (options.maxBuffer) {
      execOptions.maxBuffer = options.maxBuffer;
    }

    execFileCb(cmd, args, execOptions, (error, stdout, stderr) => {
      if (error) {
        const err = error as Error & {
          stdout?: string;
          stderr?: string;
          code?: number;
        };
        err.stdout = err.stdout || stdout || "";
        err.stderr = err.stderr || stderr || "";
        err.code = err.code ?? 1;
        reject(err);
      } else {
        resolve({ stdout: stdout || "", stderr: stderr || "" });
      }
    });
  });
}
