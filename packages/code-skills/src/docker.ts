import { z } from "zod";
import { createLogger } from "@mcp-rebuild/core";
import { runCommand } from "./run-command.js";

const logger = createLogger("DockerSkills");

// ---------------------------------------------------------------------------
// Schemas
// ---------------------------------------------------------------------------

export const dockerBuildSchema = z.object({
  repoPath: z.string().default("."),
  tag: z.string(),
  dockerfile: z.string().default("Dockerfile"),
  context: z.string().default("."),
});

export type DockerBuildInput = z.infer<typeof dockerBuildSchema>;

export const dockerBuildOutputSchema = z.object({
  success: z.boolean(),
  imageId: z.string(),
  tag: z.string(),
});

export type DockerBuildOutput = z.infer<typeof dockerBuildOutputSchema>;

export const dockerRunSchema = z.object({
  repoPath: z.string().default("."),
  image: z.string(),
  command: z.array(z.string()).optional(),
  env: z.array(z.object({ key: z.string(), value: z.string() })).optional(),
  ports: z.array(z.object({ host: z.number(), container: z.number() })).optional(),
  detach: z.boolean().default(true),
  name: z.string().optional(),
});

export type DockerRunInput = z.infer<typeof dockerRunSchema>;

export const dockerRunOutputSchema = z.object({
  success: z.boolean(),
  containerId: z.string(),
});

export type DockerRunOutput = z.infer<typeof dockerRunOutputSchema>;

export const dockerPsSchema = z.object({
  repoPath: z.string().default("."),
  all: z.boolean().default(false),
});

export type DockerPsInput = z.infer<typeof dockerPsSchema>;

export const dockerPsOutputSchema = z.object({
  containers: z.array(
    z.object({
      id: z.string(),
      image: z.string(),
      status: z.string(),
      names: z.string(),
    }),
  ),
});

export type DockerPsOutput = z.infer<typeof dockerPsOutputSchema>;

// ---------------------------------------------------------------------------
// Functions
// ---------------------------------------------------------------------------

export async function runDockerBuild(
  input: unknown,
): Promise<DockerBuildOutput> {
  const { repoPath, tag, dockerfile, context } = dockerBuildSchema.parse(input);
  const args = ["build", "-t", tag, "-f", dockerfile, context];

  const { stdout } = await runCommand("docker", args, { cwd: repoPath });
  const idMatch = stdout.match(/Successfully built ([a-f0-9]+)/);
  const imageId = idMatch ? idMatch[1] : "";

  logger.info(`Docker build: ${tag} (${imageId})`);
  return { success: true, imageId, tag };
}

export async function runDockerRun(
  input: unknown,
): Promise<DockerRunOutput> {
  const { repoPath, image, command, env, ports, detach, name } = dockerRunSchema.parse(input);
  const args = ["run"];
  if (detach) args.push("-d");
  if (name) args.push("--name", name);
  if (env) {
    for (const e of env) {
      args.push("-e", `${e.key}=${e.value}`);
    }
  }
  if (ports) {
    for (const p of ports) {
      args.push("-p", `${p.host}:${p.container}`);
    }
  }
  args.push(image);
  if (command) args.push(...command);

  const { stdout } = await runCommand("docker", args, { cwd: repoPath });
  const containerId = stdout.trim().substring(0, 12);

  logger.info(`Docker run: ${image} -> ${containerId}`);
  return { success: true, containerId };
}

export async function runDockerPs(
  input: unknown,
): Promise<DockerPsOutput> {
  const { repoPath, all } = dockerPsSchema.parse(input);
  const args = ["ps", "--format", "{{.ID}}\t{{.Image}}\t{{.Status}}\t{{.Names}}"];
  if (all) args.push("-a");

  const { stdout } = await runCommand("docker", args, { cwd: repoPath });
  const containers = stdout
    .trim()
    .split("\n")
    .filter(Boolean)
    .map((line) => {
      const [id, image, status, names] = line.split("\t");
      return {
        id: id || "",
        image: image || "",
        status: status || "",
        names: names || "",
      };
    });

  logger.info(`Docker ps: ${containers.length} containers`);
  return { containers };
}
