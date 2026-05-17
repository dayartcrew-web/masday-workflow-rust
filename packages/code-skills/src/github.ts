import { z } from "zod";
import { createLogger } from "@mcp-rebuild/core";
import { runCommand } from "./run-command.js";

const logger = createLogger("GitHubSkills");

// ---------------------------------------------------------------------------
// Schemas
// ---------------------------------------------------------------------------

export const githubPrCreateSchema = z.object({
  repoPath: z.string().default("."),
  title: z.string(),
  body: z.string().default(""),
  base: z.string().default("main"),
  head: z.string().optional(),
  draft: z.boolean().default(false),
});

export type GithubPrCreateInput = z.infer<typeof githubPrCreateSchema>;

export const githubPrCreateOutputSchema = z.object({
  success: z.boolean(),
  url: z.string(),
  number: z.number(),
});

export type GithubPrCreateOutput = z.infer<typeof githubPrCreateOutputSchema>;

export const githubPrListSchema = z.object({
  repoPath: z.string().default("."),
  state: z.enum(["open", "closed", "all"]).default("open"),
  limit: z.number().default(10),
});

export type GithubPrListInput = z.infer<typeof githubPrListSchema>;

export const githubPrListOutputSchema = z.object({
  pulls: z.array(
    z.object({
      number: z.number(),
      title: z.string(),
      state: z.string(),
      url: z.string(),
    }),
  ),
});

export type GithubPrListOutput = z.infer<typeof githubPrListOutputSchema>;

export const githubIssueListSchema = z.object({
  repoPath: z.string().default("."),
  state: z.enum(["open", "closed", "all"]).default("open"),
  limit: z.number().default(10),
  labels: z.array(z.string()).optional(),
});

export type GithubIssueListInput = z.infer<typeof githubIssueListSchema>;

export const githubIssueListOutputSchema = z.object({
  issues: z.array(
    z.object({
      number: z.number(),
      title: z.string(),
      state: z.string(),
      url: z.string(),
    }),
  ),
});

export type GithubIssueListOutput = z.infer<typeof githubIssueListOutputSchema>;

// ---------------------------------------------------------------------------
// Functions
// ---------------------------------------------------------------------------

export async function runGithubPrCreate(
  input: unknown,
): Promise<GithubPrCreateOutput> {
  const { repoPath, title, body, base, head, draft } = githubPrCreateSchema.parse(input);
  const args = ["pr", "create", "--title", title, "--body", body, "--base", base];
  if (head) args.push("--head", head);
  if (draft) args.push("--draft");

  const { stdout } = await runCommand("gh", args, { cwd: repoPath });
  const urlMatch = stdout.match(/https:\/\/github\.com\/[^\s]+\/pull\/(\d+)/);
  const url = urlMatch ? urlMatch[0] : stdout.trim();
  const number = urlMatch ? parseInt(urlMatch[1]) : 0;

  logger.info(`Created PR #${number}: ${url}`);
  return { success: true, url, number };
}

export async function runGithubPrList(
  input: unknown,
): Promise<GithubPrListOutput> {
  const { repoPath, state, limit } = githubPrListSchema.parse(input);
  const args = [
    "pr",
    "list",
    "--state",
    state,
    "--limit",
    String(limit),
    "--json",
    "number,title,state,url",
  ];

  const { stdout } = await runCommand("gh", args, { cwd: repoPath });
  const pulls = JSON.parse(stdout || "[]");

  logger.info(`Listed ${pulls.length} PRs`);
  return { pulls };
}

export async function runGithubIssueList(
  input: unknown,
): Promise<GithubIssueListOutput> {
  const { repoPath, state, limit, labels } = githubIssueListSchema.parse(input);
  const args = [
    "issue",
    "list",
    "--state",
    state,
    "--limit",
    String(limit),
    "--json",
    "number,title,state,url",
  ];
  if (labels && labels.length > 0) {
    args.push("--label", labels.join(","));
  }

  const { stdout } = await runCommand("gh", args, { cwd: repoPath });
  const issues = JSON.parse(stdout || "[]");

  logger.info(`Listed ${issues.length} issues`);
  return { issues };
}
