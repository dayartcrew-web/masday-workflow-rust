import { mkdirSync, writeFileSync, existsSync } from "fs";
import { join } from "path";

export interface CreateSkillInput {
  projectRoot: string;
  name: string;
  description: string;
  trigger: string;
  steps: string[];
  allowedTools?: string[];
}

export interface CreateSkillResult {
  ok: true;
  name: string;
  dirPath: string;
  filePath: string;
  alreadyExists: boolean;
}

export function validateSkillName(name: string): string | null {
  if (!/^[a-z][a-z0-9-]{1,63}$/.test(name)) {
    return "Skill name must be kebab-case, start with a letter, 2-64 chars (e.g. masday-deploy-check)";
  }
  return null;
}

export function buildSkillMarkdown(input: CreateSkillInput): string {
  const allowedToolsSection = input.allowedTools && input.allowedTools.length > 0
    ? `\nallowed-tools:\n${input.allowedTools.map(t => `  - ${t}`).join("\n")}`
    : "";
  const stepsSection = input.steps.map((s, i) => `${i + 1}. ${s}`).join("\n");

  return [
    "---",
    `name: ${input.name}`,
    `description: >`,
    `  ${input.description}`,
    `  Use when the user says "${input.trigger}".` + allowedToolsSection,
    "---", "",
    `# ${input.name}`, "",
    stepsSection, "",
  ].join("\n");
}

export function createSkill(input: CreateSkillInput): CreateSkillResult {
  const validationError = validateSkillName(input.name);
  if (validationError) throw new Error(validationError);

  const skillsDir = join(input.projectRoot, ".claude", "skills", input.name);
  if (!existsSync(skillsDir)) mkdirSync(skillsDir, { recursive: true });

  const filePath = join(skillsDir, "SKILL.md");
  const alreadyExists = existsSync(filePath);
  const content = buildSkillMarkdown(input);

  writeFileSync(filePath, content, "utf-8");
  return { ok: true, name: input.name, dirPath: skillsDir, filePath, alreadyExists };
}
