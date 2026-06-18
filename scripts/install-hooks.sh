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
  for hook in pre-commit pre-commit-docs pre-push; do
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

  # Update statusline + autoCompact + hooks in global settings.json
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

      // Initialize hooks object if not present
      if (!s.hooks) s.hooks = {};

      // SessionStart hook (masday-mem-context)
      if (!s.hooks.SessionStart) {
        s.hooks.SessionStart = [];
      }
      const hasSessionStart = s.hooks.SessionStart.some(h =>
        h.hooks && h.hooks.some(hh => hh.command && hh.command.includes('masday-mem-context'))
      );
      if (!hasSessionStart) {
        s.hooks.SessionStart.push({
          matcher: '',
          hooks: [{ type: 'command', command: 'node \"${HOME_CLAUDE}/hooks/run-hook.mjs masday-mem-context\"', timeout: 15 }]
        });
      }

      // PreToolUse hooks (workflow-lock, skill-wrap-guard, skill-step-guard, pre-task-complete)
      if (!s.hooks.PreToolUse) {
        s.hooks.PreToolUse = [];
      }

      // workflow-lock (Edit|Write|MultiEdit|Bash|Agent)
      let lockEntry = s.hooks.PreToolUse.find(e => e.matcher === 'Edit|Write|MultiEdit|Bash|Agent');
      if (!lockEntry) {
        lockEntry = { matcher: 'Edit|Write|MultiEdit|Bash|Agent', hooks: [] };
        s.hooks.PreToolUse.push(lockEntry);
      }
      if (!lockEntry.hooks.some(h => h.command && h.command.includes('workflow-lock'))) {
        lockEntry.hooks.push({ type: 'command', command: 'node \"${HOME_CLAUDE}/hooks/run-hook.mjs workflow-lock\"', timeout: 10 });
      }

      // skill-wrap-guard and skill-step-guard (Skill|mcp execute)
      let skillEntry = s.hooks.PreToolUse.find(e => e.matcher === 'Skill|mcp__masday__workflow_execute|mcp__workflow-orchestrator__workflow_execute');
      if (!skillEntry) {
        skillEntry = { matcher: 'Skill|mcp__masday__workflow_execute|mcp__workflow-orchestrator__workflow_execute', hooks: [] };
        s.hooks.PreToolUse.push(skillEntry);
      }
      if (!skillEntry.hooks.some(h => h.command && h.command.includes('skill-wrap-guard'))) {
        skillEntry.hooks.push({ type: 'command', command: 'node \"${HOME_CLAUDE}/hooks/run-hook.mjs skill-wrap-guard\"', timeout: 10 });
      }
      if (!skillEntry.hooks.some(h => h.command && h.command.includes('skill-step-guard'))) {
        skillEntry.hooks.push({ type: 'command', command: 'node \"${HOME_CLAUDE}/hooks/run-hook.mjs skill-step-guard\"', timeout: 10 });
      }

      // pre-task-complete (mcp workflow_completeTask)
      let taskEntry = s.hooks.PreToolUse.find(e => e.matcher === 'mcp__masday__workflow_completeTask|mcp__workflow-orchestrator__workflow_complete_task');
      if (!taskEntry) {
        taskEntry = { matcher: 'mcp__masday__workflow_completeTask|mcp__workflow-orchestrator__workflow_complete_task', hooks: [] };
        s.hooks.PreToolUse.push(taskEntry);
      }
      if (!taskEntry.hooks.some(h => h.command && h.command.includes('pre-task-complete'))) {
        taskEntry.hooks.push({ type: 'command', command: 'node \"${HOME_CLAUDE}/hooks/run-hook.mjs pre-task-complete\"', timeout: 10 });
      }

      // PostToolUse hook (skill-step-guard for validation tools)
      if (!s.hooks.PostToolUse) {
        s.hooks.PostToolUse = [];
      }
      let postEntry = s.hooks.PostToolUse.find(e => e.matcher === 'mcp__masday__policy_validate_completion|mcp__masday__policy_validate_execution|mcp__masday__review_submit|mcp__masday__workflow_saveProgress');
      if (!postEntry) {
        postEntry = { matcher: 'mcp__masday__policy_validate_completion|mcp__masday__policy_validate_execution|mcp__masday__review_submit|mcp__masday__workflow_saveProgress', hooks: [] };
        s.hooks.PostToolUse.push(postEntry);
      }
      if (!postEntry.hooks.some(h => h.command && h.command.includes('skill-step-guard'))) {
        postEntry.hooks.push({ type: 'command', command: 'node \"${HOME_CLAUDE}/hooks/run-hook.mjs skill-step-guard\"', timeout: 10 });
      }

      // Stop hook (on-stop)
      if (!s.hooks.Stop) {
        s.hooks.Stop = [];
      }
      if (!s.hooks.Stop.some(e => e.hooks && e.hooks.some(h => h.command && h.command.includes('on-stop')))) {
        s.hooks.Stop.push({
          matcher: '',
          hooks: [{ type: 'command', command: 'node \"${HOME_CLAUDE}/hooks/run-hook.mjs on-stop\"', timeout: 30 }]
        });
      }

      fs.writeFileSync(sf, JSON.stringify(s, null, 2) + '\n');
      console.log('  ✅ statusLine → masday-statusline.js');
      console.log('  ✅ autoCompact → true, threshold → 0.9');
      console.log('  ✅ hooks → SessionStart, PreToolUse, PostToolUse, Stop registered');
    " 2>/dev/null || echo "  ⚠️  Could not update settings.json"
  else
    echo "  ⚠️  No ${SETTINGS_FILE} found — skipping settings update"
  fi
else
  echo "[global claude hooks] No ${GLOBAL_HOOKS_SRC}/ found — skipping"
fi

echo ""
echo "Hooks installed."
echo "  git:        pre-commit (fmt+lint+docs), pre-commit-docs, pre-push (build+test)"
echo "  global:     statusline, session-start, compact, context-warning, bash-guard"
echo "  autoCompact: threshold 0.9 (compact at 90% context)"
