#!/usr/bin/env bash
# Wrapper for Claude Code hooks — ensures node is found via nvm
# Usage: run.sh <hook-name>

# Load nvm if available
export NVM_DIR="${NVM_DIR:-$HOME/.nvm}"
[ -s "$NVM_DIR/nvm.sh" ] && . "$NVM_DIR/nvm.sh" 2>/dev/null

exec node "$(dirname "$0")/run-hook.mjs" "$@"
