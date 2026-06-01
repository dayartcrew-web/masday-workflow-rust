#!/usr/bin/env node
//
// Masday Session Start — context bootstrap for masday-workflow-rust
//
// Runs on SessionStart when in masday project:
//   1. Check DB connectivity
//   2. Check if API/MCP binaries exist
//   3. Show quick context
//

const { execSync } = require("child_process");
const net = require("net");
const fs = require("fs");

const PROJECT = "/home/vibe-dev/masday-workflow-rust";

function isPortOpen(port) {
  return new Promise((resolve) => {
    const sock = new net.Socket();
    sock.setTimeout(1000);
    sock.on("connect", () => { sock.destroy(); resolve(true); });
    sock.on("error", () => resolve(false));
    sock.on("timeout", () => { sock.destroy(); resolve(false); });
    sock.connect(port, "localhost");
  });
}

async function main() {
  const cwd = process.cwd();
  if (!cwd.includes("masday-workflow-rust") && !cwd.includes("masday-workflow-rebuild")) {
    return; // Not a Masday session
  }

  const lines = [];

  // Database
  const dbUp = await isPortOpen(5434);
  lines.push(`PostgreSQL (5434): ${dbUp ? "✅ running" : "❌ not running — docker compose up -d postgres"}`);

  // Binaries
  const bins = [
    { name: "masday-api", path: `${PROJECT}/target/release/masday-api` },
    { name: "masday-mcp", path: `${PROJECT}/target/release/masday-mcp` },
  ];
  for (const bin of bins) {
    if (fs.existsSync(bin.path)) {
      const stat = fs.statSync(bin.path);
      const age = Date.now() - stat.mtimeMs;
      const hours = Math.floor(age / 3600000);
      lines.push(`${bin.name}: ✅ built${hours < 24 ? ` (${hours}h ago)` : " (stale — rebuild?)"}`);
    } else {
      lines.push(`${bin.name}: ❌ not built — cargo build --release -p ${bin.name}`);
    }
  }

  // API running?
  const apiUp = await isPortOpen(3010);
  lines.push(`API (3010): ${apiUp ? "✅ running" : "⚠️ not running — cargo run --release -p masday-api"}`);

  // Git branch
  try {
    const branch = execSync(`cd ${PROJECT} && git rev-parse --abbrev-ref HEAD`, { encoding: "utf-8" }).trim();
    const dirty = execSync(`cd ${PROJECT} && git status --porcelain`, { encoding: "utf-8" }).trim();
    lines.push(`Branch: ${branch}${dirty ? ` (${dirty.split("\n").length} changes)` : " (clean)"}`);
  } catch {}

  console.log(lines.join("\n"));
}

main().catch(() => {});
