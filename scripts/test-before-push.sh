#!/bin/bash
# Pre-push quality gate — run before any commit + tag + push
set -e
echo "Running quality gate..."

echo "1/3 cargo check..."
cargo check -p masday-cli -p masday-mcp 2>&1 | tail -1

echo "2/3 cargo clippy..."
cargo clippy -p masday-cli -p masday-mcp -- -D warnings 2>&1 | tail -1

echo "3/3 cargo fmt..."
cargo fmt --check -p masday-cli -p masday-mcp 2>&1 | tail -1

echo ""
echo "✓ All checks passed — safe to push"
