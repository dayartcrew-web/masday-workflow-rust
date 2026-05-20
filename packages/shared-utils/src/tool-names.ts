/**
 * Bidirectional MCP tool name converter.
 *
 * Canonical format (internal):  dot notation   — workflow.create
 * Alias format   (Copilot):     underscore     — workflow_create
 *
 * The MCP SDK already transforms dots to underscores on the wire
 * (mcp__masday__workflow_create), but GitHub Copilot validates tool
 * names against `[a-z0-9_-]` BEFORE the SDK sees them.  Registering
 * explicit underscore aliases ensures tools are discoverable on
 * platforms that reject dot notation.
 */

const SEPARATOR_DOT = ".";
const SEPARATOR_UNDERSCORE = "_";

/** Convert dot-notation tool name to underscore alias: `workflow.create` -> `workflow_create` */
export function dotToUnderscore(name: string): string {
  return name.split(SEPARATOR_DOT).join(SEPARATOR_UNDERSCORE);
}

/** Convert underscore tool name back to dot notation: `workflow_create` -> `workflow.create` */
export function underscoreToDot(name: string): string {
  const idx = name.indexOf(SEPARATOR_UNDERSCORE);
  if (idx === -1) return name;
  return name.slice(0, idx) + SEPARATOR_DOT + name.slice(idx + 1);
}

/** Check if a tool name uses dot notation */
export function isDotNotation(name: string): boolean {
  return name.includes(SEPARATOR_DOT);
}

/** Check if a tool name uses underscore notation (has underscore but no dot) */
export function isUnderscoreNotation(name: string): boolean {
  return !name.includes(SEPARATOR_DOT) && name.includes(SEPARATOR_UNDERSCORE);
}

/** Bi-directional lookup map. Built once at startup. */
export class ToolNameRegistry {
  private readonly dotToAlias = new Map<string, string>();
  private readonly aliasToDot = new Map<string, string>();

  /** Register a canonical (dot-notation) tool name and auto-create its underscore alias. */
  register(dotName: string): { dot: string; alias: string } {
    const alias = dotToUnderscore(dotName);
    this.dotToAlias.set(dotName, alias);
    this.aliasToDot.set(alias, dotName);
    return { dot: dotName, alias };
  }

  /** Resolve any name (dot or underscore) to the canonical dot-notation name. */
  resolve(name: string): string {
    if (this.dotToAlias.has(name)) return name;
    return this.aliasToDot.get(name) ?? name;
  }

  /** Get the underscore alias for a canonical name. */
  getAlias(dotName: string): string | undefined {
    return this.dotToAlias.get(dotName);
  }

  /** Get the canonical name for an alias. */
  getCanonical(alias: string): string | undefined {
    return this.aliasToDot.get(alias);
  }

  /** All registered canonical names. */
  get canonicalNames(): string[] {
    return [...this.dotToAlias.keys()];
  }

  /** All registered aliases. */
  get aliases(): string[] {
    return [...this.aliasToDot.keys()];
  }

  /** Total count (canonical names only). */
  get size(): number {
    return this.dotToAlias.size;
  }

  /** Check if a name (either format) is registered. */
  has(name: string): boolean {
    return this.dotToAlias.has(name) || this.aliasToDot.has(name);
  }
}
