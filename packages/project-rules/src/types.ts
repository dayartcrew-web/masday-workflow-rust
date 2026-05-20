import { z } from "zod";

// --- Rule definitions ---

export const RuleSeverity = z.enum(["CRITICAL", "HIGH", "MEDIUM", "LOW"]);
export type RuleSeverity = z.infer<typeof RuleSeverity>;

export const RuleCategory = z.enum([
  "NAMING",
  "PATTERN",
  "TOOLS",
  "DOCS",
  "TYPESCRIPT",
  "TESTING",
  "SECURITY",
  "ARCHITECTURE",
  "MCP",
  "DATABASE",
  "IMPORTS",
  "EXPORTS",
  "PACKAGE_JSON",
  "GIT",
  "MULTI_LLM",
]);
export type RuleCategory = z.infer<typeof RuleCategory>;

export const RefactorRule = z.object({
  id: z.string().min(1),
  title: z.string().min(1),
  description: z.string().min(1),
  category: RuleCategory,
  severity: RuleSeverity,
  pattern: z.string().optional(),
  negPattern: z.string().optional(),
  check: z
    .enum([
      "file-exists",
      "no-match",
      "has-match",
      "json-key",
      "package-field",
      "custom",
    ])
    .optional(),
  targets: z.array(z.string()).optional(),
  fixHint: z.string().optional(),
});
export type RefactorRule = z.infer<typeof RefactorRule>;

export const RuleSet = z.object({
  version: z.string(),
  updated: z.string(),
  rules: z.array(RefactorRule),
});
export type RuleSet = z.infer<typeof RuleSet>;

// --- Check results ---

export const CheckResult = z.object({
  ruleId: z.string(),
  passed: z.boolean(),
  message: z.string(),
  severity: RuleSeverity,
  category: RuleCategory,
});
export type CheckResult = z.infer<typeof CheckResult>;

export const RefactorReport = z.object({
  timestamp: z.string(),
  totalRules: z.number(),
  passed: z.number(),
  failed: z.number(),
  skipped: z.number(),
  results: z.array(CheckResult),
  summary: z.record(RuleCategory, z.object({ passed: z.number(), failed: z.number() })),
});
export type RefactorReport = z.infer<typeof RefactorReport>;

// --- Checklist for refactoring ---

export const RefactorChecklistItem = z.object({
  id: z.string(),
  label: z.string(),
  category: RuleCategory,
  required: z.boolean(),
  description: z.string(),
});
export type RefactorChecklistItem = z.infer<typeof RefactorChecklistItem>;

export const RefactorChecklist = z.object({
  version: z.string(),
  items: z.array(RefactorChecklistItem),
});
export type RefactorChecklist = z.infer<typeof RefactorChecklist>;

// --- Multi-LLM rule scanning ---

export const LlmRuleSource = z.object({
  platform: z.string(),
  rulesDir: z.string(),
  exists: z.boolean(),
  ruleFiles: z.array(z.string()),
});
export type LlmRuleSource = z.infer<typeof LlmRuleSource>;

export const LlmRulesScanResult = z.object({
  projectRoot: z.string(),
  sources: z.array(LlmRuleSource),
  totalRules: z.number(),
  platforms: z.array(z.string()),
});
export type LlmRulesScanResult = z.infer<typeof LlmRulesScanResult>;
