#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
HOOKS_SRC="${ROOT_DIR}/scripts/git-hooks"
HOOKS_DST="${ROOT_DIR}/.git/hooks"

if [ ! -d "${HOOKS_DST}" ]; then
  echo "Error: .git/hooks not found — are you in a git repository?"
  exit 1
fi

for hook in pre-commit pre-push; do
  if [ -f "${HOOKS_SRC}/${hook}" ]; then
    cp "${HOOKS_SRC}/${hook}" "${HOOKS_DST}/${hook}"
    chmod +x "${HOOKS_DST}/${hook}"
    echo "  ✅ ${hook}"
  fi
done

echo ""
echo "Git hooks installed. Multi-stack auto-detect on commit/push."
echo "  pre-commit: fmt + lint (Rust, TS, Python, Go, PHP)"
echo "  pre-push:   build + test (Rust, TS, Python, Go, PHP)"
