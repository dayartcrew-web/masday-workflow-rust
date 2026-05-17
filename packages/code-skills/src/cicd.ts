import { z } from "zod";
import { createLogger } from "@mcp-rebuild/core";
import { runCommand } from "./run-command.js";

const logger = createLogger("CICDSkills");

// ---------------------------------------------------------------------------
// Schemas
// ---------------------------------------------------------------------------

export const cicdStatusSchema = z.object({
  repoPath: z.string().default("."),
  branch: z.string().optional(),
  limit: z.number().default(5),
});

export type CicdStatusInput = z.infer<typeof cicdStatusSchema>;

export const cicdStatusOutputSchema = z.object({
  runs: z.array(
    z.object({
      name: z.string(),
      status: z.string(),
      conclusion: z.string(),
      url: z.string(),
      createdAt: z.string(),
    }),
  ),
});

export type CicdStatusOutput = z.infer<typeof cicdStatusOutputSchema>;

export const cicdTriggerSchema = z.object({
  repoPath: z.string().default("."),
  workflow: z.string(),
  ref: z.string().optional(),
  inputs: z.record(z.string()).optional(),
});

export type CicdTriggerInput = z.infer<typeof cicdTriggerSchema>;

export const cicdTriggerOutputSchema = z.object({
  success: z.boolean(),
  message: z.string(),
});

export type CicdTriggerOutput = z.infer<typeof cicdTriggerOutputSchema>;

export const cicdViewSchema = z.object({
  repoPath: z.string().default("."),
  runId: z.number(),
});

export type CicdViewInput = z.infer<typeof cicdViewSchema>;

export const cicdViewOutputSchema = z.object({
  name: z.string(),
  status: z.string(),
  conclusion: z.string(),
  url: z.string(),
  jobs: z.array(
    z.object({
      name: z.string(),
      status: z.string(),
      conclusion: z.string(),
    }),
  ),
});

export type CicdViewOutput = z.infer<typeof cicdViewOutputSchema>;

// ---------------------------------------------------------------------------
// Functions
// ---------------------------------------------------------------------------

export async function runCicdStatus(
  input: unknown,
): Promise<CicdStatusOutput> {
  const { repoPath, branch, limit } = cicdStatusSchema.parse(input);
  const args = [
    "run",
    "list",
    "--limit",
    String(limit),
    "--json",
    "name,status,conclusion,url,createdAt",
  ];
  if (branch) args.push("--branch", branch);

  const { stdout } = await runCommand("gh", args, { cwd: repoPath });
  const runs = JSON.parse(stdout || "[]").map((r: Record<string, string>) => ({
    name: r.name || "",
    status: r.status || "",
    conclusion: r.conclusion || "",
    url: r.url || "",
    createdAt: r.createdAt || "",
  }));

  logger.info(`Pipeline status: ${runs.length} runs`);
  return { runs };
}

export async function runCicdTrigger(
  input: unknown,
): Promise<CicdTriggerOutput> {
  const { repoPath, workflow, ref, inputs } = cicdTriggerSchema.parse(input);
  const args = ["workflow", "run", workflow];
  if (ref) args.push("--ref", ref);
  if (inputs) {
    for (const [key, value] of Object.entries(inputs)) {
      args.push("-f", `${key}=${value}`);
    }
  }

  await runCommand("gh", args, { cwd: repoPath });

  logger.info(`Triggered workflow: ${workflow}`);
  return { success: true, message: `Workflow ${workflow} triggered` };
}

export async function runCicdView(
  input: unknown,
): Promise<CicdViewOutput> {
  const { repoPath, runId } = cicdViewSchema.parse(input);
  const { stdout } = await runCommand(
    "gh",
    ["run", "view", String(runId), "--json", "name,status,conclusion,url,jobs"],
    { cwd: repoPath },
  );
  const data = JSON.parse(stdout || "{}") as Record<string, unknown>;
  const jobs = ((data.jobs || []) as Record<string, string>[]).map((j) => ({
    name: j.name || "",
    status: j.status || "",
    conclusion: j.conclusion || "",
  }));

  logger.info(`Viewed run ${runId}: ${data.status}`);
  return {
    name: (data.name as string) || "",
    status: (data.status as string) || "",
    conclusion: (data.conclusion as string) || "",
    url: (data.url as string) || "",
    jobs,
  };
}
