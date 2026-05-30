#!/bin/bash
# Safe Rust cleanup — preserves dependency cache
# This script cleans release artifacts and docs without removing the full target directory,
# preserving the dependency cache for faster rebuilds.

set -e

echo "Cleaning Rust build artifacts..."
echo ""

# Clean release builds
if cargo clean --release 2>/dev/null; then
    echo "✓ Cleaned release artifacts"
else
    echo "✗ No release artifacts to clean"
fi

# Clean documentation
if cargo clean --doc 2>/dev/null; then
    echo "✓ Cleaned documentation"
else
    echo "✗ No documentation to clean"
fi

echo ""
echo "Cleanup complete. Dependency cache preserved."
echo "Run 'cargo clean' for full cleanup if needed."
