#!/usr/bin/env bash
# Comprehensive RPC test for all MCP tools.
#
# This is a thin wrapper around tests/test-all-tools-rpc.py, which exercises
# every tool via stdin JSON-RPC to the masday-mcp binary. Each tool runs in its
# OWN freshly spawned MCP process with its own timeout, so a single blocking
# tool (e.g. tests_run -> `cargo test`) can never starve the queue or mask
# another tool's result. (An earlier in-process batched version of this test
# did exactly that: tests_run blocked the sequential server and only the calls
# ahead of it ever returned.)
#
# Verdict per tool: OK / TOOL_ERR (graceful) / RPC_ERROR / TIMEOUT / CRASH.
# Exit code is non-zero iff any RPC_ERROR / TIMEOUT / CRASH occurred.
#
# Environment:
#   MCP_BIN         path to the masday-mcp binary (else auto-resolved from repo target/)
#   FAST_TIMEOUT    per-call budget for ordinary tools, seconds (default 12)
#   SLOW_TIMEOUT    per-call budget for slow CLI/network tools (default 45)
#   TEST_PROJECT    project path for capability/filesystem tools (default: repo root)
#
# Usage:
#   bash tests/test-all-tools-rpc.sh
#   MCP_BIN=./target/release/masday-mcp bash tests/test-all-tools-rpc.sh
set -euo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

if ! command -v python3 >/dev/null 2>&1; then
    echo "ERROR: python3 is required to run this harness" >&2
    exit 2
fi

exec python3 "$HERE/test-all-tools-rpc.py" "$@"
