#!/usr/bin/env node
//
// Masday Session Start — context bootstrap for masday-workflow-rust
//
// Runs on SessionStart when in masday project:
//   1. Check DB connectivity
//   2. Check Redis connectivity
//   3. Check if API/MCP binaries exist and are running
//   4. Verify API health endpoint
//   5. Check MCP process
//   6. Show quick context
//   7. Initialize compact-state.json if missing
//

const { execSync } = require("child_process");
const net = require("net");
const http = require("http");
const path = require("path");
const fs = require("fs");
const os = require("os");

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

  // Redis
  const redisUp = await isPortOpen(6379);
  lines.push(`Redis (6379): ${redisUp ? "✅ running" : "⚠️ not running — docker compose up -d redis"}`);

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

  // API health check — verify /api/health responds
  let apiHealthy = false;
  try {
    apiHealthy = await new Promise((resolve) => {
      const req = http.get("http://localhost:3010/api/health", { timeout: 2000 }, (res) => {
        resolve(res.statusCode === 200);
      });
      req.on("error", () => resolve(false));
      req.on("timeout", () => { req.destroy(); resolve(false); });
    });
  } catch {}
  if (apiHealthy) {
    lines.push("API (3010): ✅ healthy");
  } else {
    const portOpen = await isPortOpen(3010);
    lines.push(`API (3010): ${portOpen ? "⚠️ port open but /api/health failing" : "❌ not running — cargo run --release -p masday-api"}`);
  }

  // MCP process check
  let mcpRunning = false;
  try {
    const result = execSync("pgrep -f masday-mcp 2>/dev/null || true", { encoding: "utf-8" }).trim();
    mcpRunning = result.length > 0;
  } catch {}
  const mcpBin = `${PROJECT}/target/release/masday-mcp`;
  const mcpExists = fs.existsSync(mcpBin);
  if (mcpRunning) {
    lines.push("MCP: ✅ running");
  } else if (mcpExists) {
    lines.push("MCP: ⚠️ binary exists but not running");
  } else {
    lines.push("MCP: ❌ not built — cargo build --release -p masday-mcp");
  }

  // Git branch
  try {
    const branch = execSync(`cd ${PROJECT} && git rev-parse --abbrev-ref HEAD`, { encoding: "utf-8" }).trim();
    const dirty = execSync(`cd ${PROJECT} && git status --porcelain`, { encoding: "utf-8" }).trim();
    lines.push(`Branch: ${branch}${dirty ? ` (${dirty.split("\n").length} changes)` : " (clean)"}`);
  } catch {}

  console.log(lines.join("\n"));

  // Initialize compact-state.json if missing (needed by statusline)
  const compactStatePath = path.join(os.homedir(), ".claude", "compact-state.json");
  if (!fs.existsSync(compactStatePath)) {
    try {
      fs.writeFileSync(compactStatePath, JSON.stringify({ count: 0, lastCompact: null }, null, 2));
    } catch {}
  }

  // Reset context cache to 0 — fresh session
  const contextCachePath = path.join(os.tmpdir(), "masday-context-cache.json");
  try {
    fs.writeFileSync(contextCachePath, JSON.stringify({ pct: 0, ts: Date.now() }));
  } catch {}
}

main().catch(() => {});
