//! masday-mcp - MCP stdio server
//!
//! Two binaries:
//! - `masday-mcp-http` — thin HTTP proxy to masday-api
//! - `masday-mcp-stdio` — standalone, direct PostgreSQL access

pub mod client;
#[cfg(feature = "sqlite")]
pub mod code_index;
#[cfg(feature = "sqlite")]
pub mod direct;
#[cfg(feature = "sqlite")]
pub mod direct_pg;
pub mod embedding;
pub mod handler;
pub mod mode;
pub mod pg;
#[cfg(feature = "sqlite")]
pub mod pg_code_index;
#[cfg(feature = "sqlite")]
pub mod pg_sync;
pub mod registry;
#[cfg(feature = "sqlite")]
pub mod sqlite;
#[cfg(feature = "sqlite")]
pub mod sqlite_schema;
pub mod tools;
pub mod transport;

// Re-export key types for public API
pub use handler::{JsonRequest, McpHandler};
pub use registry::{ToolDefinition, ToolHandler, ToolRegistry};
pub use transport::JsonRpcServer;

/// Register a single tool (definition + handler) into the registry.
#[macro_export]
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
/// Prefix `[]` for array type: `"[]name"` = required array, `"[]name?"` = optional array.
#[macro_export]
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
            let (name, type_val) = if let Some(stripped) = trimmed.strip_prefix('#') {
                (stripped.to_string(), serde_json::json!({"type":"number"}))
            } else if let Some(stripped) = trimmed.strip_prefix("[]") {
                (stripped.to_string(), serde_json::json!({"type":"array","items":{"type":"object"}}))
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

/// Build the complete tool registry with all MCP tools registered (HTTP proxy mode).
/// Tools use client::api_* functions that call the API server over HTTP.
pub fn build_registry() -> ToolRegistry {
    let mut r = ToolRegistry::new();
    register_use_masday_tools(&mut r);
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

// ── Universal Entry Point (1 tool) ─────────────────────────────────────────────

fn register_use_masday_tools(r: &mut ToolRegistry) {
    use crate::tools::use_masday as u;
    reg!(
        r,
        "use_masday",
        "Universal entry point - parse any instruction and return routing plan",
        schema!("prompt"),
        u::use_masday
    );
}

// ── Workflow (23 tools) ──────────────────────────────────────────────────────

fn register_workflow_tools(r: &mut ToolRegistry) {
    use crate::tools::workflow as w;
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
        "Create parallel branches. branches is an array of objects: [{\"task_id\": \"...\", \"branch_key\": \"...\", \"role\": \"...\"}]",
        schema!("workflow_id", "[]branches"),
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
    #[cfg(feature = "sqlite")]
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
    use crate::tools::memory as m;
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
    use crate::tools::review as rv;
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
    use crate::tools::session as s;
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
    use crate::tools::context as c;
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
    use crate::tools::policy as p;
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
    use crate::tools::reminder as rem;
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
    use crate::tools::graph as g;
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
    use crate::tools::capability as cap;
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
}

// ── Filesystem (5 tools) ─────────────────────────────────────────────────────

fn register_filesystem_tools(r: &mut ToolRegistry) {
    use crate::tools::filesystem as fs;
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

// ── Git (3 tools) ─────────────────────────────────────────────────────────────

fn register_git_tools(r: &mut ToolRegistry) {
    use crate::tools::git as g;
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

// ── NPM (2 tools) ─────────────────────────────────────────────────────────────

fn register_npm_tools(r: &mut ToolRegistry) {
    use crate::tools::npm as n;
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

// ── Docker (3 tools) ───────────────────────────────────────────────────────────

fn register_docker_tools(r: &mut ToolRegistry) {
    use crate::tools::docker as d;
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

// ── CI/CD (3 tools) ─────────────────────────────────────────────────────────

fn register_cicd_tools(r: &mut ToolRegistry) {
    use crate::tools::cicd as ci;
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

// ── GitHub (3 tools) ─────────────────────────────────────────────────────────

fn register_github_tools(r: &mut ToolRegistry) {
    use crate::tools::github as gh;
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

// ── Tests (1 tool) ─────────────────────────────────────────────────────────

fn register_tests_tools(r: &mut ToolRegistry) {
    use crate::tools::tests as t;
    reg!(
        r,
        "tests_run",
        "Run tests",
        schema!("pattern?"),
        t::tests_run
    );
}

// ── Local (4 tools) ─────────────────────────────────────────────────────────

fn register_local_tools(r: &mut ToolRegistry) {
    use crate::tools::local as l;
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

// ── Project Rules (1 tool) ─────────────────────────────────────────────────

fn register_project_rules_tools(r: &mut ToolRegistry) {
    use crate::tools::project_rules as pr;
    reg!(
        r,
        "projectRules_check",
        "Sanity-check that .claude/rules markdown files exist and are well-formed (basic file-format check: directory present, >=1 non-empty .md with markdown headers). NOT a lint or rule-content validator.",
        schema!("projectRoot?"),
        pr::projectrules_check
    );
}

/// Run the MCP stdio server in HTTP proxy mode.
/// Requires masday-api running on the given URL.
pub async fn run_http(api_url: String, api_key: String) -> Result<(), Box<dyn std::error::Error>> {
    // Stdio MCP: stdout is JSON-RPC — suppress all tracing to prevent
    // protocol corruption on Windows/Claude Desktop (merges stderr into stream).
    tracing::subscriber::set_global_default(tracing::subscriber::NoSubscriber::default()).ok(); // ignore error if already set

    client::init(api_url.clone(), api_key).map_err(|e| e.to_string())?;
    tracing::info!("MCP server (HTTP proxy) connected to {}", api_url);

    let registry = build_registry();
    tracing::info!("Registered {} tools", registry.count());

    let mut server = JsonRpcServer::new(registry);
    server.run().await
}

/// Run the MCP stdio server in standalone mode.
/// Connects directly to PostgreSQL via DATABASE_URL. No masday-api needed.
#[cfg(feature = "sqlite")]
pub async fn run_stdio() -> Result<(), Box<dyn std::error::Error>> {
    // Suppress all tracing — stdout is JSON-RPC, any log output corrupts the stream
    let _ = tracing::subscriber::set_global_default(tracing::subscriber::NoSubscriber::default());

    sqlite::init_sqlite().map_err(|e| format!("SQLite init failed: {}", e))?;

    let registry = build_stdio_registry();

    let mut server = JsonRpcServer::new(registry);
    let run_result = server.run().await;
    // C2.12: best-effort flush of in-flight PG-sync spawns before the tokio
    // runtime drops and aborts them (the last sync of a session otherwise never
    // reaches PG). Bounded to 5s so shutdown never hangs; near-instant when
    // nothing is pending.
    crate::pg_sync::drain(std::time::Duration::from_secs(5)).await;
    run_result
}

/// Run the MCP stdio server in local mode.
/// Uses PostgreSQL (primary) + SQLite (cache) + Ollama (embed).
/// Falls back to SQLite-only if PostgreSQL is unavailable.
#[cfg(feature = "sqlite")]
pub async fn run_local() -> Result<(), Box<dyn std::error::Error>> {
    // Suppress all tracing — stdout is JSON-RPC, any log output corrupts the stream
    let _ = tracing::subscriber::set_global_default(tracing::subscriber::NoSubscriber::default());

    // Init SQLite (always needed for cache/fingerprints)
    sqlite::init_sqlite().map_err(|e| format!("SQLite init failed: {}", e))?;
    tracing::info!("SQLite initialized");

    // Init the API client: local mode runs alongside the masday-api server on
    // localhost, so API-mediated tools (remote-mode parity) need it wired. Reads
    // api_url/api_key from ~/.masday/config.toml (production: no env vars). Non-fatal
    // — PG-direct tools (e.g. pgvector code search) work without it; only the API
    // path is skipped if the client can't be initialized.
    if let Some(api_url) = pg::read_api_url() {
        if !api_url.is_empty() {
            let api_key = pg::read_config_value("api_key").unwrap_or_default();
            match client::init(api_url, api_key) {
                Ok(()) => tracing::info!("API client initialized (local mode)"),
                Err(e) => tracing::warn!("API client init failed (non-fatal): {}", e),
            }
        }
    }

    // PostgreSQL: eagerly init pool at startup (not lazy) so it's ready for sync.
    let pg_ready = pg::is_configured();
    if pg_ready {
        tracing::info!("PostgreSQL configured — initializing pool...");
        // Wait up to 5s for pool to be ready before accepting tool calls
        match pg::get_pool_wait(std::time::Duration::from_secs(5)).await {
            Some(_) => tracing::info!("PostgreSQL pool ready"),
            None => tracing::warn!("PostgreSQL pool not ready after 5s — will retry on first use"),
        }
    } else {
        tracing::info!("No PostgreSQL configured — SQLite-only mode");
    }

    let registry = build_stdio_registry();
    tracing::info!(
        "Registered {} tools (local mode, PostgreSQL: {})",
        registry.count(),
        if pg_ready {
            "on-demand"
        } else {
            "not configured"
        }
    );

    let mut server = JsonRpcServer::new(registry);
    let run_result = server.run().await;
    // C2.12: best-effort flush of in-flight PG-sync spawns before the tokio
    // runtime drops and aborts them (the last sync of a session otherwise never
    // reaches PG). Bounded to 5s so shutdown never hangs; near-instant when
    // nothing is pending.
    crate::pg_sync::drain(std::time::Duration::from_secs(5)).await;
    run_result
}

/// Print embedding configuration + health to stderr.
///
/// Reads directly from `~/.masday/config.toml` (production: no env vars).
/// Uses `eprintln!` (not tracing) so it survives the stdio NoSubscriber
/// suppression — the user sees the active provider/model and any errors
/// (Ollama unavailable, wrong provider, local model not compiled in) on
/// every MCP startup.
pub async fn print_embedding_diagnostics() {
    let provider = pg::read_embedding_provider().unwrap_or_default();
    let model = pg::read_embedding_model();
    let dims = pg::read_embedding_dimensions();

    let model_str = model.clone().unwrap_or_else(|| "(default)".to_string());
    let dims_str = dims
        .map(|d| d.to_string())
        .unwrap_or_else(|| "(default)".to_string());

    let detail = format!(
        "provider={} model={} dims={}",
        provider, model_str, dims_str
    );

    match provider.to_lowercase().as_str() {
        "" => eprintln!(
            "[masday] Embedding: DISABLED — embedding_provider tidak diset di ~/.masday/config.toml. \
             Memory/code search memakai fallback feature-hashing. \
             Set embedding_provider=ollama (butuh `ollama serve` + `ollama pull nomic-embed-text`) \
             atau jalankan `masday embed setting`."
        ),
        "local" => eprintln!(
            "[masday] Embedding: ⚠ TIDAK TERSEDIA — {}. Provider 'local' (fastembed/ONNX) tidak \
             dikompilasi ke MCP server. Memory search jatuh ke mock. Solusi: set \
             embedding_provider=\"ollama\" di config.toml, ATAU rebuild masday-mcp dengan \
             feature local-embeddings.", detail
        ),
        "ollama" => {
            let base_url = pg::read_embedding_base_url()
                .unwrap_or_else(|| "http://localhost:11434".to_string());
            let probe = format!("{}/api/tags", base_url.trim_end_matches('/'));
            let ok = match reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(2))
                .build()
            {
                Ok(c) => c.get(&probe).send().await.map(|r| r.status().is_success()).unwrap_or(false),
                Err(_) => false,
            };
            if ok {
                eprintln!("[masday] Embedding: ✓ OK — {} (Ollama {} reachable)", detail, base_url);
            } else {
                eprintln!(
                    "[masday] Embedding: ⚠ OLLAMA TIDAK TERSEDIA — {} ({} tidak merespon). \
                     Jalankan `ollama serve` lalu `ollama pull {}`. Memory search akan fallback ke mock.",
                    detail, base_url, model.unwrap_or_else(|| "nomic-embed-text".to_string())
                );
            }
        }
        "openai" => {
            let has_key = pg::read_config_value("embedding_api_key").is_some()
                || pg::read_config_value("openai_api_key").is_some();
            if has_key {
                eprintln!("[masday] Embedding: ✓ configured — {} (API key present)", detail);
            } else {
                eprintln!(
                    "[masday] Embedding: ⚠ OPENAI KEY HILANG — {} (embedding_api_key/openai_api_key \
                     tidak ada di config.toml). Memory search akan fallback ke mock.",
                    detail
                );
            }
        }
        other => eprintln!(
            "[masday] Embedding: ⚠ SALAH SETTINGS — provider '{}' tidak dikenal. \
             Pilihan valid: local | ollama | openai. Memory search fallback ke mock.",
            other
        ),
    }
}

// ── Standalone Registry (direct DB calls) ──────────────────────────────────

/// Build the stdio registry: DB-dependent tools use `direct::*`, local tools unchanged.
#[cfg(feature = "sqlite")]
fn build_stdio_registry() -> ToolRegistry {
    let mut r = ToolRegistry::new();
    register_use_masday_tools(&mut r);
    register_workflow_tools_stdio(&mut r);
    register_memory_tools_stdio(&mut r);
    register_review_tools_stdio(&mut r);
    register_session_tools_stdio(&mut r);
    register_context_tools_stdio(&mut r);
    register_policy_tools_stdio(&mut r);
    register_reminder_tools_stdio(&mut r);
    register_graph_tools_stdio(&mut r);
    register_capability_tools_stdio(&mut r);
    // Local-only tools (unchanged — no DB needed)
    register_filesystem_tools(&mut r);
    register_git_tools(&mut r);
    register_npm_tools(&mut r);
    register_docker_tools(&mut r);
    register_cicd_tools(&mut r);
    register_github_tools(&mut r);
    register_tests_tools(&mut r);
    register_local_tools_stdio(&mut r);
    register_project_rules_tools(&mut r);
    r
}

// ── Stdio register functions (use direct::* instead of tools::*) ────────────

#[cfg(feature = "sqlite")]
fn register_workflow_tools_stdio(r: &mut ToolRegistry) {
    use crate::direct as d;
    reg!(
        r,
        "workflow_create",
        "Create a new workflow",
        schema!("name", "description?", "project_path?"),
        d::workflow_create
    );
    reg!(
        r,
        "workflow_execute",
        "Execute a workflow",
        schema!("workflow_id"),
        d::workflow_execute
    );
    reg!(
        r,
        "workflow_getStatus",
        "Get workflow status",
        schema!("workflow_id"),
        d::workflow_get_status
    );
    reg!(
        r,
        "workflow_get",
        "Get workflow by ID",
        schema!("workflow_id"),
        d::workflow_get
    );
    reg!(
        r,
        "workflow_list",
        "List workflows, optionally filtered by project_path",
        schema!("page?", "page_size?", "project_path?"),
        d::workflow_list
    );
    reg!(
        r,
        "workflow_getActive",
        "Get active workflows, optionally filtered by project_path",
        schema!("project_path?"),
        d::workflow_get_active
    );
    reg!(
        r,
        "workflow_delete",
        "Delete a workflow",
        schema!("workflow_id"),
        d::workflow_delete
    );
    reg!(
        r,
        "workflow_addTask",
        "Add task to workflow",
        schema!("workflow_id", "name", "agent", "skill", "dependencies?"),
        d::workflow_add_task
    );
    reg!(
        r,
        "workflow_startTask",
        "Start a task",
        schema!("workflow_id", "task_id"),
        d::workflow_start_task
    );
    reg!(
        r,
        "workflow_completeTask",
        "Complete a task",
        schema!("workflow_id", "task_id", "result?"),
        d::workflow_complete_task
    );
    reg!(
        r,
        "workflow_saveProgress",
        "Save task progress",
        schema!("workflow_id", "task_id", "agent_name", "progress_note"),
        d::workflow_save_progress
    );
    reg!(
        r,
        "workflow_createPlan",
        "Create a plan",
        schema!("workflow_id", "plan"),
        d::workflow_create_plan
    );
    reg!(
        r,
        "workflow_getPlan",
        "Get plan for workflow",
        schema!("workflow_id"),
        d::workflow_get_plan
    );
    reg!(
        r,
        "workflow_listTasks",
        "List workflow tasks",
        schema!("workflow_id"),
        d::workflow_list_tasks
    );
    reg!(
        r,
        "workflow_createParallelBranches",
        "Create parallel branches. branches is an array of objects: [{\"task_id\": \"...\", \"branch_key\": \"...\", \"role\": \"...\"}]",
        schema!("workflow_id", "[]branches"),
        d::workflow_create_parallel_branches
    );
    reg!(
        r,
        "workflow_completeParallelBranch",
        "Complete a parallel branch",
        schema!("workflow_id", "branch_key"),
        d::workflow_complete_parallel_branch
    );
    reg!(
        r,
        "workflow_listParallelBranches",
        "List parallel branches",
        schema!("workflow_id"),
        d::workflow_list_parallel_branches
    );
    reg!(
        r,
        "workflow_mark_synthesis_ready",
        "Mark synthesis ready",
        schema!("session_key", "ready"),
        d::workflow_mark_synthesis_ready
    );
    reg!(
        r,
        "workflow_mark_verification_ready",
        "Mark verification ready",
        schema!("session_key", "ready"),
        d::workflow_mark_verification_ready
    );
    reg!(
        r,
        "workflow_set_execution_mode",
        "Set execution mode",
        schema!("session_key", "mode"),
        d::workflow_set_execution_mode
    );
    reg!(
        r,
        "workflow_resume_suggestion",
        "Get resume suggestion",
        schema!("workflow_id"),
        d::workflow_resume_suggestion
    );
    reg!(
        r,
        "workflow_ping",
        "Ping workflow server",
        schema!(),
        d::workflow_ping
    );
    reg!(
        r,
        "workflow_getCurrentTask",
        "Get current task for workflow",
        schema!("workflow_id"),
        d::workflow_get_current_task
    );
}

#[cfg(feature = "sqlite")]
fn register_memory_tools_stdio(r: &mut ToolRegistry) {
    use crate::direct as d;
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
        d::memory_store
    );
    reg!(
        r,
        "memory_store_research",
        "Store research findings",
        schema!("summary", "content", "created_by_agent", "workflow_id?"),
        d::memory_store_research
    );
    reg!(
        r,
        "memory_search",
        "Search memories",
        schema!("query", "limit?"),
        d::memory_search
    );
    reg!(
        r,
        "memory_recall_documents",
        "Recall documents for workflow",
        schema!("workflow_id", "limit?"),
        d::memory_recall_documents
    );
    reg!(
        r,
        "memory_recall_document_by_type",
        "Recall documents by type",
        schema!("source_type", "limit?"),
        d::memory_recall_document_by_type
    );
    reg!(
        r,
        "memory_recall_by_task",
        "Recall memories by task",
        schema!("task_id", "limit?"),
        d::memory_recall_by_task
    );
    reg!(
        r,
        "memory_recall_recent",
        "Recall recent memories",
        schema!("limit?", "type?"),
        d::memory_recall_recent
    );
    reg!(
        r,
        "memory_update",
        "Update a memory",
        schema!("id", "content?", "importance?"),
        d::memory_update
    );
    reg!(
        r,
        "memory_delete",
        "Delete a memory",
        schema!("id"),
        d::memory_delete
    );
    reg!(
        r,
        "memory_delete_by_workflow",
        "Delete memories by workflow",
        schema!("workflow_id"),
        d::memory_delete_by_workflow
    );
    reg!(
        r,
        "memory_stats",
        "Get memory statistics",
        schema!(),
        d::memory_stats
    );
}

#[cfg(feature = "sqlite")]
fn register_review_tools_stdio(r: &mut ToolRegistry) {
    use crate::direct as d;
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
        d::review_submit
    );
    reg!(
        r,
        "review_get_latest",
        "Get latest review",
        schema!("workflow_id", "task_id"),
        d::review_get_latest
    );
}

#[cfg(feature = "sqlite")]
fn register_session_tools_stdio(r: &mut ToolRegistry) {
    use crate::direct as d;
    reg!(
        r,
        "session_init_context",
        "Initialize session context",
        schema!("cwd"),
        d::session_init_context
    );
    reg!(
        r,
        "session_get_state",
        "Get session state",
        schema!("session_key"),
        d::session_get_state
    );
    reg!(
        r,
        "session_patch_state",
        "Patch session state",
        schema!("session_key", "patch"),
        d::session_patch_state
    );
}

#[cfg(feature = "sqlite")]
fn register_context_tools_stdio(r: &mut ToolRegistry) {
    use crate::direct as d;
    reg!(
        r,
        "semantic-search_search_hybrid_context_pack",
        "Build hybrid context pack",
        schema!("workflow_id", "plan_id", "task_id"),
        d::search_hybrid_context_pack
    );
    reg!(
        r,
        "semantic-search_search_context_fingerprint",
        "Compute context fingerprint",
        schema!("workflow_id", "plan_id", "task_id"),
        d::search_context_fingerprint
    );
    reg!(
        r,
        "semantic-search_code_search",
        "Search codebase",
        schema!("query"),
        d::semantic_search_code_search
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
        d::semantic_search_make_fingerprint
    );
}

#[cfg(feature = "sqlite")]
fn register_policy_tools_stdio(r: &mut ToolRegistry) {
    use crate::direct as d;
    reg!(
        r,
        "policy_check_session_readiness",
        "Check session readiness",
        schema!("session_key"),
        d::policy_check_session_readiness
    );
    reg!(
        r,
        "policy_validate_completion",
        "Validate task completion",
        schema!("session_key", "workflow_id", "task_id"),
        d::policy_validate_completion
    );
    reg!(
        r,
        "policy_validate_execution",
        "Validate execution",
        schema!("session_key", "workflow_id", "task_id"),
        d::policy_validate_execution
    );
    reg!(
        r,
        "policy_validate_parallel_completion",
        "Validate parallel completion",
        schema!("session_key", "workflow_id", "task_id"),
        d::policy_validate_parallel_completion
    );
    reg!(
        r,
        "policy_detect_scope_drift",
        "Detect scope drift",
        schema!("workflow_id", "task_id?", "output_text?"),
        d::policy_detect_scope_drift
    );
    reg!(
        r,
        "policy_require_context_refresh",
        "Require context refresh",
        schema!("workflow_id", "plan_id?", "task_id?", "last_fingerprint?"),
        d::policy_require_context_refresh
    );
}

#[cfg(feature = "sqlite")]
fn register_reminder_tools_stdio(r: &mut ToolRegistry) {
    use crate::direct as d;
    reg!(
        r,
        "reminder_check",
        "Check for stale/stuck/failed workflows and tasks",
        schema!(
            "staleExecutionMinutes?",
            "stuckTaskMinutes?",
            "includeFailed?"
        ),
        d::reminder_check
    );
    reg!(
        r,
        "reminder_list",
        "List stored reminders",
        schema!("workflow_id?", "acknowledged?", "limit?"),
        d::reminder_list
    );
    reg!(
        r,
        "reminder_acknowledge",
        "Acknowledge a reminder",
        schema!("id?", "workflowId?"),
        d::reminder_acknowledge
    );
}

#[cfg(feature = "sqlite")]
fn register_graph_tools_stdio(r: &mut ToolRegistry) {
    use crate::direct as d;
    reg!(
        r,
        "memory_create_entities",
        "Create graph entities",
        schema!("entities"),
        d::memory_create_entities
    );
    reg!(
        r,
        "memory_search_nodes",
        "Search graph nodes",
        schema!("query"),
        d::memory_search_nodes
    );
}

#[cfg(feature = "sqlite")]
fn register_capability_tools_stdio(r: &mut ToolRegistry) {
    use crate::direct as d;
    reg!(
        r,
        "capability_create_agent",
        "Create an agent",
        schema!("projectRoot", "name", "role", "description", "instructions"),
        d::capability_create_agent
    );
    reg!(
        r,
        "capability_create_skill",
        "Create a skill",
        schema!("projectRoot", "name", "description", "trigger", "steps"),
        d::capability_create_skill
    );
    reg!(
        r,
        "capability_list_agents",
        "List registered agents",
        schema!("projectRoot"),
        d::capability_list_agents
    );
    reg!(
        r,
        "capability_list_skills",
        "List registered skills",
        schema!("projectRoot"),
        d::capability_list_skills
    );
    reg!(
        r,
        "capability_list_templates",
        "List available templates",
        schema!(),
        d::capability_list_templates
    );
    reg!(
        r,
        "capability_match_agent",
        "Match agent for a task",
        schema!("projectRoot", "taskDescription"),
        d::capability_match_agent
    );
    reg!(
        r,
        "capability_scaffold_feature",
        "Scaffold a new feature",
        schema!("projectRoot", "name", "description"),
        d::capability_scaffold_feature
    );
    reg!(
        r,
        "capability_scaffold_mcp_server",
        "Scaffold an MCP server",
        schema!("projectRoot", "name", "description"),
        d::capability_scaffold_mcp_server
    );
    reg!(
        r,
        "capability_system_readiness",
        "Check system readiness",
        schema!("projectRoot"),
        d::capability_system_readiness
    );
    reg!(
        r,
        "capability_workflow_audit",
        "Audit a workflow",
        schema!("workflowId"),
        d::capability_workflow_audit
    );
}

#[cfg(feature = "sqlite")]
fn register_local_tools_stdio(r: &mut ToolRegistry) {
    use crate::direct as d;
    use crate::tools::local as l;
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
        d::local_sync
    );
    reg!(
        r,
        "local_push",
        "Push local state to DB",
        schema!("cwd", "workflow_id?"),
        d::local_push
    );
    reg!(
        r,
        "local_save_artifact",
        "Save artifact file locally",
        schema!("cwd", "category", "filename", "content"),
        l::local_save_artifact
    );
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_placeholder() {
        // Placeholder test
    }
}
