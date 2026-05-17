import { z } from "zod";
import { createLogger } from "@mcp-rebuild/core";
import { runCommand } from "./run-command.js";

const logger = createLogger("GitSkills");

// ---------------------------------------------------------------------------
// Schemas
// ---------------------------------------------------------------------------

export const gitStatusSchema = z.object({
  repoPath: z.string().default("."),
});

export type GitStatusInput = z.infer<typeof gitStatusSchema>;

export const gitStatusOutputSchema = z.object({
  branch: z.string(),
  staged: z.array(z.string()),
  unstaged: z.array(z.string()),
  untracked: z.array(z.string()),
  clean: z.boolean(),
});

export type GitStatusOutput = z.infer<typeof gitStatusOutputSchema>;

export const gitDiffSchema = z.object({
  repoPath: z.string().default("."),
  staged: z.boolean().default(false),
  file: z.string().optional(),
});

export type GitDiffInput = z.infer<typeof gitDiffSchema>;

export const gitDiffOutputSchema = z.object({
  diff: z.string(),
  filesChanged: z.number(),
});

export type GitDiffOutput = z.infer<typeof gitDiffOutputSchema>;

export const gitCommitSchema = z.object({
  repoPath: z.string().default("."),
  message: z.string(),
  addAll: z.boolean().default(false),
});

export type GitCommitInput = z.infer<typeof gitCommitSchema>;

export const gitCommitOutputSchema = z.object({
  success: z.boolean(),
  commit: z.string(),
  message: z.string(),
});

export type GitCommitOutput = z.infer<typeof gitCommitOutputSchema>;

// ---------------------------------------------------------------------------
// Functions
// ---------------------------------------------------------------------------

export async function runGitStatus(
  input: unknown,
): Promise<GitStatusOutput> {
  const { repoPath } = gitStatusSchema.parse(input);
  const { stdout } = await runCommand("git", ["status", "--porcelain=v2", "--branch"], { cwd: repoPath });

  let branch = "";
  const staged: string[] = [];
  const unstaged: string[] = [];
  const untracked: string[] = [];

  for (const line of stdout.split("\n")) {
    if (line.startsWith("# branch.head")) {
      branch = line.split(" ").slice(2).join(" ");
    } else if (line.startsWith("1 ") || line.startsWith("2 ")) {
      const parts = line.split(" ");
      const xy = parts[1];
      const filePath = parts.slice(-1)[0];
      if (xy[0] !== "." && xy[0] !== "?") {
        staged.push(filePath);
      }
      if (xy[1] !== "." && xy[1] !== "?") {
        unstaged.push(filePath);
      }
    } else if (line.startsWith("? ")) {
      untracked.push(line.substring(2));
    }
  }

  logger.info(`Git status: branch=${branch}, staged=${staged.length}, unstaged=${unstaged.length}`);
  return {
    branch,
    staged,
    unstaged,
    untracked,
    clean: staged.length === 0 && unstaged.length === 0 && untracked.length === 0,
  };
}

export async function runGitDiff(
  input: unknown,
): Promise<GitDiffOutput> {
  const { repoPath, staged, file } = gitDiffSchema.parse(input);
  const args = ["diff"];
  if (staged) args.push("--cached");
  if (file) args.push("--", file);

  const { stdout } = await runCommand("git", args, { cwd: repoPath });
  const filesChanged = (stdout.match(/^diff --git/gm) || []).length;

  logger.info(`Git diff: ${filesChanged} files changed`);
  return { diff: stdout, filesChanged };
}

export async function runGitCommit(
  input: unknown,
): Promise<GitCommitOutput> {
  const { repoPath, message, addAll } = gitCommitSchema.parse(input);

  if (addAll) {
    await runCommand("git", ["add", "-A"], { cwd: repoPath });
  }

  const { stdout } = await runCommand("git", ["commit", "-m", message], { cwd: repoPath });
  const match = stdout.match(/\[[\w/.-]+ (?:\([^)]+\) )?([a-f0-9]+)/);
  const commitHash = match ? match[1] : "";

  logger.info(`Git commit: ${commitHash}`);
  return { success: true, commit: commitHash, message };
}
