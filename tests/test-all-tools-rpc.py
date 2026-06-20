#!/usr/bin/env python3
"""MCP RPC harness — every tool called in ISOLATION with its own timeout.

Earlier versions of this test piped all ~90 calls through ONE masday-mcp
process. The stdio server processes JSON-RPC requests sequentially, so a
single blocking tool (notably `tests_run`, which shells out to `cargo test`)
starved the whole queue: only the calls ahead of it ever got a response, and
a `timeout` then killed the server. One slow tool masked every other tool.

This harness spawns a FRESH masday-mcp process per tool and caps each call
with its own timeout, so no tool can block or hide another. Slow CLI/network
tools (`tests_run`, docker, npm, cicd, github_pr_create, git_commit) get a
longer budget; everything else a short one.

Verdict per tool:
  OK         -> got a normal (non-error) result
  TOOL_ERR   -> result.isError true (graceful tool-level error, e.g. a fake id)
  RPC_ERROR  -> JSON-RPC error envelope
  TIMEOUT    -> no response within the per-call budget (slow/hung tool)
  CRASH      -> process exited with no response

OK and TOOL_ERR are both healthy (most tools are exercised with deliberately
fake ids / missing preconditions, so a clean TOOL_ERR is the expected pass).
RPC_ERROR / TIMEOUT / CRASH are real problems and cause a non-zero exit.

Environment:
  MCP_BIN         path to the masday-mcp binary (else auto-resolved from the repo)
  FAST_TIMEOUT    per-call budget for ordinary tools, seconds (default 12)
  SLOW_TIMEOUT    per-call budget for slow CLI/network tools (default 45)
  TEST_PROJECT    project path passed to capability/filesystem tools (default: repo root)

Run:  bash tests/test-all-tools-rpc.sh     (or: python3 tests/test-all-tools-rpc.py)
"""
import json
import os
import subprocess
import sys

FAST = float(os.environ.get("FAST_TIMEOUT", "12"))
SLOW = float(os.environ.get("SLOW_TIMEOUT", "45"))

# Repo root = parent of the tests/ directory holding this script.
HERE = os.path.dirname(os.path.abspath(__file__))
REPO = os.path.dirname(HERE)
PROJ = os.environ.get("TEST_PROJECT", REPO)
TMPP = "/tmp/mcp-test-local"


def resolve_mcp_bin():
    """Find the masday-mcp binary: $MCP_BIN, then repo target/{release,debug}."""
    env = os.environ.get("MCP_BIN")
    if env:
        return env
    for profile in ("release", "debug"):
        cand = os.path.join(REPO, "target", profile, "masday-mcp")
        if os.path.exists(cand):
            return cand
    # Fall through to the release path so the eventual Popen error names it.
    return os.path.join(REPO, "target", "release", "masday-mcp")


MCP_BIN = resolve_mcp_bin()

FAKE_WF = "00000000-0000-0000-0000-000000000000"
FAKE_T = "00000000-0000-0000-0000-000000000001"

# Tools that shell out to slow CLI / network calls get the larger budget.
SLOW_TOOLS = {
    "tests_run",
    "docker_build",
    "docker_run",
    "cicd_pipeline_trigger",
    "github_pr_create",
    "npm_install",
    "npm_run",
    "git_commit",
}

# (tool_name, args_as_json_string). Most use deliberately fake ids so the
# expected verdict is a graceful TOOL_ERR; a few are read-only and return OK.
TOOLS = [
    ("workflow_ping", "{}"),
    ("workflow_list", "{}"),
    ("workflow_getActive", "{}"),
    ("memory_stats", "{}"),
    ("memory_recall_recent", "{}"),
    ("memory_search", '{"query":"test"}'),
    ("memory_search_nodes", '{"query":"test"}'),
    ("capability_list_templates", "{}"),
    ("capability_list_agents", json.dumps({"project_root": PROJ})),
    ("capability_list_skills", json.dumps({"project_root": PROJ})),
    ("capability_system_readiness", json.dumps({"project_root": PROJ})),
    ("capability_match_agent", json.dumps({"project_root": PROJ, "task_description": "test"})),
    ("filesystem_read", json.dumps({"path": os.path.join(REPO, "Cargo.toml")})),
    ("filesystem_list", json.dumps({"path": REPO})),
    ("filesystem_stat", json.dumps({"path": os.path.join(REPO, "Cargo.toml")})),
    ("git_status", "{}"),
    ("git_diff", "{}"),
    ("docker_ps", "{}"),
    ("cicd_pipeline_status", "{}"),
    ("cicd_runs_view", "{}"),
    ("github_pr_list", "{}"),
    ("github_issue_list", "{}"),
    ("tests_run", '{"pattern":"__nonexistent_marker_xyz__"}'),  # slow: compiles, then no match
    ("projectRules_check", json.dumps({"project_root": PROJ})),
    ("reminder_check", "{}"),
    ("reminder_list", "{}"),
    ("session_get_state", '{"session_key":"smoke-none"}'),
    ("policy_check_session_readiness", '{"session_key":"smoke-none"}'),
    ("use_masday", '{"prompt":"create a workflow"}'),
    ("semantic-search_code_search", '{"query":"workflow state machine"}'),
    ("workflow_create", '{"name":"__TEST_RPC__","description":"automated"}'),
    ("memory_store", '{"memory_type":"fact","summary":"rpc test","content":"x","created_by_agent":"test"}'),
    ("memory_store_research", '{"summary":"rpc research","content":"x","created_by_agent":"test"}'),
    ("session_init_context", json.dumps({"cwd": PROJ})),
    ("local_init", json.dumps({"cwd": TMPP})),
    ("local_save_artifact", json.dumps({"cwd": TMPP, "category": "test", "filename": "rpc.txt", "content": "hi"})),
    ("workflow_get", json.dumps({"workflow_id": FAKE_WF})),
    ("workflow_getStatus", json.dumps({"workflow_id": FAKE_WF})),
    ("workflow_getCurrentTask", json.dumps({"workflow_id": FAKE_WF})),
    ("workflow_getPlan", json.dumps({"workflow_id": FAKE_WF})),
    ("workflow_listTasks", json.dumps({"workflow_id": FAKE_WF})),
    ("workflow_listParallelBranches", json.dumps({"workflow_id": FAKE_WF})),
    ("workflow_resume_suggestion", json.dumps({"workflow_id": FAKE_WF})),
    ("workflow_execute", json.dumps({"workflow_id": FAKE_WF})),
    ("workflow_startTask", json.dumps({"workflow_id": FAKE_WF, "task_id": FAKE_T})),
    ("workflow_completeTask", json.dumps({"workflow_id": FAKE_WF, "task_id": FAKE_T})),
    ("workflow_saveProgress", json.dumps({"workflow_id": FAKE_WF, "task_id": FAKE_T, "agent_name": "t", "progress_note": "x"})),
    ("workflow_createPlan", json.dumps({"workflow_id": FAKE_WF, "plan": "x"})),
    ("workflow_addTask", json.dumps({"workflow_id": FAKE_WF, "name": "t", "agent": "t", "skill": "t"})),
    ("workflow_createParallelBranches", json.dumps({"workflow_id": FAKE_WF, "branches": [{"task_id": FAKE_T, "branch_key": "t", "role": "t"}]})),
    ("workflow_completeParallelBranch", json.dumps({"workflow_id": FAKE_WF, "branch_key": "t"})),
    ("workflow_delete", json.dumps({"workflow_id": FAKE_WF})),
    ("review_get_latest", json.dumps({"workflow_id": FAKE_WF, "task_id": FAKE_T})),
    ("memory_recall_documents", json.dumps({"workflow_id": FAKE_WF})),
    ("memory_recall_document_by_type", '{"source_type":"research"}'),
    ("memory_recall_by_task", json.dumps({"task_id": FAKE_T})),
    ("memory_delete_by_workflow", json.dumps({"workflow_id": FAKE_WF})),
    ("memory_update", '{"id":"nope","content":"x"}'),
    ("memory_delete", '{"id":"nope"}'),
    ("review_submit", json.dumps({"workflow_id": FAKE_WF, "task_id": FAKE_T, "reviewer_agent": "t", "decision": "APPROVED", "notes": "x"})),
    ("session_patch_state", '{"session_key":"smoke","patch":"{}"}'),
    ("semantic-search_search_hybrid_context_pack", json.dumps({"workflow_id": FAKE_WF, "plan_id": FAKE_WF, "task_id": FAKE_T})),
    ("semantic-search_search_context_fingerprint", json.dumps({"workflow_id": FAKE_WF, "plan_id": FAKE_WF, "task_id": FAKE_T})),
    ("semantic-search_make_fingerprint", json.dumps({"workflow_id": FAKE_WF, "plan_id": FAKE_WF, "task_id": FAKE_T})),
    ("policy_validate_completion", json.dumps({"session_key": "s", "workflow_id": FAKE_WF, "task_id": FAKE_T})),
    ("policy_validate_execution", json.dumps({"session_key": "s", "workflow_id": FAKE_WF, "task_id": FAKE_T})),
    ("policy_validate_parallel_completion", json.dumps({"session_key": "s", "workflow_id": FAKE_WF, "task_id": FAKE_T})),
    ("policy_detect_scope_drift", json.dumps({"workflow_id": FAKE_WF})),
    ("policy_require_context_refresh", json.dumps({"workflow_id": FAKE_WF})),
    ("capability_workflow_audit", json.dumps({"workflow_id": FAKE_WF})),
    ("memory_create_entities", json.dumps({"entities": json.dumps([{"name": "t", "entity_type": "concept", "properties": "{}"}])})),
    ("capability_scaffold_feature", json.dumps({"project_root": TMPP, "name": "test-feature", "description": "x"})),
    ("capability_scaffold_mcp_server", json.dumps({"project_root": TMPP, "name": "test-mcp", "description": "x"})),
    ("capability_create_agent", json.dumps({"project_root": TMPP, "name": "test-agent", "role": "tester", "description": "x", "instructions": "x"})),
    ("capability_create_skill", json.dumps({"project_root": TMPP, "name": "test-skill", "description": "x", "trigger": "t", "steps": "x"})),
    ("reminder_acknowledge", '{"id":"nope"}'),
    ("workflow_mark_synthesis_ready", '{"session_key":"s","ready":"true"}'),
    ("workflow_mark_verification_ready", '{"session_key":"s","ready":"true"}'),
    ("workflow_set_execution_mode", '{"session_key":"s","mode":"sequential"}'),
    ("filesystem_write", '{"path":"/tmp/mcp-test-rpc-write.txt","content":"x"}'),
    ("filesystem_delete", '{"path":"/tmp/mcp-test-rpc-write.txt"}'),
    ("local_sync", json.dumps({"cwd": TMPP})),
    ("local_push", json.dumps({"cwd": TMPP})),
    ("npm_install", "{}"),
    ("npm_run", '{"script":"__nope__xyz"}'),
    ("git_commit", '{"message":"rpc test"}'),
    ("docker_build", '{"tag":"test-no-build"}'),
    ("docker_run", '{"image":"nonexistent-image-xyz:latest"}'),
    ("cicd_pipeline_trigger", '{"pipeline":"__nope__"}'),
    ("github_pr_create", '{"title":"rpc test"}'),
]


def call_one(name, args):
    """Invoke one tool against a fresh MCP process; return (verdict, detail)."""
    budget = SLOW if name in SLOW_TOOLS else FAST
    call_id = 2
    req = json.dumps({"jsonrpc": "2.0", "id": call_id, "method": "tools/call",
                      "params": {"name": name, "arguments": json.loads(args)}})
    payload = (
        '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}\n'
        '{"jsonrpc":"2.0","method":"notifications/initialized"}\n'
        + req + "\n"
    )
    try:
        proc = subprocess.Popen([MCP_BIN], stdin=subprocess.PIPE,
                                stdout=subprocess.PIPE, stderr=subprocess.DEVNULL)
    except Exception as e:
        return "CRASH", f"spawn failed: {e}"

    verdict, msg = "TIMEOUT", "no response"
    try:
        out, _ = proc.communicate(input=payload.encode(), timeout=budget)
        for raw in out.decode(errors="replace").splitlines():
            raw = raw.strip()
            if not raw.startswith("{"):  # skip any stderr/log bleed on stdout
                continue
            try:
                d = json.loads(raw)
            except Exception:
                continue
            if d.get("id") != call_id:
                continue
            if d.get("error"):
                verdict, msg = "RPC_ERROR", str(d["error"].get("message", ""))[:70]
            elif d.get("result", {}).get("isError"):
                content = d["result"].get("content", [])
                verdict = "TOOL_ERR"
                msg = (content[0].get("text", "") if content else "")[:70]
            else:
                verdict, msg = "OK", ""
            break
        else:
            if proc.poll() is not None:
                verdict, msg = "CRASH", f"exit={proc.returncode}"
    except subprocess.TimeoutExpired:
        proc.kill()
        try:
            out, _ = proc.communicate(timeout=5)
            for raw in out.decode(errors="replace").splitlines():
                if '"id": 2' in raw or '"id":2' in raw:
                    try:
                        d = json.loads(raw.strip())
                        if d.get("id") == call_id and not d.get("error"):
                            verdict, msg = "OK", "(late)"
                            break
                    except Exception:
                        pass
        except Exception:
            pass
    finally:
        if proc.poll() is None:
            proc.kill()
    return verdict, msg


def main():
    counts = {"OK": 0, "TOOL_ERR": 0, "RPC_ERROR": 0, "TIMEOUT": 0, "CRASH": 0}
    rows = []
    for name, args in TOOLS:
        verdict, msg = call_one(name, args)
        counts[verdict] = counts.get(verdict, 0) + 1
        rows.append((name, verdict, msg))

    print("═══════════════════════════════════════════════════════════════════")
    print("  MCP RPC HARNESS — per-tool isolated calls")
    print(f"  MCP_BIN={MCP_BIN}  fast={FAST}s  slow={SLOW}s  project={PROJ}")
    print("═══════════════════════════════════════════════════════════════════")
    for name, verdict, msg in rows:
        print(f"  {verdict:<9} {name:<46} {msg}")
    print("═══════════════════════════════════════════════════════════════════")
    total = len(rows)
    print(f"  TOTAL={total}  OK={counts['OK']}  TOOL_ERR={counts['TOOL_ERR']}  "
          f"RPC_ERROR={counts['RPC_ERROR']}  TIMEOUT={counts['TIMEOUT']}  CRASH={counts['CRASH']}")
    print("═══════════════════════════════════════════════════════════════════")
    # OK + TOOL_ERR (graceful) are healthy; RPC_ERROR/TIMEOUT/CRASH are problems.
    bad = counts["RPC_ERROR"] + counts["TIMEOUT"] + counts["CRASH"]
    sys.exit(0 if bad == 0 else 1)


if __name__ == "__main__":
    main()
