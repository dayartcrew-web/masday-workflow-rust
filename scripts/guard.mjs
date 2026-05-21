#!/usr/bin/env node

/**
 * Universal Guard Script — Replicates Claude Code hook behaviors for non-Claude AI tools.
 *
 * Commands:
 *   node scripts/guard.mjs pre-edit <file>     — TDD check, secret detection, console.log, masday-first reminder
 *   node scripts/guard.mjs post-edit <file>    — Wrap-back reminder (saveProgress, memory_store, review_submit)
 *   node scripts/guard.mjs pre-commit          — Check all staged files for secrets, console.log, TDD, tool names
 *   node scripts/guard.mjs pre-complete        — Review gate verification (requires review_submit APPROVED)
 *   node scripts/guard.mjs check-staged        — Alias for pre-commit
 */

import { readFileSync, existsSync } from "node:fs";
import { join, basename } from "node:path";
import { execSync } from "node:child_process";

// --- Helpers ---

function fileExists(p) {
  return existsSync(p);
}

function isSourceFile(f) {
  return /\.(ts|tsx|js|jsx)$/.test(f) && !f.includes("node_modules") && !f.includes(".d.ts");
}

function isTestFile(f) {
  return /\.(test|spec)\.(ts|tsx|js|jsx)$/.test(f);
}

function guessTestPaths(srcFile) {
  const dir = srcFile.replace(/\/src\//, "/tests/");
  const base = basename(srcFile).replace(/\.(ts|tsx|js|jsx)$/, "");
  return [
    srcFile.replace(/(\.\w+)$/, ".test$1"),
    srcFile.replace(/(\.\w+)$/, ".spec$1"),
    join(dir, `../${base}.test.ts`),
    join(dir, `../${base}.spec.ts`),
    srcFile.replace(/\/src\//, "/tests/").replace(/(\.\w+)$/, ".test$1"),
    srcFile.replace(/\/src\//, "/tests/").replace(/(\.\w+)$/, ".spec$1"),
  ];
}

function hasTestFile(srcFile) {
  return guessTestPaths(srcFile).some((p) => fileExists(p));
}

function loadActiveTask() {
  try {
    const masdayDir = join(process.cwd(), ".masday");
    if (!existsSync(masdayDir)) return null;
    const stateFile = join(masdayDir, "active-task.json");
    if (!existsSync(stateFile)) return null;
    return JSON.parse(readFileSync(stateFile, "utf-8"));
  } catch {
    return null;
  }
}

function log(level, msg) {
  const prefix = { PASS: "  ✓", FAIL: "  ✗", WARN: "  ⚠", INFO: "  →" }[level] || "  ";
  const color = { PASS: "\x1b[32m", FAIL: "\x1b[31m", WARN: "\x1b[33m", INFO: "\x1b[36m" }[level] || "";
  const reset = "\x1b[0m";
  console.log(`${color}${prefix}${reset} ${msg}`);
}

function die(msg) {
  console.error(`\x1b[31m[BLOCKED]\x1b[0m ${msg}`);
  process.exit(1);
}

function warn(msg) {
  console.error(`\x1b[33m[WARNING]\x1b[0m ${msg}`);
}

// --- Checks ---

function checkTdd(srcFile) {
  if (!isSourceFile(srcFile) || isTestFile(srcFile)) return true;
  if (!hasTestFile(srcFile)) {
    warn(`No test file found for ${basename(srcFile)}. Consider writing tests first (TDD).`);
  }
  const task = loadActiveTask();
  if (task && task.requiresTdd && !hasTestFile(srcFile)) {
    die(`TDD guard: Task "${task.name}" requires TDD but no test file exists for ${basename(srcFile)}. Write tests first.`);
  }
  return true;
}

const SECRET_PATTERNS = [
  { re: /sk-[a-zA-Z0-9]{20,}/, name: "OpenAI API key" },
  { re: /sk-proj-[a-zA-Z0-9]{20,}/, name: "Anthropic API key" },
  { re: /AIza[a-zA-Z0-9]{30,}/, name: "Google API key" },
  { re: /ghp_[a-zA-Z0-9]{30,}/, name: "GitHub PAT" },
  { re: /gho_[a-zA-Z0-9]{30,}/, name: "GitHub OAuth token" },
  { re: /ghs_[a-zA-Z0-9]{30,}/, name: "GitHub app token" },
  { re: /["']?password["']?\s*[:=]\s*["'][^"']{8,}["']/i, name: "Hardcoded password" },
  { re: /["']?secret["']?\s*[:=]\s*["'][^"']{8,}["']/i, name: "Hardcoded secret" },
  { re: /["']?api[-_]?key["']?\s*[:=]\s*["'][^"']{8,}["']/i, name: "Hardcoded API key value" },
];

function checkSecrets(filePath) {
  if (!fileExists(filePath)) return true;
  const content = readFileSync(filePath, "utf-8");
  const lines = content.split("\n");
  let found = false;
  for (let i = 0; i < lines.length; i++) {
    for (const { re, name } of SECRET_PATTERNS) {
      if (re.test(lines[i])) {
        log("FAIL", `${name} detected at line ${i + 1} in ${basename(filePath)}`);
        found = true;
      }
    }
  }
  if (found) die("Secrets detected! Remove hardcoded credentials before proceeding.");
  return true;
}

function checkConsoleLog(filePath) {
  if (!fileExists(filePath)) return true;
  if (!isSourceFile(filePath)) return true;
  const content = readFileSync(filePath, "utf-8");
  const hasConsoleLog = /\bconsole\.log\s*\(/.test(content);
  if (hasConsoleLog) {
    warn(`console.log found in ${basename(filePath)}. Remove before committing.`);
  }
  return !hasConsoleLog;
}

const KNOWN_NAMESPACES = [
  "workflow", "memory", "semantic-search", "policy", "capability",
  "filesystem", "review", "session", "local", "git", "npm",
  "docker", "cicd", "github", "tests", "reminder", "projectRules",
];

function checkToolNames(mcpFilePath) {
  if (!fileExists(mcpFilePath)) return true;
  const content = readFileSync(mcpFilePath, "utf-8");

  // If the file has a dot-to-underscore converter, tool names with dots are intentional
  const hasConverter = /dotToUnderscore|dot.*underscore|registerTool.*wrapper|convertName/i.test(content);

  const toolNameRe = /server\.registerTool\(\s*["']([^"']+)["']/g;
  let match;
  let allValid = true;
  while ((match = toolNameRe.exec(content)) !== null) {
    const name = match[1];
    if (name.includes(".")) {
      if (hasConverter) {
        const converted = name.replace(/\./g, "_");
        const ns = converted.split("_")[0];
        if (!KNOWN_NAMESPACES.includes(ns)) {
          log("FAIL", `Unknown namespace "${ns}" in converted tool "${converted}" (source: "${name}")`);
          allValid = false;
        }
      } else {
        log("FAIL", `Dot in tool name "${name}" — use underscores: ${name.replace(/\./g, "_")}`);
        allValid = false;
      }
    } else {
      const ns = name.split("_")[0];
      if (!KNOWN_NAMESPACES.includes(ns)) {
        log("FAIL", `Unknown namespace "${ns}" in tool "${name}"`);
        allValid = false;
      }
    }
  }
  if (!allValid) die("Invalid tool names detected in MCP server. Fix before committing.");
  return allValid;
}

// --- Command Handlers ---

function preEdit(filePath) {
  console.log(`\n[masday guard] pre-edit: ${basename(filePath)}`);
  checkSecrets(filePath);
  checkConsoleLog(filePath);
  checkTdd(filePath);
  log("INFO", "Remember: Use masday MCP tools first → agent orchestrator → sub-agents → masday skills");
  return true;
}

function postEdit(filePath) {
  console.log(`\n[masday guard] post-edit: ${basename(filePath)}`);
  if (isSourceFile(filePath)) {
    log("INFO", "After editing source: run tests, workflow_saveProgress, memory_store, review_submit");
  }
  return true;
}

function getStagedFiles() {
  try {
    const output = execSync("git diff --cached --name-only --diff-filter=ACM", { encoding: "utf-8" });
    return output.trim().split("\n").filter(Boolean);
  } catch {
    return [];
  }
}

function preCommit() {
  const staged = getStagedFiles();
  if (staged.length === 0) {
    console.log("[masday guard] No staged files to check.");
    return true;
  }
  console.log(`\n[masday guard] pre-commit: checking ${staged.length} staged files...\n`);

  let blocked = false;

  for (const f of staged) {
    const fullPath = join(process.cwd(), f);
    if (!isSourceFile(f)) continue;

    if (!checkSecrets(fullPath)) blocked = true;
    checkConsoleLog(fullPath);

    if (isSourceFile(f) && !isTestFile(f) && !hasTestFile(fullPath)) {
      log("WARN", `No test file for ${f}. Consider adding tests.`);
    }
  }

  // Check tool names in MCP server file
  const mcpFile = join(process.cwd(), "apps/agent-runner/src/runtime/mcp.ts");
  if (staged.some((f) => f.includes("mcp.ts")) && fileExists(mcpFile)) {
    if (!checkToolNames(mcpFile)) blocked = true;
  }

  if (blocked) die("Pre-commit checks failed. Fix the issues above before committing.");
  log("PASS", "All pre-commit checks passed.");
  return true;
}

function preComplete() {
  console.log("\n[masday guard] pre-complete: checking review gate...");
  const masdayDir = join(process.cwd(), ".masday");
  if (!existsSync(masdayDir)) {
    log("INFO", "No .masday/ directory — skipping review gate check.");
    return true;
  }
  const reviewFile = join(masdayDir, "latest-review.json");
  if (!existsSync(reviewFile)) {
    die("Review gate: No review found. Run review_submit with APPROVED before completing task.");
  }
  try {
    const review = JSON.parse(readFileSync(reviewFile, "utf-8"));
    if (review.decision !== "APPROVED") {
      die(`Review gate: Latest review is "${review.decision}", not APPROVED. Fix issues and re-submit.`);
    }
    log("PASS", `Review gate: APPROVED (reviewer: ${review.reviewer_agent || "unknown"})`);
  } catch {
    die("Review gate: Could not read review file. Run review_submit first.");
  }
  return true;
}

// --- Main Router ---

const command = process.argv[2];
const arg = process.argv[3];

switch (command) {
  case "pre-edit":
    if (!arg) die("Usage: node scripts/guard.mjs pre-edit <file>");
    preEdit(arg);
    break;
  case "post-edit":
    if (!arg) die("Usage: node scripts/guard.mjs post-edit <file>");
    postEdit(arg);
    break;
  case "pre-commit":
  case "check-staged":
    preCommit();
    break;
  case "pre-complete":
    preComplete();
    break;
  default:
    console.log("Usage: node scripts/guard.mjs <command> [file]");
    console.log("");
    console.log("Commands:");
    console.log("  pre-edit <file>      TDD check, secret detection, masday-first reminder");
    console.log("  post-edit <file>     Wrap-back reminder (saveProgress, review_submit)");
    console.log("  pre-commit           Check staged files for secrets, console.log, TDD, tool names");
    console.log("  pre-complete         Review gate verification");
    console.log("  check-staged         Alias for pre-commit");
    process.exit(1);
}
