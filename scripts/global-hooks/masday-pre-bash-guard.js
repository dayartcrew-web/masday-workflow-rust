#!/usr/bin/env node
//
// Masday Pre-Bash Guard — protect masday infrastructure
//
// Runs on PreToolUse (Bash) in masday-workflow-rust:
//   - Block destructive DB operations
//   - Warn when stopping containers
//   - Warn about cargo clean (big rebuild)
//

const fs = require("fs");
const path = require("path");

function main() {
  const cwd = process.cwd();
  // Only activate in masday project directories
  const hasMasdayDir = fs.existsSync(path.join(cwd, ".masday")) ||
    fs.existsSync(path.join(cwd, "masday-core")) ||
    fs.existsSync(path.join(cwd, ".claude", "agents"));
  if (!hasMasdayDir) return;

  const input = JSON.parse(process.argv[2] || "{}");
  const command = (input.command || "").toLowerCase();

  // Block dangerous DB operations
  const dangerPatterns = [
    "drop database",
    "drop table",
    "truncate table",
    "drop schema",
    "delete from workflows",
    "delete from tasks",
    "delete from memories",
  ];

  for (const pattern of dangerPatterns) {
    if (command.includes(pattern)) {
      console.log(`🚫 Masday Guard: destructive operation "${pattern}" detected`);
      console.log("Proceed only with explicit approval.");
      return;
    }
  }

  // Warn about cargo clean
  if (command.includes("cargo clean")) {
    console.log("⚠️ Masday: cargo clean will require full rebuild (~2-5 min). Use with caution.");
    return;
  }

  // Warn about docker compose down
  if (command.includes("docker compose down") || command.includes("docker-compose down")) {
    if (command.includes("masday") || cwd.includes("masday")) {
      console.log("⚠️ Masday: stopping all containers. Active sessions will be interrupted.");
    }
  }
}

try { main(); } catch {}
