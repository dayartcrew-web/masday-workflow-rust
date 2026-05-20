import { mkdirSync, writeFileSync, existsSync } from "fs";
import { join } from "path";

export interface CreateAgentInput {
  projectRoot: string;
  name: string;
  role: string;
  description: string;
  model?: string;
  tools?: string[];
  instructions: string;
}

export interface CreateAgentResult {
  ok: true;
  name: string;
  filePath: string;
  alreadyExists: boolean;
}

export function validateAgentName(name: string): string | null {
  if (!/^[a-z][a-z0-9-]{1,63}$/.test(name)) {
    return "Agent name must be kebab-case, start with a letter, 2-64 chars (e.g. security-reviewer)";
  }
  return null;
}

export function buildAgentMarkdown(input: CreateAgentInput): string {
  const toolsSection = input.tools && input.tools.length > 0
    ? `\ntools:\n${input.tools.map(t => `  - ${t}`).join("\n")}`
    : "";
  const modelSection = input.model ? `\nmodel: ${input.model}` : "";
  return [
    "---",
    `name: ${input.name}`,
    `role: ${input.role}`,
    `description: ${input.description}` + modelSection + toolsSection,
    "---", "",
    input.instructions, "",
  ].join("\n");
}

export function createAgent(input: CreateAgentInput): CreateAgentResult {
  const validationError = validateAgentName(input.name);
  if (validationError) throw new Error(validationError);

  const agentDir = join(input.projectRoot, ".claude", "agents");
  if (!existsSync(agentDir)) mkdirSync(agentDir, { recursive: true });

  const fileName = `${input.name}.md`;
  const filePath = join(agentDir, fileName);
  const alreadyExists = existsSync(filePath);
  const content = buildAgentMarkdown(input);

  writeFileSync(filePath, content, "utf-8");
  return { ok: true, name: input.name, filePath, alreadyExists };
}
