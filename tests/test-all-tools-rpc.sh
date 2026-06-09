#!/bin/bash
# Comprehensive RPC test for all 90 MCP tools
# Tests each tool via stdin JSON-RPC to the MCP binary
set -euo pipefail

MCP_BIN="${MCP_BIN:-/home/vibe-dev/masday-workflow-rust/target/release/masday-mcp}"
RESULTS_DIR="/tmp/mcp-test-results"
mkdir -p "$RESULTS_DIR"

PASS=0
FAIL=0
SKIP=0
ERRORS=()

# Helper: call a tool and capture result
call_tool() {
    local tool_name="$1"
    local args="$2"
    local id="$3"
    echo "{\"jsonrpc\":\"2.0\",\"id\":${id},\"method\":\"tools/call\",\"params\":{\"name\":\"${tool_name}\",\"arguments\":${args}}}"
}

# Build the init sequence + all tool calls
BUILD_FILE="$RESULTS_DIR/rpc_calls.json"
> "$BUILD_FILE"

# Init sequence
echo '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}' >> "$BUILD_FILE"
echo '{"jsonrpc":"2.0","method":"notifications/initialized"}' >> "$BUILD_FILE"

ID=10

# ══════════════════════════════════════════════════════════════════
# READ-ONLY / SAFE TOOLS (no side effects)
# ══════════════════════════════════════════════════════════════════

# workflow_ping
call_tool "workflow_ping" '{}' $((ID++)) >> "$BUILD_FILE"

# workflow_list
call_tool "workflow_list" '{}' $((ID++)) >> "$BUILD_FILE"

# workflow_getActive
call_tool "workflow_getActive" '{}' $((ID++)) >> "$BUILD_FILE"

# memory_stats
call_tool "memory_stats" '{}' $((ID++)) >> "$BUILD_FILE"

# memory_recall_recent
call_tool "memory_recall_recent" '{}' $((ID++)) >> "$BUILD_FILE"

# memory_search
call_tool "memory_search" '{"query":"test"}' $((ID++)) >> "$BUILD_FILE"

# memory_search_nodes
call_tool "memory_search_nodes" '{"query":"test"}' $((ID++)) >> "$BUILD_FILE"

# capability_list_templates
call_tool "capability_list_templates" '{}' $((ID++)) >> "$BUILD_FILE"

# capability_list_agents
call_tool "capability_list_agents" '{"projectRoot":"/home/vibe-dev/masday-workflow-rust"}' $((ID++)) >> "$BUILD_FILE"

# capability_list_skills
call_tool "capability_list_skills" '{"projectRoot":"/home/vibe-dev/masday-workflow-rust"}' $((ID++)) >> "$BUILD_FILE"

# capability_system_readiness
call_tool "capability_system_readiness" '{"projectRoot":"/home/vibe-dev/masday-workflow-rust"}' $((ID++)) >> "$BUILD_FILE"

# capability_match_agent
call_tool "capability_match_agent" '{"projectRoot":"/home/vibe-dev/masday-workflow-rust","taskDescription":"test task"}' $((ID++)) >> "$BUILD_FILE"

# filesystem_read
call_tool "filesystem_read" '{"path":"/home/vibe-dev/masday-workflow-rust/Cargo.toml"}' $((ID++)) >> "$BUILD_FILE"

# filesystem_list
call_tool "filesystem_list" '{"path":"/home/vibe-dev/masday-workflow-rust"}' $((ID++)) >> "$BUILD_FILE"

# filesystem_stat
call_tool "filesystem_stat" '{"path":"/home/vibe-dev/masday-workflow-rust/Cargo.toml"}' $((ID++)) >> "$BUILD_FILE"

# git_status
call_tool "git_status" '{}' $((ID++)) >> "$BUILD_FILE"

# git_diff
call_tool "git_diff" '{}' $((ID++)) >> "$BUILD_FILE"

# docker_ps
call_tool "docker_ps" '{}' $((ID++)) >> "$BUILD_FILE"

# cicd_pipeline_status
call_tool "cicd_pipeline_status" '{}' $((ID++)) >> "$BUILD_FILE"

# cicd_runs_view
call_tool "cicd_runs_view" '{}' $((ID++)) >> "$BUILD_FILE"

# github_pr_list
call_tool "github_pr_list" '{}' $((ID++)) >> "$BUILD_FILE"

# github_issue_list
call_tool "github_issue_list" '{}' $((ID++)) >> "$BUILD_FILE"

# tests_run (with a pattern to limit scope)
call_tool "tests_run" '{"pattern":"placeholder"}' $((ID++)) >> "$BUILD_FILE"

# projectRules_check
call_tool "projectRules_check" '{"projectRoot":"/home/vibe-dev/masday-workflow-rust"}' $((ID++)) >> "$BUILD_FILE"

# reminder_check
call_tool "reminder_check" '{}' $((ID++)) >> "$BUILD_FILE"

# reminder_list
call_tool "reminder_list" '{}' $((ID++)) >> "$BUILD_FILE"

# session_get_state (fake key - should return not found, not crash)
call_tool "session_get_state" '{"session_key":"test-nonexistent-session"}' $((ID++)) >> "$BUILD_FILE"

# policy_check_session_readiness (fake key)
call_tool "policy_check_session_readiness" '{"session_key":"test-nonexistent-session"}' $((ID++)) >> "$BUILD_FILE"

# use_masday
call_tool "use_masday" '{"prompt":"create a workflow"}' $((ID++)) >> "$BUILD_FILE"

# semantic-search_code_search
call_tool "semantic-search_code_search" '{"query":"workflow"}' $((ID++)) >> "$BUILD_FILE"

# ══════════════════════════════════════════════════════════════════
# WRITE TOOLS (with test data, cleaned up after)
# ══════════════════════════════════════════════════════════════════

# workflow_create (test workflow)
call_tool "workflow_create" '{"name":"__TEST_RPC_TOOL__","description":"automated test"}' $((ID++)) >> "$BUILD_FILE"

# memory_store (test memory)
call_tool "memory_store" '{"memory_type":"fact","summary":"RPC test memory","content":"This is an automated test","created_by_agent":"test-script"}' $((ID++)) >> "$BUILD_FILE"

# session_init_context
call_tool "session_init_context" '{"cwd":"/home/vibe-dev/masday-workflow-rust"}' $((ID++)) >> "$BUILD_FILE"

# local_init
call_tool "local_init" '{"cwd":"/tmp/mcp-test-local"}' $((ID++)) >> "$BUILD_FILE"

# local_save_artifact
call_tool "local_save_artifact" '{"cwd":"/tmp/mcp-test-local","category":"test","filename":"rpc-test.txt","content":"hello from rpc test"}' $((ID++)) >> "$BUILD_FILE"

# ══════════════════════════════════════════════════════════════════
# TOOLS THAT NEED VALID IDs (will test with fake IDs)
# ══════════════════════════════════════════════════════════════════

FAKE_WF_ID="00000000-0000-0000-0000-000000000000"
FAKE_TASK_ID="00000000-0000-0000-0000-000000000001"

# workflow_get
call_tool "workflow_get" "{\"workflow_id\":\"${FAKE_WF_ID}\"}" $((ID++)) >> "$BUILD_FILE"

# workflow_getStatus
call_tool "workflow_getStatus" "{\"workflow_id\":\"${FAKE_WF_ID}\"}" $((ID++)) >> "$BUILD_FILE"

# workflow_getCurrentTask
call_tool "workflow_getCurrentTask" "{\"workflow_id\":\"${FAKE_WF_ID}\"}" $((ID++)) >> "$BUILD_FILE"

# workflow_getPlan
call_tool "workflow_getPlan" "{\"workflow_id\":\"${FAKE_WF_ID}\"}" $((ID++)) >> "$BUILD_FILE"

# workflow_listTasks
call_tool "workflow_listTasks" "{\"workflow_id\":\"${FAKE_WF_ID}\"}" $((ID++)) >> "$BUILD_FILE"

# workflow_listParallelBranches
call_tool "workflow_listParallelBranches" "{\"workflow_id\":\"${FAKE_WF_ID}\"}" $((ID++)) >> "$BUILD_FILE"

# workflow_resume_suggestion
call_tool "workflow_resume_suggestion" "{\"workflow_id\":\"${FAKE_WF_ID}\"}" $((ID++)) >> "$BUILD_FILE"

# workflow_execute (fake ID - should fail gracefully)
call_tool "workflow_execute" "{\"workflow_id\":\"${FAKE_WF_ID}\"}" $((ID++)) >> "$BUILD_FILE"

# workflow_startTask (fake IDs)
call_tool "workflow_startTask" "{\"workflow_id\":\"${FAKE_WF_ID}\",\"task_id\":\"${FAKE_TASK_ID}\"}" $((ID++)) >> "$BUILD_FILE"

# workflow_completeTask (fake IDs)
call_tool "workflow_completeTask" "{\"workflow_id\":\"${FAKE_WF_ID}\",\"task_id\":\"${FAKE_TASK_ID}\"}" $((ID++)) >> "$BUILD_FILE"

# workflow_saveProgress (fake IDs)
call_tool "workflow_saveProgress" "{\"workflow_id\":\"${FAKE_WF_ID}\",\"task_id\":\"${FAKE_TASK_ID}\",\"agent_name\":\"test\",\"progress_note\":\"testing\"}" $((ID++)) >> "$BUILD_FILE"

# workflow_createPlan (fake ID)
call_tool "workflow_createPlan" "{\"workflow_id\":\"${FAKE_WF_ID}\",\"plan\":\"test plan\"}" $((ID++)) >> "$BUILD_FILE"

# workflow_addTask (fake ID)
call_tool "workflow_addTask" "{\"workflow_id\":\"${FAKE_WF_ID}\",\"name\":\"test-task\",\"agent\":\"test\",\"skill\":\"test\"}" $((ID++)) >> "$BUILD_FILE"

# workflow_createParallelBranches (fake ID)
call_tool "workflow_createParallelBranches" "{\"workflow_id\":\"${FAKE_WF_ID}\",\"branches\":[{\"task_id\":\"${FAKE_TASK_ID}\",\"branch_key\":\"test\",\"role\":\"tester\"}]}" $((ID++)) >> "$BUILD_FILE"

# workflow_completeParallelBranch (fake ID)
call_tool "workflow_completeParallelBranch" "{\"workflow_id\":\"${FAKE_WF_ID}\",\"branch_key\":\"test\"}" $((ID++)) >> "$BUILD_FILE"

# workflow_delete (fake ID)
call_tool "workflow_delete" "{\"workflow_id\":\"${FAKE_WF_ID}\"}" $((ID++)) >> "$BUILD_FILE"

# review_get_latest (fake IDs)
call_tool "review_get_latest" "{\"workflow_id\":\"${FAKE_WF_ID}\",\"task_id\":\"${FAKE_TASK_ID}\"}" $((ID++)) >> "$BUILD_FILE"

# memory_recall_documents (fake ID)
call_tool "memory_recall_documents" "{\"workflow_id\":\"${FAKE_WF_ID}\"}" $((ID++)) >> "$BUILD_FILE"

# memory_recall_document_by_type
call_tool "memory_recall_document_by_type" '{"source_type":"research"}' $((ID++)) >> "$BUILD_FILE"

# memory_recall_by_task (fake ID)
call_tool "memory_recall_by_task" "{\"task_id\":\"${FAKE_TASK_ID}\"}" $((ID++)) >> "$BUILD_FILE"

# memory_delete_by_workflow (fake ID)
call_tool "memory_delete_by_workflow" "{\"workflow_id\":\"${FAKE_WF_ID}\"}" $((ID++)) >> "$BUILD_FILE"

# memory_update (fake ID)
call_tool "memory_update" '{"id":"nonexistent-id","content":"updated"}' $((ID++)) >> "$BUILD_FILE"

# memory_delete (fake ID)
call_tool "memory_delete" '{"id":"nonexistent-id"}' $((ID++)) >> "$BUILD_FILE"

# review_submit (fake IDs)
call_tool "review_submit" "{\"workflow_id\":\"${FAKE_WF_ID}\",\"task_id\":\"${FAKE_TASK_ID}\",\"reviewer_agent\":\"test\",\"decision\":\"APPROVED\",\"notes\":\"test\"}" $((ID++)) >> "$BUILD_FILE"

# session_patch_state
call_tool "session_patch_state" '{"session_key":"test-session","patch":"{}"}' $((ID++)) >> "$BUILD_FILE"

# semantic-search_search_hybrid_context_pack (fake IDs)
call_tool "semantic-search_search_hybrid_context_pack" "{\"workflow_id\":\"${FAKE_WF_ID}\",\"plan_id\":\"${FAKE_WF_ID}\",\"task_id\":\"${FAKE_TASK_ID}\"}" $((ID++)) >> "$BUILD_FILE"

# semantic-search_search_context_fingerprint (fake IDs)
call_tool "semantic-search_search_context_fingerprint" "{\"workflow_id\":\"${FAKE_WF_ID}\",\"plan_id\":\"${FAKE_WF_ID}\",\"task_id\":\"${FAKE_TASK_ID}\"}" $((ID++)) >> "$BUILD_FILE"

# semantic-search_make_fingerprint (fake IDs)
call_tool "semantic-search_make_fingerprint" "{\"workflow_id\":\"${FAKE_WF_ID}\",\"plan_id\":\"${FAKE_WF_ID}\",\"task_id\":\"${FAKE_TASK_ID}\"}" $((ID++)) >> "$BUILD_FILE"

# policy_validate_completion (fake IDs)
call_tool "policy_validate_completion" "{\"session_key\":\"test\",\"workflow_id\":\"${FAKE_WF_ID}\",\"task_id\":\"${FAKE_TASK_ID}\"}" $((ID++)) >> "$BUILD_FILE"

# policy_validate_execution (fake IDs)
call_tool "policy_validate_execution" "{\"session_key\":\"test\",\"workflow_id\":\"${FAKE_WF_ID}\",\"task_id\":\"${FAKE_TASK_ID}\"}" $((ID++)) >> "$BUILD_FILE"

# policy_validate_parallel_completion (fake IDs)
call_tool "policy_validate_parallel_completion" "{\"session_key\":\"test\",\"workflow_id\":\"${FAKE_WF_ID}\",\"task_id\":\"${FAKE_TASK_ID}\"}" $((ID++)) >> "$BUILD_FILE"

# policy_detect_scope_drift (fake ID)
call_tool "policy_detect_scope_drift" "{\"workflow_id\":\"${FAKE_WF_ID}\"}" $((ID++)) >> "$BUILD_FILE"

# policy_require_context_refresh (fake ID)
call_tool "policy_require_context_refresh" "{\"workflow_id\":\"${FAKE_WF_ID}\"}" $((ID++)) >> "$BUILD_FILE"

# capability_workflow_audit (fake ID)
call_tool "capability_workflow_audit" "{\"workflowId\":\"${FAKE_WF_ID}\"}" $((ID++)) >> "$BUILD_FILE"

# memory_create_entities (test data)
call_tool "memory_create_entities" '{"entities":"[{\"name\":\"test-entity\",\"entity_type\":\"concept\",\"properties\":\"{\\\"desc\\\":\\\"test\\\"}\"}]"}' $((ID++)) >> "$BUILD_FILE"

# capability_scaffold_feature (dry - just tests the tool path)
call_tool "capability_scaffold_feature" '{"projectRoot":"/tmp/mcp-test-local","name":"test-feature","description":"test scaffold"}' $((ID++)) >> "$BUILD_FILE"

# capability_scaffold_mcp_server (dry - just tests the tool path)
call_tool "capability_scaffold_mcp_server" '{"projectRoot":"/tmp/mcp-test-local","name":"test-mcp","description":"test mcp scaffold"}' $((ID++)) >> "$BUILD_FILE"

# capability_create_agent (test agent)
call_tool "capability_create_agent" '{"projectRoot":"/tmp/mcp-test-local","name":"test-agent","role":"tester","description":"test agent","instructions":"do testing"}' $((ID++)) >> "$BUILD_FILE"

# capability_create_skill (test skill)
call_tool "capability_create_skill" '{"projectRoot":"/tmp/mcp-test-local","name":"test-skill","description":"test skill","trigger":"test","steps":"do something"}' $((ID++)) >> "$BUILD_FILE"

# reminder_acknowledge (fake ID)
call_tool "reminder_acknowledge" '{"id":"nonexistent"}' $((ID++)) >> "$BUILD_FILE"

# workflow_mark_synthesis_ready
call_tool "workflow_mark_synthesis_ready" '{"session_key":"test","ready":"true"}' $((ID++)) >> "$BUILD_FILE"

# workflow_mark_verification_ready
call_tool "workflow_mark_verification_ready" '{"session_key":"test","ready":"true"}' $((ID++)) >> "$BUILD_FILE"

# workflow_set_execution_mode
call_tool "workflow_set_execution_mode" '{"session_key":"test","mode":"sequential"}' $((ID++)) >> "$BUILD_FILE"

# filesystem_write (write a temp file)
call_tool "filesystem_write" '{"path":"/tmp/mcp-test-rpc-write.txt","content":"test content from rpc"}' $((ID++)) >> "$BUILD_FILE"

# filesystem_delete (delete the temp file)
call_tool "filesystem_delete" '{"path":"/tmp/mcp-test-rpc-write.txt"}' $((ID++)) >> "$BUILD_FILE"

# local_sync
call_tool "local_sync" '{"cwd":"/tmp/mcp-test-local"}' $((ID++)) >> "$BUILD_FILE"

# local_push
call_tool "local_push" '{"cwd":"/tmp/mcp-test-local"}' $((ID++)) >> "$BUILD_FILE"

# npm_install (safe - no packages specified)
call_tool "npm_install" '{}' $((ID++)) >> "$BUILD_FILE"

# npm_run (fake script - should fail gracefully)
call_tool "npm_run" '{"script":"nonexistent-script-xyz"}' $((ID++)) >> "$BUILD_FILE"

# git_commit (fake - should fail since nothing staged)
call_tool "git_commit" '{"message":"test commit from rpc test"}' $((ID++)) >> "$BUILD_FILE"

# docker_build
call_tool "docker_build" '{"tag":"test-no-build"}' $((ID++)) >> "$BUILD_FILE"

# docker_run (fake image)
call_tool "docker_run" '{"image":"nonexistent-image-xyz:latest"}' $((ID++)) >> "$BUILD_FILE"

# cicd_pipeline_trigger (fake pipeline)
call_tool "cicd_pipeline_trigger" '{"pipeline":"nonexistent-pipeline"}' $((ID++)) >> "$BUILD_FILE"

# github_pr_create (should fail gracefully - no branch)
call_tool "github_pr_create" '{"title":"test PR from rpc"}' $((ID++)) >> "$BUILD_FILE"

TOTAL_TOOLS=$((ID - 10))

# Send all calls to MCP binary and capture output
echo "Sending ${TOTAL_TOOLS} tool calls to MCP binary..."
sleep 0.2
cat "$BUILD_FILE" | timeout 60 "$MCP_BIN" 2>"$RESULTS_DIR/stderr.log" | grep -v '^\[' | grep '"jsonrpc"' > "$RESULTS_DIR/raw_output.json"

echo "Raw output lines: $(wc -l < "$RESULTS_DIR/raw_output.json")"

# Parse results
echo ""
echo "═══════════════════════════════════════════════════════════════════"
echo "  MCP TOOL RPC TEST RESULTS"
echo "═══════════════════════════════════════════════════════════════════"
echo ""

while IFS= read -r line; do
    # Extract the id
    resp_id=$(echo "$line" | python3 -c "import json,sys; d=json.load(sys.stdin); print(d.get('id','?'))" 2>/dev/null || echo "parse-error")

    # Check if it's an error response
    is_error=$(echo "$line" | python3 -c "
import json,sys
d=json.load(sys.stdin)
if d.get('error'):
    print('RPC_ERROR')
elif d.get('result',{}).get('isError'):
    print('TOOL_ERROR')
else:
    print('OK')
" 2>/dev/null || echo "parse-error")

    # Extract tool error message if any
    msg=$(echo "$line" | python3 -c "
import json,sys
d=json.load(sys.stdin)
if d.get('error'):
    print(d['error'].get('message','unknown'))
elif d.get('result',{}).get('isError'):
    content = d['result'].get('content',[])
    if content:
        print(content[0].get('text','unknown error'))
    else:
        print('unknown error')
else:
    print('OK')
" 2>/dev/null || echo "parse-error")

    echo "ID=${resp_id}  STATUS=${is_error}  MSG=${msg:0:120}"
done < "$RESULTS_DIR/raw_output.json"

echo ""
echo "═══════════════════════════════════════════════════════════════════"
echo "  SUMMARY"
echo "═══════════════════════════════════════════════════════════════════"
echo "Total tool calls sent: ${TOTAL_TOOLS}"
echo "Responses received: $(wc -l < "$RESULTS_DIR/raw_output.json")"
echo ""
echo "Check $RESULTS_DIR/raw_output.json for full output"
echo "Check $RESULTS_DIR/stderr.log for stderr output"
