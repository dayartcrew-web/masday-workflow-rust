//! MCP server entry point — JSON-RPC 2.0 over stdio

use masday_mcp::async_tool_handler;
use masday_mcp::client;
use masday_mcp::registry::{ToolDefinition, ToolRegistry};
use masday_mcp::transport::JsonRpcServer;

/// Register a single tool (definition + handler) into the registry.
macro_rules! reg {
    ($registry:expr, $name:expr, $desc:expr, $schema:expr, $handler:expr) => {
        $registry.register(
            ToolDefinition {
                name: $name.to_string(),
                description: $desc.to_string(),
                input_schema: $schema,
            },
            async_tool_handler!($handler),
        );
    };
}

/// Build JSON input schema. `"name"` = required, `"name?"` = optional.
/// Prefix `#` for number type: `"#name"` = required number, `"#name?"` = optional number.
macro_rules! schema {
    ($($key:expr),* $(,)?) => {{
        #[allow(unused_mut)]
        let mut props = serde_json::Map::new();
        #[allow(unused_mut)]
        let mut req: Vec<String> = Vec::new();
        $(
            let k: &str = $key;
            let is_opt = k.ends_with('?');
            let trimmed = k.trim_end_matches('?');
            let (name, type_val) = if trimmed.starts_with('#') {
                (trimmed[1..].to_string(), serde_json::json!({"type":"number"}))
            } else {
                (trimmed.to_string(), serde_json::json!({"type":"string"}))
            };
            props.insert(name.clone(), type_val);
            if !is_opt { req.push(name); }
        )*
        serde_json::json!({"type":"object","properties":props,"required":req})
    }};
    () => { serde_json::json!({"type":"object","properties":{},"required":[]}) };
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    let api_url =
        std::env::var("MASDAY_API_URL").unwrap_or_else(|_| "http://localhost:3010".to_string());
    let api_key = std::env::var("MASDAY_API_KEY").unwrap_or_else(|_| "PLACEHOLDER".to_string());

    client::init(api_url.clone(), api_key)?;
    tracing::info!("MCP server connected to {}", api_url);

    let registry = build_registry();
    tracing::info!("Registered {} tools", registry.count());

    let mut server = JsonRpcServer::new(registry);
    server.run().await?;

    Ok(())
}

fn build_registry() -> ToolRegistry {
    let mut r = ToolRegistry::new();
    register_workflow_tools(&mut r);
    register_memory_tools(&mut r);
    register_review_tools(&mut r);
    register_session_tools(&mut r);
    register_context_tools(&mut r);
    register_policy_tools(&mut r);
    register_reminder_tools(&mut r);
    register_graph_tools(&mut r);
    register_capability_tools(&mut r);
    register_filesystem_tools(&mut r);
    register_git_tools(&mut r);
    register_npm_tools(&mut r);
    register_docker_tools(&mut r);
    register_cicd_tools(&mut r);
    register_github_tools(&mut r);
    register_tests_tools(&mut r);
    register_local_tools(&mut r);
    register_project_rules_tools(&mut r);
    r
}

// ── Workflow (23 tools) ──────────────────────────────────────────────────────

fn register_workflow_tools(r: &mut ToolRegistry) {
    use masday_mcp::tools::workflow as w;
    reg!(
        r,
        "workflow_create",
        "Create a new workflow",
        schema!("name", "description?", "project_path?"),
        w::workflow_create
    );
    reg!(
        r,
        "workflow_execute",
        "Execute a workflow",
        schema!("workflow_id"),
        w::workflow_execute
    );
    reg!(
        r,
        "workflow_getStatus",
        "Get workflow status",
        schema!("workflow_id"),
        w::workflow_get_status
    );
    reg!(
        r,
        "workflow_get",
        "Get workflow by ID",
        schema!("workflow_id"),
        w::workflow_get
    );
    reg!(
        r,
        "workflow_list",
        "List all workflows",
        schema!("page?", "page_size?"),
        w::workflow_list
    );
    reg!(
        r,
        "workflow_getActive",
        "Get active workflows",
        schema!(),
        w::workflow_get_active
    );
    reg!(
        r,
        "workflow_delete",
        "Delete a workflow",
        schema!("workflow_id"),
        w::workflow_delete
    );
    reg!(
        r,
        "workflow_addTask",
        "Add task to workflow",
        schema!("workflow_id", "name", "agent", "skill", "dependencies?"),
        w::workflow_add_task
    );
    reg!(
        r,
        "workflow_startTask",
        "Start a task",
        schema!("workflow_id", "task_id"),
        w::workflow_start_task
    );
    reg!(
        r,
        "workflow_completeTask",
        "Complete a task",
        schema!("workflow_id", "task_id", "result?"),
        w::workflow_complete_task
    );
    reg!(
        r,
        "workflow_saveProgress",
        "Save task progress",
        schema!("workflow_id", "task_id", "agent_name", "progress_note"),
        w::workflow_save_progress
    );
    reg!(
        r,
        "workflow_createPlan",
        "Create a plan",
        schema!("workflow_id", "plan"),
        w::workflow_create_plan
    );
    reg!(
        r,
        "workflow_getPlan",
        "Get plan for workflow",
        schema!("workflow_id"),
        w::workflow_get_plan
    );
    reg!(
        r,
        "workflow_listTasks",
        "List workflow tasks",
        schema!("workflow_id"),
        w::workflow_list_tasks
    );
    reg!(
        r,
        "workflow_createParallelBranches",
        "Create parallel branches",
        schema!("workflow_id", "branches"),
        w::workflow_create_parallel_branches
    );
    reg!(
        r,
        "workflow_completeParallelBranch",
        "Complete a parallel branch",
        schema!("workflow_id", "branch_key"),
        w::workflow_complete_parallel_branch
    );
    reg!(
        r,
        "workflow_listParallelBranches",
        "List parallel branches",
        schema!("workflow_id"),
        w::workflow_list_parallel_branches
    );
    reg!(
        r,
        "workflow_mark_synthesis_ready",
        "Mark synthesis ready",
        schema!("session_key", "ready"),
        w::workflow_mark_synthesis_ready
    );
    reg!(
        r,
        "workflow_mark_verification_ready",
        "Mark verification ready",
        schema!("session_key", "ready"),
        w::workflow_mark_verification_ready
    );
    reg!(
        r,
        "workflow_set_execution_mode",
        "Set execution mode",
        schema!("session_key", "mode"),
        w::workflow_set_execution_mode
    );
    reg!(
        r,
        "workflow_resume_suggestion",
        "Get resume suggestion",
        schema!("workflow_id"),
        w::workflow_resume_suggestion
    );
    reg!(
        r,
        "workflow_ping",
        "Ping workflow server",
        schema!(),
        w::workflow_ping
    );
    reg!(
        r,
        "workflow_getCurrentTask",
        "Get current task for workflow",
        schema!("workflow_id"),
        w::workflow_get_current_task
    );
}

// ── Memory (11 tools) ────────────────────────────────────────────────────────

fn register_memory_tools(r: &mut ToolRegistry) {
    use masday_mcp::tools::memory as m;
    reg!(
        r,
        "memory_store",
        "Store a memory",
        schema!(
            "memory_type",
            "summary",
            "content",
            "created_by_agent",
            "#importance_score?",
            "tags?",
            "workflow_id?",
            "task_id?"
        ),
        m::memory_store
    );
    reg!(
        r,
        "memory_store_research",
        "Store research findings",
        schema!("summary", "content", "created_by_agent", "workflow_id?"),
        m::memory_store_research
    );
    reg!(
        r,
        "memory_search",
        "Search memories",
        schema!("query", "limit?"),
        m::memory_search
    );
    reg!(
        r,
        "memory_recall_documents",
        "Recall documents for workflow",
        schema!("workflow_id", "limit?"),
        m::memory_recall_documents
    );
    reg!(
        r,
        "memory_recall_document_by_type",
        "Recall documents by type",
        schema!("source_type", "limit?"),
        m::memory_recall_document_by_type
    );
    reg!(
        r,
        "memory_recall_by_task",
        "Recall memories by task",
        schema!("task_id", "limit?"),
        m::memory_recall_by_task
    );
    reg!(
        r,
        "memory_recall_recent",
        "Recall recent memories",
        schema!("limit?", "type?"),
        m::memory_recall_recent
    );
    reg!(
        r,
        "memory_update",
        "Update a memory",
        schema!("id", "content?", "importance?"),
        m::memory_update
    );
    reg!(
        r,
        "memory_delete",
        "Delete a memory",
        schema!("id"),
        m::memory_delete
    );
    reg!(
        r,
        "memory_delete_by_workflow",
        "Delete memories by workflow",
        schema!("workflow_id"),
        m::memory_delete_by_workflow
    );
    reg!(
        r,
        "memory_stats",
        "Get memory statistics",
        schema!(),
        m::memory_stats
    );
}

// ── Review (2 tools) ─────────────────────────────────────────────────────────

fn register_review_tools(r: &mut ToolRegistry) {
    use masday_mcp::tools::review as rv;
    reg!(
        r,
        "review_submit",
        "Submit a review",
        schema!(
            "workflow_id",
            "task_id",
            "reviewer_agent",
            "decision",
            "notes",
            "gaps?"
        ),
        rv::review_submit
    );
    reg!(
        r,
        "review_get_latest",
        "Get latest review",
        schema!("workflow_id", "task_id"),
        rv::review_get_latest
    );
}

// ── Session (3 tools) ────────────────────────────────────────────────────────

fn register_session_tools(r: &mut ToolRegistry) {
    use masday_mcp::tools::session as s;
    reg!(
        r,
        "session_init_context",
        "Initialize session context",
        schema!("cwd"),
        s::session_init_context
    );
    reg!(
        r,
        "session_get_state",
        "Get session state",
        schema!("session_key"),
        s::session_get_state
    );
    reg!(
        r,
        "session_patch_state",
        "Patch session state",
        schema!("session_key", "patch"),
        s::session_patch_state
    );
}

// ── Context / Semantic Search (4 tools) ──────────────────────────────────────

fn register_context_tools(r: &mut ToolRegistry) {
    use masday_mcp::tools::context as c;
    reg!(
        r,
        "semantic-search_search_hybrid_context_pack",
        "Build hybrid context pack",
        schema!("workflow_id", "plan_id", "task_id"),
        c::semantic_search_search_hybrid_context_pack
    );
    reg!(
        r,
        "semantic-search_search_context_fingerprint",
        "Compute context fingerprint",
        schema!("workflow_id", "plan_id", "task_id"),
        c::semantic_search_search_context_fingerprint
    );
    reg!(
        r,
        "semantic-search_code_search",
        "Search codebase",
        schema!("query"),
        c::semantic_search_code_search
    );
    reg!(
        r,
        "semantic-search_make_fingerprint",
        "Generate deterministic fingerprint",
        schema!(
            "workflow_id",
            "plan_id",
            "task_id",
            "acceptance_criteria?",
            "required_context?",
            "document_ids?",
            "memory_ids?"
        ),
        c::semantic_search_make_fingerprint
    );
}

// ── Policy (6 tools) ─────────────────────────────────────────────────────────

fn register_policy_tools(r: &mut ToolRegistry) {
    use masday_mcp::tools::policy as p;
    reg!(
        r,
        "policy_check_session_readiness",
        "Check session readiness",
        schema!("session_key"),
        p::policy_check_session_readiness
    );
    reg!(
        r,
        "policy_validate_completion",
        "Validate task completion",
        schema!("session_key", "workflow_id", "task_id"),
        p::policy_validate_completion
    );
    reg!(
        r,
        "policy_validate_execution",
        "Validate execution",
        schema!("session_key", "workflow_id", "task_id"),
        p::policy_validate_execution
    );
    reg!(
        r,
        "policy_validate_parallel_completion",
        "Validate parallel completion",
        schema!("session_key", "workflow_id", "task_id"),
        p::policy_validate_parallel_completion
    );
    reg!(
        r,
        "policy_detect_scope_drift",
        "Detect scope drift",
        schema!("workflow_id", "task_id?", "output_text?"),
        p::policy_detect_scope_drift
    );
    reg!(
        r,
        "policy_require_context_refresh",
        "Require context refresh",
        schema!("workflow_id", "plan_id?", "task_id?", "last_fingerprint?"),
        p::policy_require_context_refresh
    );
}

// ── Reminder (3 tools) ───────────────────────────────────────────────────────

fn register_reminder_tools(r: &mut ToolRegistry) {
    use masday_mcp::tools::reminder as rem;
    reg!(
        r,
        "reminder_check",
        "Check for stale/stuck/failed workflows and tasks",
        schema!(
            "staleExecutionMinutes?",
            "stuckTaskMinutes?",
            "includeFailed?"
        ),
        rem::reminder_check
    );
    reg!(
        r,
        "reminder_list",
        "List stored reminders",
        schema!("workflow_id?", "acknowledged?", "limit?"),
        rem::reminder_list
    );
    reg!(
        r,
        "reminder_acknowledge",
        "Acknowledge a reminder",
        schema!("id?", "workflowId?"),
        rem::reminder_acknowledge
    );
}

// ── Graph (2 tools) ──────────────────────────────────────────────────────────

fn register_graph_tools(r: &mut ToolRegistry) {
    use masday_mcp::tools::graph as g;
    reg!(
        r,
        "memory_create_entities",
        "Create graph entities",
        schema!("entities"),
        g::memory_create_entities
    );
    reg!(
        r,
        "memory_search_nodes",
        "Search graph nodes",
        schema!("query"),
        g::memory_search_nodes
    );
}

// ── Capability (11 tools) ────────────────────────────────────────────────────

fn register_capability_tools(r: &mut ToolRegistry) {
    use masday_mcp::tools::capability as cap;
    reg!(
        r,
        "capability_create_agent",
        "Create an agent",
        schema!("projectRoot", "name", "role", "description", "instructions"),
        cap::capability_create_agent
    );
    reg!(
        r,
        "capability_create_skill",
        "Create a skill",
        schema!("projectRoot", "name", "description", "trigger", "steps"),
        cap::capability_create_skill
    );
    reg!(
        r,
        "capability_list_agents",
        "List registered agents",
        schema!("projectRoot"),
        cap::capability_list_agents
    );
    reg!(
        r,
        "capability_list_skills",
        "List registered skills",
        schema!("projectRoot"),
        cap::capability_list_skills
    );
    reg!(
        r,
        "capability_list_templates",
        "List available templates",
        schema!(),
        cap::capability_list_templates
    );
    reg!(
        r,
        "capability_match_agent",
        "Match agent for a task",
        schema!("projectRoot", "taskDescription"),
        cap::capability_match_agent
    );
    reg!(
        r,
        "capability_scaffold_feature",
        "Scaffold a new feature",
        schema!("projectRoot", "name", "description"),
        cap::capability_scaffold_feature
    );
    reg!(
        r,
        "capability_scaffold_mcp_server",
        "Scaffold an MCP server",
        schema!("projectRoot", "name", "description"),
        cap::capability_scaffold_mcp_server
    );
    reg!(
        r,
        "capability_system_readiness",
        "Check system readiness",
        schema!("projectRoot"),
        cap::capability_system_readiness
    );
    reg!(
        r,
        "capability_workflow_audit",
        "Audit a workflow",
        schema!("workflowId"),
        cap::capability_workflow_audit
    );
    reg!(
        r,
        "capability_ping",
        "Ping capability service",
        schema!(),
        cap::capability_ping
    );
}

// ── Filesystem (5 tools) — stubs for Phase 3.2 ───────────────────────────────

fn register_filesystem_tools(r: &mut ToolRegistry) {
    use masday_mcp::tools::filesystem as fs;
    reg!(
        r,
        "filesystem_read",
        "Read a file",
        schema!("path"),
        fs::filesystem_read
    );
    reg!(
        r,
        "filesystem_write",
        "Write a file",
        schema!("path", "content"),
        fs::filesystem_write
    );
    reg!(
        r,
        "filesystem_list",
        "List directory",
        schema!("path"),
        fs::filesystem_list
    );
    reg!(
        r,
        "filesystem_delete",
        "Delete a file",
        schema!("path"),
        fs::filesystem_delete
    );
    reg!(
        r,
        "filesystem_stat",
        "Get file stat",
        schema!("path"),
        fs::filesystem_stat
    );
}

// ── Git (3 tools) — stubs for Phase 3.2 ──────────────────────────────────────

fn register_git_tools(r: &mut ToolRegistry) {
    use masday_mcp::tools::git as g;
    reg!(r, "git_status", "Get git status", schema!(), g::git_status);
    reg!(r, "git_diff", "Get git diff", schema!(), g::git_diff);
    reg!(
        r,
        "git_commit",
        "Git commit",
        schema!("message"),
        g::git_commit
    );
}

// ── NPM (2 tools) — stubs for Phase 3.2 ──────────────────────────────────────

fn register_npm_tools(r: &mut ToolRegistry) {
    use masday_mcp::tools::npm as n;
    reg!(
        r,
        "npm_install",
        "Install npm packages",
        schema!("packages?"),
        n::npm_install
    );
    reg!(
        r,
        "npm_run",
        "Run npm script",
        schema!("script"),
        n::npm_run
    );
}

// ── Docker (3 tools) — stubs for Phase 3.2 ───────────────────────────────────

fn register_docker_tools(r: &mut ToolRegistry) {
    use masday_mcp::tools::docker as d;
    reg!(
        r,
        "docker_build",
        "Build Docker image",
        schema!("tag?"),
        d::docker_build
    );
    reg!(
        r,
        "docker_run",
        "Run Docker container",
        schema!("image"),
        d::docker_run
    );
    reg!(
        r,
        "docker_ps",
        "List running containers",
        schema!(),
        d::docker_ps
    );
}

// ── CI/CD (3 tools) — stubs for Phase 3.2 ────────────────────────────────────

fn register_cicd_tools(r: &mut ToolRegistry) {
    use masday_mcp::tools::cicd as ci;
    reg!(
        r,
        "cicd_pipeline_status",
        "Get CI/CD pipeline status",
        schema!(),
        ci::cicd_pipeline_status
    );
    reg!(
        r,
        "cicd_pipeline_trigger",
        "Trigger CI/CD pipeline",
        schema!("pipeline"),
        ci::cicd_pipeline_trigger
    );
    reg!(
        r,
        "cicd_runs_view",
        "View CI/CD runs",
        schema!(),
        ci::cicd_runs_view
    );
}

// ── GitHub (3 tools) — stubs for Phase 3.2 ───────────────────────────────────

fn register_github_tools(r: &mut ToolRegistry) {
    use masday_mcp::tools::github as gh;
    reg!(
        r,
        "github_pr_create",
        "Create a pull request",
        schema!("title", "body?"),
        gh::github_pr_create
    );
    reg!(
        r,
        "github_pr_list",
        "List pull requests",
        schema!(),
        gh::github_pr_list
    );
    reg!(
        r,
        "github_issue_list",
        "List issues",
        schema!(),
        gh::github_issue_list
    );
}

// ── Tests (1 tool) — stub for Phase 3.2 ──────────────────────────────────────

fn register_tests_tools(r: &mut ToolRegistry) {
    use masday_mcp::tools::tests as t;
    reg!(
        r,
        "tests_run",
        "Run tests",
        schema!("pattern?"),
        t::tests_run
    );
}

// ── Local (4 tools) — stubs for Phase 3.2 ────────────────────────────────────

fn register_local_tools(r: &mut ToolRegistry) {
    use masday_mcp::tools::local as l;
    reg!(
        r,
        "local_init",
        "Initialize local state directory",
        schema!("cwd"),
        l::local_init
    );
    reg!(
        r,
        "local_sync",
        "Sync local state from DB",
        schema!("cwd", "workflow_id?"),
        l::local_sync
    );
    reg!(
        r,
        "local_push",
        "Push local state to DB",
        schema!("cwd", "workflow_id?"),
        l::local_push
    );
    reg!(
        r,
        "local_save_artifact",
        "Save artifact file locally",
        schema!("cwd", "category", "filename", "content"),
        l::local_save_artifact
    );
}

// ── Project Rules (1 tool) — stub for Phase 3.2 ──────────────────────────────

fn register_project_rules_tools(r: &mut ToolRegistry) {
    use masday_mcp::tools::project_rules as pr;
    reg!(
        r,
        "projectRules_check",
        "Validate project against refactor rules",
        schema!("projectRoot?"),
        pr::projectrules_check
    );
}
