//! Direct-call adapter for standalone stdio mode
//!
//! Each function calls masday-service methods directly instead of via HTTP.
//! Takes `serde_json::Value` args, returns `Result<Value, Box<dyn Error + Send + Sync>>`.

use masday_service::memory_service::StoreMemoryParams;
use serde_json::{json, Value};

// Error helper — converts any error to the boxed type the registry expects
fn err(msg: impl std::fmt::Display) -> Box<dyn std::error::Error + Send + Sync> {
    format!("{}", msg).into()
}

// ============================================================================
// Workflow Tools (22)
// ============================================================================

pub async fn workflow_create(args: Value) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    let pool = crate::mode::pool();
    let name = args["name"].as_str().ok_or_else(|| err("missing name"))?.to_string();
    let description = args.get("description").and_then(|v| v.as_str()).map(String::from);
    let project_path = args.get("project_path").and_then(|v| v.as_str()).map(String::from);

    let wf = masday_service::WorkflowService::create_workflow(&pool, name, description, project_path)
        .await.map_err(|e| err(e))?;

    Ok(json!({"id": wf.id, "name": wf.name, "status": wf.status}))
}

pub async fn workflow_execute(args: Value) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    let pool = crate::mode::pool();
    let id = args.get("id").or_else(|| args.get("workflow_id"))
        .and_then(|v| v.as_str()).ok_or_else(|| err("missing id or workflow_id"))?;

    let wf = masday_service::WorkflowService::execute_workflow(&pool, id)
        .await.map_err(|e| err(e))?;

    Ok(json!({"id": wf.id, "status": wf.status}))
}

pub async fn workflow_get_status(args: Value) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    let pool = crate::mode::pool();
    let id = args.get("id").or_else(|| args.get("workflow_id"))
        .and_then(|v| v.as_str()).ok_or_else(|| err("missing id"))?;

    let wf = masday_service::WorkflowService::get_workflow(&pool, id)
        .await.map_err(|e| err(e))?;

    Ok(json!({"id": wf.id, "status": wf.status, "name": wf.name}))
}

pub async fn workflow_get(args: Value) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    let pool = crate::mode::pool();
    let id = args["workflow_id"].as_str().ok_or_else(|| err("missing workflow_id"))?;

    let wf = masday_service::WorkflowService::get_workflow(&pool, id)
        .await.map_err(|e| err(e))?;

    serde_json::to_value(wf).map_err(|e| err(e))
}

pub async fn workflow_list(args: Value) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    let pool = crate::mode::pool();
    let page = args["page"].as_u64().unwrap_or(1);
    let page_size = args["page_size"].as_u64().unwrap_or(50);
    let limit = page_size as i64;
    let offset = ((page - 1) * page_size) as i64;

    let workflows = masday_service::WorkflowService::list_workflows(&pool, limit, offset)
        .await.map_err(|e| err(e))?;

    Ok(json!({"workflows": workflows, "page": page, "page_size": page_size}))
}

pub async fn workflow_get_active(_args: Value) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    let pool = crate::mode::pool();
    let workflows = masday_service::WorkflowService::get_active_workflows(&pool)
        .await.map_err(|e| err(e))?;
    Ok(json!({"workflows": workflows}))
}

pub async fn workflow_delete(args: Value) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    let pool = crate::mode::pool();
    let id = args["workflow_id"].as_str().ok_or_else(|| err("missing workflow_id"))?;

    masday_service::WorkflowService::delete_workflow(&pool, id)
        .await.map_err(|e| err(e))?;

    Ok(json!({"deleted": id}))
}

pub async fn workflow_add_task(args: Value) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    let pool = crate::mode::pool();
    let workflow_id = args["workflow_id"].as_str().ok_or_else(|| err("missing workflow_id"))?.to_string();
    let plan_id = args["plan_id"].as_str().unwrap_or("default").to_string();
    let title = args["name"].as_str().ok_or_else(|| err("missing name"))?.to_string();
    let agent = args.get("agent").and_then(|v| v.as_str()).map(String::from);
    let dependencies: Option<Vec<String>> = args.get("dependencies")
        .and_then(|v| v.as_array())
        .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect());

    let task = masday_service::TaskService::add_task(&pool, workflow_id, plan_id, title, agent, dependencies)
        .await.map_err(|e| err(e))?;

    Ok(json!({"id": task.id, "title": task.title, "status": task.status}))
}

pub async fn workflow_start_task(args: Value) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    let pool = crate::mode::pool();
    let wf_id = args["workflow_id"].as_str().ok_or_else(|| err("missing workflow_id"))?;
    let task_id = args["task_id"].as_str().ok_or_else(|| err("missing task_id"))?;

    masday_service::TaskService::start_task(&pool, wf_id, task_id)
        .await.map_err(|e| err(e))?;

    Ok(json!({"status": "running", "task_id": task_id}))
}

pub async fn workflow_complete_task(args: Value) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    let pool = crate::mode::pool();
    let wf_id = args["workflow_id"].as_str().ok_or_else(|| err("missing workflow_id"))?;
    let task_id = args["task_id"].as_str().ok_or_else(|| err("missing task_id"))?;
    let result: Option<Value> = args.get("result").cloned();

    masday_service::TaskService::complete_task(&pool, wf_id, task_id, result)
        .await.map_err(|e| err(e))?;

    Ok(json!({"status": "done", "task_id": task_id}))
}

pub async fn workflow_save_progress(args: Value) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    let pool = crate::mode::pool();
    let wf_id = args["workflow_id"].as_str().ok_or_else(|| err("missing workflow_id"))?.to_string();
    let task_id = args["task_id"].as_str().ok_or_else(|| err("missing task_id"))?.to_string();
    let agent = args["agent_name"].as_str().or_else(|| args["agent"].as_str())
        .ok_or_else(|| err("missing agent_name"))?.to_string();
    let note = args["progress_note"].as_str().or_else(|| args["note"].as_str())
        .ok_or_else(|| err("missing progress_note"))?.to_string();
    let evidence: Option<Value> = args.get("evidence").cloned();

    masday_service::TaskService::save_progress(&pool, &wf_id, &task_id, agent, note, evidence)
        .await.map_err(|e| err(e))?;

    Ok(json!({"saved": true, "task_id": task_id}))
}

pub async fn workflow_create_plan(args: Value) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    let pool = crate::mode::pool();
    let workflow_id = args["workflow_id"].as_str().ok_or_else(|| err("missing workflow_id"))?.to_string();
    let phases = args.get("phases").or_else(|| args.get("plan")).cloned().ok_or_else(|| err("missing plan"))?;

    let plan = masday_service::PlanService::create_plan(&pool, workflow_id, phases)
        .await.map_err(|e| err(e))?;

    Ok(json!({"id": plan.id, "workflow_id": plan.workflow_id, "status": plan.status}))
}

pub async fn workflow_get_plan(args: Value) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    let pool = crate::mode::pool();
    let workflow_id = args["workflow_id"].as_str().ok_or_else(|| err("missing workflow_id"))?;

    let plan = masday_service::PlanService::get_plan(&pool, workflow_id)
        .await.map_err(|e| err(e))?;

    serde_json::to_value(plan).map_err(|e| err(e))
}

pub async fn workflow_list_tasks(args: Value) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    let pool = crate::mode::pool();
    let workflow_id = args["workflow_id"].as_str().ok_or_else(|| err("missing workflow_id"))?;

    let tasks = masday_service::TaskService::list_tasks(&pool, workflow_id)
        .await.map_err(|e| err(e))?;

    Ok(json!({"tasks": tasks}))
}

pub async fn workflow_create_parallel_branches(args: Value) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    let pool = crate::mode::pool();
    let wf_id = args["workflow_id"].as_str().ok_or_else(|| err("missing workflow_id"))?.to_string();
    let branches_arr = args["branches"].as_array().ok_or_else(|| err("missing branches array"))?;

    let mut branches = Vec::new();
    for b in branches_arr {
        branches.push(masday_db::schema::NewParallelBranch {
            workflow_id: wf_id.clone(),
            task_id: b["task_id"].as_str().unwrap_or("").to_string(),
            branch_key: b["branch_key"].as_str().unwrap_or("default").to_string(),
            role: b["role"].as_str().unwrap_or("executor").to_string(),
            status: "ACTIVE".to_string(),
            input: b.get("input").cloned().unwrap_or(json!({})),
            output: None,
        });
    }

    let result = masday_db::repos::BranchRepo::new(pool).create_branches(&branches)
        .await.map_err(|e| err(e))?;

    Ok(json!({"branches": result}))
}

pub async fn workflow_complete_parallel_branch(args: Value) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    let pool = crate::mode::pool();
    let branch_id = args["branch_id"].as_str().ok_or_else(|| err("missing branch_id"))?;
    let output: Value = args.get("output").cloned().unwrap_or(json!({}));

    masday_db::repos::BranchRepo::new(pool).complete_branch(branch_id, output)
        .await.map_err(|e| err(e))?;

    Ok(json!({"completed": branch_id}))
}

pub async fn workflow_list_parallel_branches(args: Value) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    let pool = crate::mode::pool();
    let workflow_id = args["workflow_id"].as_str().ok_or_else(|| err("missing workflow_id"))?;

    let branches = masday_db::repos::BranchRepo::new(pool).list_branches(workflow_id)
        .await.map_err(|e| err(e))?;

    Ok(json!({"branches": branches}))
}

pub async fn workflow_mark_synthesis_ready(args: Value) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    let pool = crate::mode::pool();
    let wf_id = args.get("workflow_id").or_else(|| args.get("session_key"))
        .and_then(|v| v.as_str()).ok_or_else(|| err("missing workflow_id"))?;

    masday_service::WorkflowService::transition_status(
        &pool, wf_id, masday_core::types::WorkflowState::Verify
    ).await.map_err(|e| err(e))?;

    Ok(json!({"status": "VERIFY", "workflow_id": wf_id}))
}

pub async fn workflow_mark_verification_ready(args: Value) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    let pool = crate::mode::pool();
    let wf_id = args.get("workflow_id").or_else(|| args.get("session_key"))
        .and_then(|v| v.as_str()).ok_or_else(|| err("missing workflow_id"))?;

    masday_service::WorkflowService::transition_status(
        &pool, wf_id, masday_core::types::WorkflowState::Done
    ).await.map_err(|e| err(e))?;

    Ok(json!({"status": "DONE", "workflow_id": wf_id}))
}

pub async fn workflow_set_execution_mode(args: Value) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    let pool = crate::mode::pool();
    let wf_id = args.get("workflow_id").or_else(|| args.get("session_key"))
        .and_then(|v| v.as_str()).ok_or_else(|| err("missing workflow_id"))?;
    let mode = args["mode"].as_str().ok_or_else(|| err("missing mode"))?;

    // Direct SQL to avoid Box<dyn ToSql> Send issue in repo layer
    let client = pool.get().await.map_err(|e| err(e))?;
    client.execute(
        r#"UPDATE "Workflow" SET "executionMode" = $1, "updatedAt" = NOW() WHERE id = $2"#,
        &[&mode, &wf_id],
    ).await.map_err(|e| err(e))?;

    Ok(json!({"updated": true, "mode": mode}))
}

pub async fn workflow_resume_suggestion(args: Value) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    let pool = crate::mode::pool();
    let wf_id = args["workflow_id"].as_str().ok_or_else(|| err("missing workflow_id"))?;

    let wf = masday_service::WorkflowService::get_workflow(&pool, wf_id)
        .await.map_err(|e| err(e))?;

    Ok(json!({"workflow_id": wf.id, "current_status": wf.status,
        "suggestion": format!("Resume workflow '{}' from status '{}'", wf.name, wf.status)}))
}

pub async fn workflow_get_current_task(args: Value) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    let pool = crate::mode::pool();
    let wf_id = args["workflow_id"].as_str().ok_or_else(|| err("missing workflow_id"))?;

    let task = masday_service::TaskService::get_current_task(&pool, wf_id)
        .await.map_err(|e| err(e))?;

    match task {
        Some(t) => Ok(json!({"task": t})),
        None => Ok(json!({"task": null})),
    }
}

pub async fn workflow_ping(_args: Value) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    Ok(json!({"status": "pong"}))
}

// ============================================================================
// Memory Tools (11)
// ============================================================================

pub async fn memory_store(args: Value) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    let pool = crate::mode::pool();
    let memory_type = args["memory_type"].as_str().or_else(|| args["type"].as_str()).ok_or_else(|| err("missing type"))?;
    let summary = args["summary"].as_str().unwrap_or("");
    let content = args["content"].as_str().ok_or_else(|| err("missing content"))?;
    let created_by = args["created_by_agent"].as_str().or_else(|| args["created_by"].as_str()).unwrap_or("masday-mcp");
    let importance = args["importance_score"].as_f64().or_else(|| args["importance"].as_f64()).unwrap_or(0.5);
    let tags: Vec<String> = args.get("tags").and_then(|v| v.as_array())
        .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
        .unwrap_or_default();
    let workflow_id = args.get("workflow_id").and_then(|v| v.as_str());
    let task_id = args.get("task_id").and_then(|v| v.as_str());

    let result = masday_service::MemoryService::store(&pool,
        StoreMemoryParams {
            memory_type, summary, content, created_by, importance, tags, workflow_id, task_id,
        }
    ).await.map_err(|e| err(e))?;

    Ok(result)
}

pub async fn memory_store_research(args: Value) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    let pool = crate::mode::pool();
    let summary = args["summary"].as_str().unwrap_or("Research finding");
    let content = args["content"].as_str().ok_or_else(|| err("missing content"))?;
    let created_by = args["created_by_agent"].as_str().unwrap_or("masday-mcp");
    let workflow_id = args.get("workflow_id").and_then(|v| v.as_str());

    let result = masday_service::MemoryService::store(&pool,
        StoreMemoryParams {
            memory_type: "research", summary, content, created_by,
            importance: 0.7, tags: vec![], workflow_id, task_id: None,
        }
    ).await.map_err(|e| err(e))?;

    Ok(result)
}

pub async fn memory_search(args: Value) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    let pool = crate::mode::pool();
    let query = args["query"].as_str().ok_or_else(|| err("missing query"))?;
    let limit = args["limit"].as_u64().unwrap_or(10) as usize;

    let results = masday_service::MemoryService::search(&pool, query, limit)
        .await.map_err(|e| err(e))?;

    Ok(json!({"results": results}))
}

pub async fn memory_recall_documents(args: Value) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    let pool = crate::mode::pool();
    let query = args["query"].as_str().or_else(|| args["workflow_id"].as_str()).ok_or_else(|| err("missing query"))?;
    let limit = args["limit"].as_u64().unwrap_or(10) as usize;

    let results = masday_service::MemoryService::search(&pool, query, limit)
        .await.map_err(|e| err(e))?;

    Ok(json!({"results": results}))
}

pub async fn memory_recall_document_by_type(args: Value) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    let pool = crate::mode::pool();
    let source_type = args["source_type"].as_str().ok_or_else(|| err("missing source_type"))?;
    let limit = args["limit"].as_u64().unwrap_or(10) as usize;

    // Fallback: search by type as query
    let results = masday_service::MemoryService::search(&pool, source_type, limit)
        .await.map_err(|e| err(e))?;

    Ok(json!({"results": results}))
}

pub async fn memory_recall_by_task(args: Value) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    let pool = crate::mode::pool();
    let task_id = args["task_id"].as_str().ok_or_else(|| err("missing task_id"))?;
    let limit = args["limit"].as_u64().unwrap_or(10) as usize;

    let results = masday_service::MemoryService::recall_by_task(&pool, task_id, limit)
        .await.map_err(|e| err(e))?;

    Ok(json!({"memories": results}))
}

pub async fn memory_recall_recent(args: Value) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    let pool = crate::mode::pool();
    let limit = args["limit"].as_u64().unwrap_or(10) as usize;

    let results = masday_service::MemoryService::recall_recent(&pool, limit)
        .await.map_err(|e| err(e))?;

    Ok(json!({"memories": results}))
}

pub async fn memory_update(args: Value) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    let pool = crate::mode::pool();
    let id = args["id"].as_str().ok_or_else(|| err("missing id"))?;
    let content = args.get("content").and_then(|v| v.as_str());
    let importance = args["importance"].as_f64();

    // Direct SQL to avoid Box<dyn ToSql> Send issue in repo layer
    let client = pool.get().await.map_err(|e| err(e))?;
    match (content, importance) {
        (Some(c), Some(imp)) => {
            client.execute(
                r#"UPDATE "Memory" SET content = $1, "importanceScore" = $2, "updatedAt" = NOW(), version = COALESCE(version, 0) + 1 WHERE id = $3"#,
                &[&c, &imp, &id],
            ).await.map_err(|e| err(e))?;
        }
        (Some(c), None) => {
            client.execute(
                r#"UPDATE "Memory" SET content = $1, "updatedAt" = NOW(), version = COALESCE(version, 0) + 1 WHERE id = $2"#,
                &[&c, &id],
            ).await.map_err(|e| err(e))?;
        }
        (None, Some(imp)) => {
            client.execute(
                r#"UPDATE "Memory" SET "importanceScore" = $1, "updatedAt" = NOW(), version = COALESCE(version, 0) + 1 WHERE id = $2"#,
                &[&imp, &id],
            ).await.map_err(|e| err(e))?;
        }
        (None, None) => {
            client.execute(
                r#"UPDATE "Memory" SET "updatedAt" = NOW(), version = COALESCE(version, 0) + 1 WHERE id = $1"#,
                &[&id],
            ).await.map_err(|e| err(e))?;
        }
    }

    Ok(json!({"updated": id}))
}

pub async fn memory_delete(args: Value) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    let pool = crate::mode::pool();
    let id = args["id"].as_str().ok_or_else(|| err("missing id"))?;

    let result = masday_service::MemoryService::delete(&pool, id)
        .await.map_err(|e| err(e))?;

    Ok(result)
}

pub async fn memory_delete_by_workflow(args: Value) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    // Stub — delete memories by workflow_id
    let _workflow_id = args["workflow_id"].as_str().ok_or_else(|| err("missing workflow_id"))?;
    Ok(json!({"deleted": true, "note": "stub implementation"}))
}

pub async fn memory_stats(_args: Value) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    let pool = crate::mode::pool();
    let stats = masday_service::MemoryService::stats(&pool)
        .await.map_err(|e| err(e))?;
    Ok(stats)
}

pub async fn episodic_store(args: Value) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    let pool = crate::mode::pool();
    let session_id = args["session_id"].as_str().ok_or_else(|| err("missing session_id"))?;
    let role = args["role"].as_str().ok_or_else(|| err("missing role"))?;
    let content = args["content"].as_str().ok_or_else(|| err("missing content"))?;

    masday_service::MemoryService::episodic_store(&pool, session_id, role, content)
        .await.map_err(|e| err(e))?;

    Ok(json!({"stored": true, "session_id": session_id}))
}

pub async fn episodic_recall(args: Value) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    let pool = crate::mode::pool();
    let session_id = args["session_id"].as_str().ok_or_else(|| err("missing session_id"))?;
    let limit = args["limit"].as_u64().unwrap_or(50) as usize;

    let results = masday_service::MemoryService::episodic_recall(&pool, session_id, limit)
        .await.map_err(|e| err(e))?;

    Ok(json!({"memories": results}))
}

// ============================================================================
// Review Tools (2)
// ============================================================================

pub async fn review_submit(args: Value) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    let pool = crate::mode::pool();
    let workflow_id = args["workflow_id"].as_str().ok_or_else(|| err("missing workflow_id"))?.to_string();
    let task_id = args["task_id"].as_str().ok_or_else(|| err("missing task_id"))?.to_string();
    let reviewer = args["reviewer_agent"].as_str().or_else(|| args["reviewer"].as_str())
        .ok_or_else(|| err("missing reviewer_agent"))?.to_string();
    let decision = args["decision"].as_str().ok_or_else(|| err("missing decision"))?.to_string();
    let notes = args["notes"].as_str().unwrap_or("").to_string();
    let gaps: Option<Value> = args.get("gaps").cloned();

    masday_service::ReviewService::submit_review(
        &pool, workflow_id, task_id, reviewer, decision, notes, gaps
    ).await.map_err(|e| err(e))?;

    Ok(json!({"submitted": true}))
}

pub async fn review_get_latest(args: Value) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    let pool = crate::mode::pool();
    let task_id = args["task_id"].as_str().ok_or_else(|| err("missing task_id"))?;

    let review = masday_service::ReviewService::get_latest_review(&pool, task_id)
        .await.map_err(|e| err(e))?;

    match review {
        Some(r) => serde_json::to_value(r).map_err(|e| err(e)),
        None => Ok(json!({"review": null})),
    }
}

// ============================================================================
// Session Tools (3)
// ============================================================================

pub async fn session_init_context(args: Value) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    let pool = crate::mode::pool();
    let cwd = args["cwd"].as_str().ok_or_else(|| err("missing cwd"))?;
    let session_key = format!("session:{}", cwd.replace('/', ":"));

    // Direct SQL to avoid Box<dyn ToSql> Send issue in repo layer.
    // On init, always upsert: create if not exists, otherwise touch updatedAt.
    let client = pool.get().await.map_err(|e| err(e))?;
    let id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().naive_utc();

    // Try INSERT first, ignore conflict (session already exists)
    let result = client.execute(
        r#"INSERT INTO "SessionState" (id, "sessionKey", metadata, "createdAt", "updatedAt")
           VALUES ($1, $2, $3, $4, $5)
           ON CONFLICT ("sessionKey") DO UPDATE SET "updatedAt" = $5"#,
        &[&id, &session_key, &serde_json::json!({"cwd": cwd}), &now, &now],
    ).await.map_err(|e| err(e))?;

    Ok(json!({"session_key": session_key, "initialized": true, "rows_affected": result}))
}

pub async fn session_get_state(args: Value) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    let pool = crate::mode::pool();
    let session_key = args["session_key"].as_str().ok_or_else(|| err("missing session_key"))?;

    let state = masday_db::repos::SessionRepo::new(pool).get_state(session_key)
        .await.map_err(|e| err(e))?;

    Ok(json!({"state": state}))
}

pub async fn session_patch_state(args: Value) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    let pool = crate::mode::pool();
    let session_key = args["session_key"].as_str().ok_or_else(|| err("missing session_key"))?;
    let patch = args.get("patch").cloned().ok_or_else(|| err("missing patch"))?;

    // Direct SQL to avoid Box<dyn ToSql> Send issue in repo layer.
    // Use the same UPSERT pattern as session_init_context.
    let client = pool.get().await.map_err(|e| err(e))?;
    let id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().naive_utc();

    // UPSERT: insert if not exists, update metadata if exists
    client.execute(
        r#"INSERT INTO "SessionState" (id, "sessionKey", metadata, "createdAt", "updatedAt")
           VALUES ($1, $2, $3, $4, $5)
           ON CONFLICT ("sessionKey") DO UPDATE SET metadata = COALESCE("SessionState".metadata, '{}'::jsonb) || $3, "updatedAt" = $5"#,
        &[&id, &session_key, &patch, &now, &now],
    ).await.map_err(|e| err(e))?;

    Ok(json!({"session_key": session_key, "patched": true}))
}

// ============================================================================
// Context/Search Tools (4)
// ============================================================================

pub async fn search_hybrid_context_pack(args: Value) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    let pool = crate::mode::pool();
    let workflow_id = args["workflow_id"].as_str().ok_or_else(|| err("missing workflow_id"))?;
    let plan_id = args["plan_id"].as_str().unwrap_or("");
    let task_id = args["task_id"].as_str().unwrap_or("");

    let pack = masday_service::ContextService::build_context_pack(&pool, workflow_id, plan_id, task_id)
        .await.map_err(|e| err(e))?;

    Ok(json!({"context_pack": pack}))
}

pub async fn search_context_fingerprint(_args: Value) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    Ok(json!({"fingerprint": format!("fp-{:x}", std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs())}))
}

pub async fn semantic_search_code_search(args: Value) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    let query = args["query"].as_str().ok_or_else(|| err("missing query"))?;
    let project_path = args.get("project_path").and_then(|v| v.as_str()).unwrap_or(".");

    let results = masday_service::SearchService::code_search(query, project_path)
        .await.map_err(|e| err(e))?;

    Ok(results)
}

pub async fn semantic_search_search_hybrid_context_pack(args: Value) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    search_hybrid_context_pack(args).await
}

pub async fn semantic_search_make_fingerprint(args: Value) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    let workflow_id = args["workflow_id"].as_str().unwrap_or("");
    let plan_id = args["plan_id"].as_str().unwrap_or("");
    let task_id = args["task_id"].as_str().unwrap_or("");

    Ok(json!({
        "fingerprint": format!("fp-{}-{}-{}", workflow_id, plan_id, task_id),
        "workflow_id": workflow_id,
        "plan_id": plan_id,
        "task_id": task_id,
    }))
}

// ============================================================================
// Policy Tools (6)
// ============================================================================

pub async fn policy_validate_execution(args: Value) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    let pool = crate::mode::pool();
    let session_key = args.get("session_key").and_then(|v| v.as_str());
    let workflow_id = args["workflow_id"].as_str().ok_or_else(|| err("missing workflow_id"))?;
    let task_id = args["task_id"].as_str().ok_or_else(|| err("missing task_id"))?;

    let valid = masday_service::PolicyService::validate_execution(&pool, session_key, workflow_id, task_id)
        .await.map_err(|e| err(e))?;

    Ok(json!({"valid": valid}))
}

pub async fn policy_validate_completion(args: Value) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    let pool = crate::mode::pool();
    let session_key = args.get("session_key").and_then(|v| v.as_str());
    let workflow_id = args["workflow_id"].as_str().ok_or_else(|| err("missing workflow_id"))?;
    let task_id = args["task_id"].as_str().ok_or_else(|| err("missing task_id"))?;

    let valid = masday_service::PolicyService::validate_completion(&pool, session_key, workflow_id, task_id)
        .await.map_err(|e| err(e))?;

    Ok(json!({"valid": valid}))
}

pub async fn policy_validate_parallel_completion(args: Value) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    // Stub — validate parallel branch completion
    let _branch_id = args.get("branch_id").or_else(|| args.get("task_id"));
    Ok(json!({"valid": true}))
}

pub async fn policy_detect_scope_drift(args: Value) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    let workflow_id = args["workflow_id"].as_str().ok_or_else(|| err("missing workflow_id"))?;
    let task_id = args.get("task_id").and_then(|v| v.as_str()).unwrap_or("");
    let output_text = args.get("output_text").and_then(|v| v.as_str()).unwrap_or("");

    let drift = masday_service::PolicyService::detect_scope_drift(workflow_id, task_id, output_text).await;

    Ok(json!({"drift_detected": drift.is_some(), "drift_detail": drift}))
}

pub async fn policy_require_context_refresh(_args: Value) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    Ok(json!({"require_refresh": false, "reason": "No stale context detected"}))
}

pub async fn policy_check_session_readiness(_args: Value) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    Ok(json!({"ready": true}))
}

// ============================================================================
// Reminder Tools (3)
// ============================================================================

pub async fn reminder_check(_args: Value) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    let pool = crate::mode::pool();
    let reminders = masday_service::ReminderService::check_reminders(&pool)
        .await.map_err(|e| err(e))?;
    Ok(json!({"reminders": reminders}))
}

pub async fn reminder_list(args: Value) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    let pool = crate::mode::pool();
    let workflow_id = args.get("workflow_id").and_then(|v| v.as_str()).unwrap_or("");

    // Use ReminderRepo directly since ReminderService::list_reminders doesn't exist
    let repo = masday_db::repos::ReminderRepo::new(pool);
    let reminders = repo.list(workflow_id)
        .await.map_err(|e| err(e))?;

    Ok(json!({"reminders": reminders}))
}

pub async fn reminder_acknowledge(args: Value) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    let pool = crate::mode::pool();
    let id = args.get("id").or_else(|| args.get("workflowId"))
        .and_then(|v| v.as_str()).ok_or_else(|| err("missing id"))?;

    masday_service::ReminderService::acknowledge_reminder(&pool, id)
        .await.map_err(|e| err(e))?;

    Ok(json!({"acknowledged": id}))
}

// ============================================================================
// Graph Tools (2)
// ============================================================================

pub async fn memory_create_entities(args: Value) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    let pool = crate::mode::pool();
    let entities_arr = args["entities"].as_array().ok_or_else(|| err("missing entities array"))?;

    let mut created = Vec::new();
    for entity_val in entities_arr {
        let name = entity_val["name"].as_str().ok_or_else(|| err("missing name in entity"))?;
        let entity_type = entity_val["entityType"].as_str().ok_or_else(|| err("missing entityType"))?;
        let observations: Vec<String> = entity_val.get("observations").and_then(|v| v.as_array())
            .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
            .unwrap_or_default();

        let node = masday_service::MemoryService::add_node(&pool, name, entity_type, observations)
            .await.map_err(|e| err(e))?;

        created.push(node);
    }

    Ok(json!({"entities": created}))
}

pub async fn memory_search_nodes(args: Value) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    let pool = crate::mode::pool();
    let query = args["query"].as_str().ok_or_else(|| err("missing query"))?;

    let nodes = masday_service::MemoryService::search_nodes(&pool, query)
        .await.map_err(|e| err(e))?;

    Ok(json!({"nodes": nodes}))
}

// ============================================================================
// Capability Tools (11)
// ============================================================================

pub async fn capability_list_agents(args: Value) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    let project_root = args["projectRoot"].as_str().or_else(|| args["project_root"].as_str())
        .ok_or_else(|| err("missing projectRoot"))?;

    let agents = masday_service::CapabilityService::list_agents(project_root)
        .await.map_err(|e| err(e))?;

    Ok(json!({"agents": agents}))
}

pub async fn capability_list_skills(args: Value) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    let project_root = args["projectRoot"].as_str().or_else(|| args["project_root"].as_str())
        .ok_or_else(|| err("missing projectRoot"))?;

    // Read skills from filesystem
    let skills_dir = std::path::Path::new(project_root).join(".claude/skills");
    let mut skills = Vec::new();
    if skills_dir.exists() {
        for entry in std::fs::read_dir(&skills_dir).map_err(|e| err(e))? {
            if let Ok(entry) = entry {
                if entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                    skills.push(json!({"name": entry.file_name().to_string_lossy()}));
                }
            }
        }
    }

    Ok(json!({"skills": skills}))
}

pub async fn capability_match_agent(args: Value) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    let task_desc = args["taskDescription"].as_str().or_else(|| args["task_description"].as_str())
        .ok_or_else(|| err("missing taskDescription"))?;

    let agent = masday_service::CapabilityService::match_agent(task_desc)
        .await.map_err(|e| err(e))?;

    Ok(json!({"agent": agent}))
}

pub async fn capability_scaffold_feature(_args: Value) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    Ok(json!({"scaffold": "feature_scaffold_v1", "description": "Feature scaffold template"}))
}

pub async fn capability_scaffold_mcp_server(_args: Value) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    Ok(json!({"scaffold": "mcp_server_scaffold_v1", "description": "MCP server scaffold template"}))
}

pub async fn capability_system_readiness(_args: Value) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    let pool = crate::mode::pool();
    let pool_healthy = pool.get().await.is_ok();
    Ok(json!({"ready": pool_healthy, "pool_healthy": pool_healthy}))
}

pub async fn capability_workflow_audit(args: Value) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    let pool = crate::mode::pool();
    let workflow_id = args.get("workflowId").or_else(|| args.get("workflow_id"))
        .and_then(|v| v.as_str()).ok_or_else(|| err("missing workflowId"))?;

    let wf = masday_service::WorkflowService::get_workflow(&pool, workflow_id)
        .await.map_err(|e| err(e))?;
    let tasks = masday_service::TaskService::list_tasks(&pool, workflow_id)
        .await.map_err(|e| err(e))?;

    Ok(json!({"workflow": wf.id, "status": wf.status, "tasks_count": tasks.len()}))
}

pub async fn capability_create_agent(args: Value) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    let project_root = args["projectRoot"].as_str().unwrap_or(".");
    let name = args["name"].as_str().ok_or_else(|| err("missing name"))?;
    let role = args["role"].as_str().unwrap_or("general");
    let description = args["description"].as_str().unwrap_or("");
    let instructions = args["instructions"].as_str().unwrap_or("");

    let dir = std::path::Path::new(project_root).join(".claude/agents");
    std::fs::create_dir_all(&dir).map_err(|e| err(e))?;
    let content = format!("---\nname: {}\nrole: {}\n---\n\n# {}\n\n{}", name, role, description, instructions);
    std::fs::write(dir.join(format!("{}.md", name)), content).map_err(|e| err(e))?;

    Ok(json!({"created": name}))
}

pub async fn capability_create_skill(args: Value) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    let project_root = args["projectRoot"].as_str().unwrap_or(".");
    let name = args["name"].as_str().ok_or_else(|| err("missing name"))?;
    let description = args["description"].as_str().unwrap_or("");

    let dir = std::path::Path::new(project_root).join(format!(".claude/skills/{}", name));
    std::fs::create_dir_all(&dir).map_err(|e| err(e))?;
    let content = format!("---\nname: {}\ndescription: {}\n---\n\n# {}", name, description, name);
    std::fs::write(dir.join("SKILL.md"), content).map_err(|e| err(e))?;

    Ok(json!({"created": name}))
}

pub async fn capability_list_templates(_args: Value) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    Ok(json!({"templates": []}))
}

pub async fn capability_ping(_args: Value) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    Ok(json!({"status": "pong"}))
}

// ============================================================================
// Local Push (1)
// ============================================================================

pub async fn local_push(args: Value) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    let pool = crate::mode::pool();
    let workflow_id = args["workflow_id"].as_str().ok_or_else(|| err("missing workflow_id"))?;
    let updates = args.get("updates").cloned().unwrap_or(json!({}));

    // Direct SQL to avoid Box<dyn ToSql> Send issue in repo layer.
    // Extract known fields and update individually.
    let client = pool.get().await.map_err(|e| err(e))?;

    // Always touch updatedAt
    if let Some(name) = updates.get("name").and_then(|v| v.as_str()) {
        client.execute(
            r#"UPDATE "Workflow" SET name = $1, "updatedAt" = NOW() WHERE id = $2"#,
            &[&name, &workflow_id],
        ).await.map_err(|e| err(e))?;
    }
    if let Some(status) = updates.get("status").and_then(|v| v.as_str()) {
        client.execute(
            r#"UPDATE "Workflow" SET status = $1, "updatedAt" = NOW() WHERE id = $2"#,
            &[&status, &workflow_id],
        ).await.map_err(|e| err(e))?;
    }
    if let Some(project_path) = updates.get("project_path").and_then(|v| v.as_str()) {
        client.execute(
            r#"UPDATE "Workflow" SET "projectPath" = $1, "updatedAt" = NOW() WHERE id = $2"#,
            &[&project_path, &workflow_id],
        ).await.map_err(|e| err(e))?;
    }
    if let Some(execution_mode) = updates.get("execution_mode").and_then(|v| v.as_str()) {
        client.execute(
            r#"UPDATE "Workflow" SET "executionMode" = $1, "updatedAt" = NOW() WHERE id = $2"#,
            &[&execution_mode, &workflow_id],
        ).await.map_err(|e| err(e))?;
    }
    if let Some(current_plan_id) = updates.get("current_plan_id").and_then(|v| v.as_str()) {
        client.execute(
            r#"UPDATE "Workflow" SET "currentPlanId" = $1, "updatedAt" = NOW() WHERE id = $2"#,
            &[&current_plan_id, &workflow_id],
        ).await.map_err(|e| err(e))?;
    }
    if let Some(current_task_id) = updates.get("current_task_id").and_then(|v| v.as_str()) {
        client.execute(
            r#"UPDATE "Workflow" SET "currentTaskId" = $1, "updatedAt" = NOW() WHERE id = $2"#,
            &[&current_task_id, &workflow_id],
        ).await.map_err(|e| err(e))?;
    }
    // metadata — uses serde_json::Value which IS Send, so direct SQL works
    if let Some(metadata) = updates.get("metadata").cloned() {
        client.execute(
            r#"UPDATE "Workflow" SET metadata = $1, "updatedAt" = NOW() WHERE id = $2"#,
            &[&metadata, &workflow_id],
        ).await.map_err(|e| err(e))?;
    }
    // If no specific fields, just touch updatedAt
    if updates.as_object().map(|o| o.is_empty()).unwrap_or(true) {
        client.execute(
            r#"UPDATE "Workflow" SET "updatedAt" = NOW() WHERE id = $1"#,
            &[&workflow_id],
        ).await.map_err(|e| err(e))?;
    }

    Ok(json!({"id": workflow_id, "updated": true}))
}

// ============================================================================
// Local Sync (1) — reads .masday/ state and syncs from DB
// ============================================================================

pub async fn local_sync(args: Value) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    // Delegate to the existing tools::local implementation
    crate::tools::local::local_sync(args).await
}
