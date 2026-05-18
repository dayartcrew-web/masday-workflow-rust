/**
 * Skill Loader
 *
 * Parses SKILL.md files from .claude/skills/ directories.
 * Extracts frontmatter (name, description, allowed-tools) and
 * the markdown body (steps).
 */

import * as fs from "fs";
import * as path from "path";
import { createLogger } from "@mcp-rebuild/core";

const logger = createLogger("SkillLoader");

export interface SkillDefinition {
  name: string;
  description: string;
  allowedTools: string[];
  steps: string;
  filePath: string;
}

/**
 * Parse YAML-like frontmatter from a markdown file.
 * Handles both scalar values and array values (prefixed with `  - `).
 */
export function parseSkillFrontmatter(content: string): {
  meta: Record<string, string | string[]>;
  body: string;
} {
  const trimmed = content.trimStart();
  const result: Record<string, string | string[]> = {};

  if (!trimmed.startsWith("---")) {
    return { meta: result, body: content };
  }

  const endIndex = trimmed.indexOf("---", 3);
  if (endIndex === -1) {
    return { meta: result, body: content };
  }

  const frontmatter = trimmed.slice(3, endIndex).trim();
  const body = trimmed.slice(endIndex + 3).trim();

  const lines = frontmatter.split("\n");
  let currentKey: string | null = null;
  const currentArray: string[] = [];

  for (const line of lines) {
    const arrayMatch = line.match(/^\s+-\s+(.+)$/);
    if (arrayMatch && currentKey) {
      currentArray.push(arrayMatch[1].trim());
      continue;
    }

    if (currentKey && currentArray.length > 0) {
      result[currentKey] = [...currentArray];
      currentArray.length = 0;
    }

    const colonIndex = line.indexOf(":");
    if (colonIndex === -1) {
      currentKey = null;
      continue;
    }

    const key = line.slice(0, colonIndex).trim();
    const value = line.slice(colonIndex + 1).trim();
    currentKey = key;

    if (value === "" || value === ">" || value === "|") {
      continue;
    }

    result[key] = value;
  }

  if (currentKey && currentArray.length > 0) {
    result[currentKey] = [...currentArray];
  }

  return { meta: result, body };
}

export class SkillLoader {
  private skills = new Map<string, SkillDefinition>();
  private loaded = false;

  constructor(private skillsDir: string) {}

  loadAll(): SkillDefinition[] {
    this.skills.clear();

    if (!fs.existsSync(this.skillsDir)) {
      logger.warn({ dir: this.skillsDir }, "Skills directory not found");
      return [];
    }

    const entries = fs.readdirSync(this.skillsDir, { withFileTypes: true });

    for (const entry of entries) {
      if (!entry.isDirectory()) continue;

      const skillFile = path.join(this.skillsDir, entry.name, "SKILL.md");
      if (!fs.existsSync(skillFile)) continue;

      try {
        const content = fs.readFileSync(skillFile, "utf-8");
        const { meta, body } = parseSkillFrontmatter(content);

        const name =
          typeof meta.name === "string"
            ? meta.name
            : entry.name;

        const description =
          typeof meta.description === "string"
            ? meta.description
            : "";

        const allowedTools =
          Array.isArray(meta["allowed-tools"])
            ? (meta["allowed-tools"] as string[])
            : [];

        const def: SkillDefinition = {
          name,
          description,
          allowedTools,
          steps: body,
          filePath: skillFile,
        };

        this.skills.set(name, def);
      } catch (err) {
        logger.warn({ file: skillFile, error: err }, "Failed to parse skill");
      }
    }

    this.loaded = true;
    logger.info({ count: this.skills.size }, "Loaded skills");
    return Array.from(this.skills.values());
  }

  load(name: string): SkillDefinition | undefined {
    if (!this.loaded) this.loadAll();
    return this.skills.get(name);
  }

  has(name: string): boolean {
    if (!this.loaded) this.loadAll();
    return this.skills.has(name);
  }

  getAll(): Array<{ name: string; description: string }> {
    if (!this.loaded) this.loadAll();
    return Array.from(this.skills.values()).map((s) => ({
      name: s.name,
      description: s.description,
    }));
  }

  get size(): number {
    return this.skills.size;
  }
}
