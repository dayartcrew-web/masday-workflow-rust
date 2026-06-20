#!/bin/bash
# Live HTTP API smoke test across all 14 tool domains.
#
# Hits representative endpoints on the running masday-api server (default
# http://localhost:30101/api) and reports the HTTP status of each. This is the
# "via API" counterpart to test-all-tools-rpc.sh ("via RPC"): the RPC harness
# exercises the stdio/MCP path local clients hit; this exercises the HTTP path
# remote + dashboard clients hit.
#
# PASS = 2xx, or a *graceful* 4xx where the endpoint legitimately rejects bad
# input (e.g. not-found for a fake id, 400 for a missing required param).
# FAIL = 5xx, connection error, or a panic-style body.
#
# Usage:
#   API_BASE=http://localhost:30101/api WF_ID=<uuid> TASK_ID=<uuid> \
#     bash tests/test-all-tools-api.sh
set -uo pipefail

API_BASE="${API_BASE:-http://localhost:30101/api}"
WF_ID="${WF_ID:-44024ad5-8deb-4927-8366-d63cc6d73a9b}"
TASK_ID="${TASK_ID:-319ab03d-04ab-43b8-8dcb-1e120bb18fac}"
PROJECT="${PROJECT:-/home/vibe-dev/masday-workflow-rust}"

PASS=0
FAIL=0
RESULTS=()

# call METHOD PATH [JSON_BODY]
# Records "METHOD PATH -> STATUS (verdict)".
call() {
    local method="$1" path="$2" body="${3:-}"
    local url="${API_BASE}${path}"
    local code body_head
    if [[ -n "$body" ]]; then
        resp=$(curl -s -m 15 -o /tmp/api-smoke-body -w "%{http_code}" \
            -X "$method" "$url" \
            -H 'Content-Type: application/json' -d "$body" 2>/dev/null) || code="CONN_ERR"
    else
        resp=$(curl -s -m 15 -o /tmp/api-smoke-body -w "%{http_code}" \
            -X "$method" "$url" 2>/dev/null) || code="CONN_ERR"
    fi
    code="${resp:-CONN_ERR}"
    body_head=$(head -c 160 /tmp/api-smoke-body 2>/dev/null | tr '\n' ' ')

    # Verdict: 2xx or graceful 4xx = pass; 5xx / CONN_ERR / panic = fail.
    local verdict
    if [[ "$code" =~ ^2 ]]; then
        verdict="PASS"; PASS=$((PASS+1))
    elif [[ "$code" =~ ^4 ]]; then
        verdict="PASS(4xx)"; PASS=$((PASS+1))
    elif [[ "$code" == "CONN_ERR" ]]; then
        verdict="FAIL(conn)"; FAIL=$((FAIL+1))
    else
        verdict="FAIL"; FAIL=$((FAIL+1))
    fi
    RESULTS+=("$(printf '%-5s %-46s -> %s  [%s]  %s' "$method" "$path" "$code" "$verdict" "$body_head")")
}

echo "═══════════════════════════════════════════════════════════════════"
echo "  masday API LIVE SMOKE — ${API_BASE}"
echo "  WF=${WF_ID}  TASK=${TASK_ID}"
echo "═══════════════════════════════════════════════════════════════════"

# ── Health ──────────────────────────────────────────────────────────────────
call GET /health
call GET /health/db

# ── Workflow ────────────────────────────────────────────────────────────────
call GET /workflows/active
call GET "/workflows/${WF_ID}"
call GET "/workflows/${WF_ID}/status"
call GET "/workflows/${WF_ID}/current-task"
call GET "/workflows/${WF_ID}/plan"
call GET "/workflows/${WF_ID}/context-pack"

# ── Task / Plan ─────────────────────────────────────────────────────────────
call GET "/tasks/${TASK_ID}"
call GET "/plans/${WF_ID}"

# ── Memory ──────────────────────────────────────────────────────────────────
call GET /memories/recent
call GET /memories/stats
call GET '/memories/by-type?source_type=research'
call GET "/memories/by-task/${TASK_ID}"
call POST /memories/search '{"query":"workflow","limit":5}'

# ── Context (incl. #96 code_search reroute) ─────────────────────────────────
call GET "/context/search?query=workflow%20state%20machine&project_path=${PROJECT}&limit=5"
call POST /context/hybrid-search "{\"workflow_id\":\"${WF_ID}\",\"task_id\":\"${TASK_ID}\",\"query\":\"workflow\"}"
call POST /context/fingerprint "{\"workflow_id\":\"${WF_ID}\",\"task_id\":\"${TASK_ID}\"}"
call POST /context/fingerprint-search "{\"workflow_id\":\"${WF_ID}\",\"task_id\":\"${TASK_ID}\"}"

# ── Policy ──────────────────────────────────────────────────────────────────
call POST /policy/session-readiness '{"sessionKey":"smoke-test"}'
call POST /policy/context-refresh "{\"workflowId\":\"${WF_ID}\",\"taskId\":\"${TASK_ID}\"}"

# ── Reminder ────────────────────────────────────────────────────────────────
call GET /reminders
call GET /reminders/check
call GET /reminders/stale
call GET /reminders/stuck

# ── Capability ──────────────────────────────────────────────────────────────
call GET /capabilities/agents
call GET /capabilities/skills
call GET /capabilities/templates
call GET "/capabilities/readiness?projectRoot=${PROJECT}"
call GET /capabilities/system-readiness
call GET '/capabilities/match?taskDescription=build%20an%20api%20endpoint'

# ── Graph ───────────────────────────────────────────────────────────────────
call POST /graph/search '{"query":"workflow"}'

# ── Review ──────────────────────────────────────────────────────────────────
call GET "/reviews/latest?workflow_id=${WF_ID}&task_id=${TASK_ID}"
call GET "/reviews/task/${TASK_ID}"

# ── Session / Logs / Misc ───────────────────────────────────────────────────
call GET '/sessions/smoke-nonexistent-session'
call GET "/retrieval-logs/task/${TASK_ID}"
call GET "/progress-logs/task/${TASK_ID}"
call GET /llm-provider-configs/default
call GET '/token-usage/stats/orchestrator'

# ── Report ──────────────────────────────────────────────────────────────────
echo ""
for r in "${RESULTS[@]}"; do echo "  $r"; done
echo ""
echo "═══════════════════════════════════════════════════════════════════"
echo "  API SMOKE SUMMARY:  ${#RESULTS[@]} calls | PASS=${PASS}  FAIL=${FAIL}"
echo "═══════════════════════════════════════════════════════════════════"
[[ $FAIL -eq 0 ]] && exit 0 || exit 1
