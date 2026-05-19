import { readdirSync, readFileSync } from "node:fs";
import { join, relative } from "node:path";
import type {
  CheckResult,
  RefactorReport,
  RefactorRule,
  RuleCategory,
} from "./types.js";
import { PROJECT_RULES } from "./rules.js";

function readText(p: string): string {
  try {
    return readFileSync(p, "utf-8");
  } catch {
    return "";
  }
}

function collectFiles(root: string, glob: string): string[] {
  const results: string[] = [];
  const parts = glob.split("/");
  const fileName = parts[parts.length - 1];
  const regex = new RegExp("^" + fileName.replace(/\./g, "\\.").replace(/\*/g, ".*") + "$");

  function walk(dir: string, depth: number): void {
    try {
      for (const entry of readdirSync(dir, { withFileTypes: true })) {
        if (entry.name === "node_modules" || entry.name === "dist" || entry.name === ".git") continue;
        const full = join(dir, entry.name);
        if (entry.isDirectory()) {
          if (depth < parts.length - 1) {
            const globPart = parts[depth];
            if (globPart === "*" || globPart === entry.name || globPart === "**") {
              walk(full, depth + 1);
            }
          } else {
            walk(full, depth);
          }
        } else if (regex.test(entry.name) && depth >= parts.length - 1) {
          results.push(full);
        }
      }
    } catch {
      // permission denied etc
    }
  }

  walk(root, 0);
  return results;
}

function checkPattern(rule: RefactorRule, projectRoot: string): CheckResult {
  const { passed, message } = runCheck(rule, projectRoot);
  return {
    ruleId: rule.id,
    passed,
    message,
    severity: rule.severity,
    category: rule.category,
  };
}

function runCheck(
  rule: RefactorRule,
  projectRoot: string
): { passed: boolean; message: string } {
  if (rule.negPattern) {
    const hits = grepProject(projectRoot, rule.negPattern);
    if (hits.length > 0) {
      return {
        passed: false,
        message: `Found ${hits.length} violation(s). ${rule.fixHint ?? ""}`,
      };
    }
    return { passed: true, message: "No violations found." };
  }

  if (rule.pattern && rule.check && rule.targets) {
    switch (rule.check) {
      case "has-match": {
        let found = false;
        for (const t of rule.targets) {
          const files = collectFiles(projectRoot, t);
          for (const f of files) {
            if (readText(f).includes(rule.pattern)) {
              found = true;
              break;
            }
          }
          if (found) break;
        }
        return {
          passed: found,
          message: found
            ? `Pattern "${rule.pattern}" found in targets.`
            : `Pattern "${rule.pattern}" NOT found in ${rule.targets.join(", ")}. ${rule.fixHint ?? ""}`,
        };
      }
      default:
        break;
    }
  }

  return { passed: true, message: "Manual check required." };
}

function grepProject(root: string, pattern: string): string[] {
  const regex = new RegExp(pattern);
  const hits: string[] = [];

  function walk(dir: string): void {
    try {
      for (const entry of readdirSync(dir, { withFileTypes: true })) {
        if (
          entry.name === "node_modules" ||
          entry.name === "dist" ||
          entry.name === ".git" ||
          entry.name === ".turbo"
        )
          continue;
        const full = join(dir, entry.name);
        if (entry.isDirectory()) {
          walk(full);
        } else if (
          entry.name.endsWith(".ts") ||
          entry.name.endsWith(".tsx") ||
          entry.name.endsWith(".js") ||
          entry.name.endsWith(".jsx") ||
          entry.name.endsWith(".json") ||
          entry.name.endsWith(".md")
        ) {
          const content = readText(full);
          if (regex.test(content)) {
            hits.push(relative(root, full));
          }
        }
      }
    } catch {
      // permission denied
    }
  }

  walk(root);
  return hits;
}

export function validateProject(projectRoot: string): RefactorReport {
  const results: CheckResult[] = [];
  const summary: Record<string, { passed: number; failed: number }> = {};

  for (const rule of PROJECT_RULES.rules) {
    const result = checkPattern(rule, projectRoot);
    results.push(result);

    if (!summary[rule.category]) {
      summary[rule.category] = { passed: 0, failed: 0 };
    }
    if (result.passed) {
      summary[rule.category].passed++;
    } else {
      summary[rule.category].failed++;
    }
  }

  const passed = results.filter((r) => r.passed).length;
  const failed = results.filter((r) => !r.passed).length;

  return {
    timestamp: new Date().toISOString(),
    totalRules: results.length,
    passed,
    failed,
    skipped: 0,
    results,
    summary: summary as Record<RuleCategory, { passed: number; failed: number }>,
  };
}

export function getFailedCritical(report: RefactorReport): CheckResult[] {
  return report.results.filter(
    (r) => !r.passed && r.severity === "CRITICAL"
  );
}

export function formatReport(report: RefactorReport): string {
  const lines: string[] = [
    `Refactor Check Report — ${report.timestamp}`,
    `Total: ${report.totalRules} | Passed: ${report.passed} | Failed: ${report.failed}`,
    "",
  ];

  const failed = report.results.filter((r) => !r.passed);
  if (failed.length === 0) {
    lines.push("All checks passed!");
    return lines.join("\n");
  }

  lines.push("FAILED CHECKS:");
  for (const f of failed) {
    lines.push(
      `  [${f.severity}] ${f.ruleId}: ${f.message}`
    );
  }

  lines.push("");
  lines.push("SUMMARY BY CATEGORY:");
  for (const [cat, counts] of Object.entries(report.summary)) {
    if (counts.failed > 0) {
      lines.push(`  ${cat}: ${counts.passed} passed, ${counts.failed} failed`);
    }
  }

  return lines.join("\n");
}
