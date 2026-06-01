#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
HOOKS_SRC="${ROOT_DIR}/scripts/git-hooks"
HOOKS_DST="${ROOT_DIR}/.git/hooks"
GLOBAL_HOOKS_SRC="${ROOT_DIR}/scripts/global-hooks"
HOME_CLAUDE="${HOME}/.claude"

echo "=== Installing Hooks ==="
echo ""

# --- Git hooks ---
if [ -d "${HOOKS_DST}" ]; then
  echo "[git hooks]"
  for hook in pre-commit pre-push; do
    if [ -f "${HOOKS_SRC}/${hook}" ]; then
      cp "${HOOKS_SRC}/${hook}" "${HOOKS_DST}/${hook}"
      chmod +x "${HOOKS_DST}/${hook}"
      echo "  ✅ ${hook}"
    fi
  done
else
  echo "  ⚠️  .git/hooks not found — skipping git hooks"
fi

echo ""

# --- Global Claude hooks ---
if [ -d "${GLOBAL_HOOKS_SRC}" ]; then
  echo "[global claude hooks → ${HOME_CLAUDE}/hooks/]"
  mkdir -p "${HOME_CLAUDE}/hooks" 2>/dev/null || true
  copied=0
  for hook in "${GLOBAL_HOOKS_SRC}"/*.js "${GLOBAL_HOOKS_SRC}"/*.cjs "${GLOBAL_HOOKS_SRC}"/*.mjs; do
    [ -f "$hook" ] || continue
    name="$(basename "$hook")"
    cp "$hook" "${HOME_CLAUDE}/hooks/${name}" 2>/dev/null || true
    echo "  ✅ ${name}"
    copied=$((copied + 1))
  done
  echo "  ${copied} hooks installed"

  # Update statusline + autoCompact in global settings.json
  SETTINGS_FILE="${HOME_CLAUDE}/settings.json"
  if [ -f "$SETTINGS_FILE" ]; then
    echo ""
    echo "[updating ${SETTINGS_FILE}]"
    node -e "
      const fs = require('fs');
      const sf = '${SETTINGS_FILE}';
      const s = JSON.parse(fs.readFileSync(sf, 'utf8'));
      s.statusLine = { type: 'command', command: 'node \"${HOME_CLAUDE}/hooks/masday-statusline.js\"' };
      s.autoCompact = true;
      s.autoCompactThreshold = 0.9;
      fs.writeFileSync(sf, JSON.stringify(s, null, 2) + '\n');
      console.log('  ✅ statusLine → masday-statusline.js');
      console.log('  ✅ autoCompact → true, threshold → 0.9');
    " 2>/dev/null || echo "  ⚠️  Could not update settings.json"
  else
    echo "  ⚠️  No ${SETTINGS_FILE} found — skipping settings update"
  fi
else
  echo "[global claude hooks] No ${GLOBAL_HOOKS_SRC}/ found — skipping"
fi

echo ""
echo "Hooks installed."
echo "  git:        pre-commit (fmt+lint), pre-push (build+test)"
echo "  global:     statusline, session-start, compact, context-warning, bash-guard"
echo "  autoCompact: threshold 0.9 (compact at 90% context)"
