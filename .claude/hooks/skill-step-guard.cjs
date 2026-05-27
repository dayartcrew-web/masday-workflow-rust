#!/usr/bin/env node
// skill-step-guard.js — PreToolUse hook
// Validates step transitions for multi-step skills by tracking real evidence:
// - File creation (Write tool with .test.ts paths for TDD RED phase)
// - File editing (Edit tool for GREEN/REFACTOR phases)
// - Test execution (tests_run / Bash with test commands)
// - MCP tool calls (workflow tools, memory, search, etc.)
//
// BLOCKS tool calls that violate step ordering.
// WARNS when step prerequisites are incomplete.

const fs = require("fs");
const path = require("path");
const os = require("os");

const STATE_DIR = path.join(os.tmpdir(), "masday-step-guard");

// ── Skill Step Definitions ──────────────────────────────────────────────────
// Each skill defines ordered steps with validation criteria.
// A step is "complete" when ALL required evidence is present.

const SKILL_STEPS = {
  // ═══════════════════════════════════════════════════════════════════════════
  // TDD SKILLS
  // ═══════════════════════════════════════════════════════════════════════════
  "masday-tdd": {
    steps: [
      { name: "RED", order: 1, description: "Write failing tests", requireEvidence: ["testFileWritten"], blockUntil: null },
      { name: "RED_VERIFY", order: 2, description: "Verify tests fail (RED confirmation)", requireEvidence: ["testsRun"], blockUntil: "RED" },
      { name: "GREEN", order: 3, description: "Implement minimum code to pass tests", requireEvidence: ["sourceFileEdited"], blockUntil: "RED_VERIFY" },
      { name: "GREEN_VERIFY", order: 4, description: "Verify tests pass (GREEN confirmation)", requireEvidence: ["testsPass"], blockUntil: "GREEN" },
      { name: "REFACTOR", order: 5, description: "Clean up code while tests stay green", requireEvidence: ["sourceFileEdited"], blockUntil: "GREEN_VERIFY" },
      { name: "COVERAGE", order: 6, description: "Verify 80%+ coverage", requireEvidence: ["coverageChecked"], blockUntil: "REFACTOR" },
    ],
  },

  // ═══════════════════════════════════════════════════════════════════════════
  // WORKFLOW LIFECYCLE SKILLS
  // ═══════════════════════════════════════════════════════════════════════════
  "masday-workflow-new": {
    steps: [
      { name: "READINESS", order: 1, description: "Check system readiness", requireEvidence: ["tool:capability_system_readiness"], blockUntil: null },
      { name: "CONTEXT", order: 2, description: "Search context (memory + code)", requireEvidence: ["tool:memory_search", "tool:memory_recall_recent", "tool:semantic-search_code_search"], blockUntil: "READINESS" },
      { name: "CREATE", order: 3, description: "Create workflow", requireEvidence: ["tool:workflow_create"], blockUntil: "CONTEXT" },
      { name: "CONTEXT_PACK", order: 4, description: "Build hybrid context pack", requireEvidence: ["tool:semantic-search_search_hybrid_context_pack", "tool:memory_recall_documents"], blockUntil: "CREATE" },
      { name: "AGENT_MATCH", order: 5, description: "Match best agent for tasks", requireEvidence: ["tool:capability_list_agents", "tool:capability_match_agent"], blockUntil: "CONTEXT_PACK" },
      { name: "SKILL_VERIFY", order: 6, description: "Verify skill exists", requireEvidence: ["tool:capability_list_skills"], blockUntil: "AGENT_MATCH" },
      { name: "EXECUTE", order: 7, description: "Execute workflow (GATE)", requireEvidence: [], blockUntil: "SKILL_VERIFY", isGate: true },
      { name: "STORE", order: 8, description: "Store artifacts in memory", requireEvidence: ["tool:memory_store"], blockUntil: "EXECUTE" },
    ],
  },
  "masday-workflow-plan": {
    steps: [
      { name: "ANALYZE", order: 1, description: "Analyze codebase for planning", requireEvidence: ["tool:semantic-search_code_search"], blockUntil: null },
      { name: "MEMORY", order: 2, description: "Search memory for past patterns", requireEvidence: ["tool:memory_search"], blockUntil: "ANALYZE" },
      { name: "PLAN", order: 3, description: "Create execution plan", requireEvidence: ["tool:workflow_createPlan"], blockUntil: "MEMORY" },
      { name: "TASKS", order: 4, description: "Add tasks to plan", requireEvidence: ["tool:workflow_addTask"], blockUntil: "PLAN" },
    ],
  },
  "masday-workflow-run": {
    steps: [
      { name: "VERIFY", order: 1, description: "Verify workflow exists and is ready", requireEvidence: ["tool:workflow_get", "tool:workflow_getStatus"], blockUntil: null },
      { name: "REVIEW_TASKS", order: 2, description: "List and review tasks", requireEvidence: ["tool:workflow_listTasks"], blockUntil: "VERIFY" },
      { name: "EXECUTE", order: 3, description: "Execute the workflow", requireEvidence: ["tool:workflow_execute"], blockUntil: "REVIEW_TASKS", isGate: true },
      { name: "MONITOR", order: 4, description: "Monitor task execution", requireEvidence: ["tool:workflow_saveProgress", "tool:policy_validate_completion"], blockUntil: "EXECUTE" },
      { name: "STORE", order: 5, description: "Store execution artifacts", requireEvidence: ["tool:memory_store"], blockUntil: "MONITOR" },
    ],
  },
  "masday-workflow-init": {
    steps: [
      { name: "MEMORY", order: 1, description: "Search memory for context", requireEvidence: ["tool:memory_search", "tool:memory_recall_recent"], blockUntil: null },
      { name: "SCAN", order: 2, description: "Scan relevant code", requireEvidence: ["tool:semantic-search_code_search"], blockUntil: "MEMORY" },
      { name: "READINESS", order: 3, description: "Check system readiness", requireEvidence: ["tool:capability_system_readiness"], blockUntil: "SCAN" },
      { name: "CREATE", order: 4, description: "Create the workflow", requireEvidence: ["tool:workflow_create"], blockUntil: "READINESS" },
      { name: "STORE", order: 5, description: "Store initial context", requireEvidence: ["tool:memory_store"], blockUntil: "CREATE" },
    ],
  },
  "masday-workflow-fix": {
    steps: [
      { name: "DIAGNOSE", order: 1, description: "Get workflow state and diagnose failures", requireEvidence: ["tool:workflow_get", "tool:workflow_listTasks"], blockUntil: null },
      { name: "SEARCH", order: 2, description: "Search for solutions", requireEvidence: ["tool:semantic-search_code_search", "tool:memory_search"], blockUntil: "DIAGNOSE" },
      { name: "FIX", order: 3, description: "Add fix task and validate", requireEvidence: ["tool:policy_validate_execution"], blockUntil: "SEARCH" },
      { name: "STORE", order: 4, description: "Store the fix", requireEvidence: ["tool:memory_store"], blockUntil: "FIX" },
    ],
  },
  "masday-workflow-verify": {
    steps: [
      { name: "LOAD", order: 1, description: "Get workflow state", requireEvidence: ["tool:workflow_get", "tool:workflow_listTasks"], blockUntil: null },
      { name: "VALIDATE", order: 2, description: "Validate tasks against acceptance criteria", requireEvidence: ["tool:policy_validate_completion"], blockUntil: "LOAD" },
      { name: "TEST", order: 3, description: "Run tests", requireEvidence: ["testsRun"], blockUntil: "VALIDATE" },
      { name: "DRIFT", order: 4, description: "Detect scope drift", requireEvidence: ["tool:policy_detect_scope_drift"], blockUntil: "TEST" },
      { name: "STORE", order: 5, description: "Store verification results", requireEvidence: ["tool:memory_store"], blockUntil: "DRIFT" },
    ],
  },
  "masday-workflow-audit": {
    steps: [
      { name: "AUDIT", order: 1, description: "Run system audit", requireEvidence: ["tool:capability_workflow_audit"], blockUntil: null },
      { name: "INSPECT", order: 2, description: "Inspect active workflows", requireEvidence: ["tool:workflow_get", "tool:workflow_listTasks"], blockUntil: "AUDIT" },
      { name: "MEMORY", order: 3, description: "Check memory health", requireEvidence: ["tool:memory_stats", "tool:memory_search"], blockUntil: "INSPECT" },
    ],
  },
  "masday-workflow-add-task": {
    steps: [
      { name: "VERIFY", order: 1, description: "Verify the workflow exists", requireEvidence: ["tool:workflow_get"], blockUntil: null },
      { name: "REVIEW", order: 2, description: "Review current tasks", requireEvidence: ["tool:workflow_listTasks"], blockUntil: "VERIFY" },
      { name: "MATCH", order: 3, description: "Match the best agent", requireEvidence: ["tool:capability_match_agent"], blockUntil: "REVIEW" },
      { name: "ADD", order: 4, description: "Add the task", requireEvidence: ["tool:workflow_addTask"], blockUntil: "MATCH" },
    ],
  },
  "masday-workflow-discipline": {
    steps: [
      { name: "STATE", order: 1, description: "Get current workflow state", requireEvidence: ["tool:workflow_get", "tool:workflow_listTasks"], blockUntil: null },
      { name: "SESSION", order: 2, description: "Check session readiness", requireEvidence: ["tool:policy_check_session_readiness"], blockUntil: "STATE" },
      { name: "EXEC_VALIDATE", order: 3, description: "Validate execution permission", requireEvidence: ["tool:policy_validate_execution"], blockUntil: "SESSION" },
      { name: "DRIFT", order: 4, description: "Detect scope drift", requireEvidence: ["tool:policy_detect_scope_drift"], blockUntil: "EXEC_VALIDATE" },
      { name: "COMPLETION", order: 5, description: "Validate task completion", requireEvidence: ["tool:policy_validate_completion"], blockUntil: "DRIFT" },
    ],
  },
  "masday-workflow-continue": {
    steps: [
      { name: "DETECT", order: 1, description: "Detect active workflow", requireEvidence: ["tool:workflow_getActive"], blockUntil: null },
      { name: "LOAD", order: 2, description: "Load full state", requireEvidence: ["tool:workflow_getPlan", "tool:workflow_listTasks"], blockUntil: "DETECT" },
      { name: "RESUME", order: 3, description: "Resume interrupted task", requireEvidence: ["tool:workflow_startTask"], blockUntil: "LOAD" },
      { name: "PROGRESS", order: 4, description: "Save progress", requireEvidence: ["tool:workflow_saveProgress"], blockUntil: "RESUME" },
    ],
  },
  "masday-workflow-next": {
    steps: [
      { name: "SCAN", order: 1, description: "Scan workflow state", requireEvidence: ["tool:workflow_getActive"], blockUntil: null },
      { name: "LOAD", order: 2, description: "Load workflow context", requireEvidence: ["tool:workflow_getPlan", "tool:workflow_listTasks"], blockUntil: "SCAN" },
      { name: "ROUTE", order: 3, description: "Route and start task", requireEvidence: ["tool:workflow_startTask"], blockUntil: "LOAD" },
      { name: "PROGRESS", order: 4, description: "Save progress", requireEvidence: ["tool:workflow_saveProgress"], blockUntil: "ROUTE" },
    ],
  },

  // ═══════════════════════════════════════════════════════════════════════════
  // RESEARCH & ANALYSIS SKILLS
  // ═══════════════════════════════════════════════════════════════════════════
  "masday-research": {
    steps: [
      { name: "SEARCH", order: 1, description: "Search for existing knowledge", requireEvidence: ["tool:memory_search"], blockUntil: null },
      { name: "CODEBASE", order: 2, description: "Search codebase for related code", requireEvidence: ["tool:semantic-search_code_search"], blockUntil: "SEARCH" },
      { name: "STORE", order: 3, description: "Store research findings", requireEvidence: ["tool:memory_store"], blockUntil: "CODEBASE" },
    ],
  },
  "masday-web-research": {
    steps: [
      { name: "MEMORY", order: 1, description: "Search memory for existing knowledge", requireEvidence: ["tool:memory_search"], blockUntil: null },
      { name: "WEB", order: 2, description: "Web search for information", requireEvidence: ["webSearch"], blockUntil: "MEMORY" },
      { name: "CODEBASE", order: 3, description: "Search codebase for related code", requireEvidence: ["tool:semantic-search_code_search"], blockUntil: "WEB" },
      { name: "STORE", order: 4, description: "Store research findings", requireEvidence: ["tool:memory_store_research"], blockUntil: "CODEBASE" },
    ],
  },
  "masday-code-analyze": {
    steps: [
      { name: "SCAN", order: 1, description: "Scan directory structure", requireEvidence: ["tool:filesystem_list"], blockUntil: null },
      { name: "READ", order: 2, description: "Read source files", requireEvidence: ["tool:filesystem_read"], blockUntil: "SCAN" },
      { name: "SEARCH", order: 3, description: "Semantic code search", requireEvidence: ["tool:semantic-search_code_search"], blockUntil: "READ" },
      { name: "CONTEXT", order: 4, description: "Build context pack", requireEvidence: ["tool:semantic-search_search_hybrid_context_pack"], blockUntil: "SEARCH" },
    ],
  },
  "masday-context-retrieval": {
    steps: [
      { name: "WORKFLOW", order: 1, description: "Get active workflow", requireEvidence: ["tool:workflow_getActive"], blockUntil: null },
      { name: "PLAN", order: 2, description: "Get plan and tasks", requireEvidence: ["tool:workflow_getPlan", "tool:workflow_listTasks"], blockUntil: "WORKFLOW" },
      { name: "CONTEXT", order: 3, description: "Build context pack", requireEvidence: ["tool:semantic-search_search_hybrid_context_pack"], blockUntil: "PLAN" },
      { name: "MEMORY", order: 4, description: "Recall documents and memory", requireEvidence: ["tool:memory_recall_documents", "tool:memory_search"], blockUntil: "CONTEXT" },
    ],
  },
  "masday-memory-search": {
    steps: [
      { name: "RECALL_TYPE", order: 1, description: "Recall documents by type", requireEvidence: ["tool:memory_recall_document_by_type"], blockUntil: null },
      { name: "SEARCH", order: 2, description: "Search memories", requireEvidence: ["tool:memory_search"], blockUntil: "RECALL_TYPE" },
      { name: "RECALL_RECENT", order: 3, description: "Recall recent memories", requireEvidence: ["tool:memory_recall_recent"], blockUntil: "SEARCH" },
    ],
  },

  // ═══════════════════════════════════════════════════════════════════════════
  // SCAFFOLDING & CREATION SKILLS
  // ═══════════════════════════════════════════════════════════════════════════
  "masday-create-agent": {
    steps: [
      { name: "LIST", order: 1, description: "List existing agents", requireEvidence: ["tool:capability_list_agents"], blockUntil: null },
      { name: "TEMPLATES", order: 2, description: "List templates", requireEvidence: ["tool:capability_list_templates"], blockUntil: "LIST" },
      { name: "CREATE", order: 3, description: "Create the agent", requireEvidence: ["tool:capability_create_agent"], blockUntil: "TEMPLATES" },
    ],
  },
  "masday-create-skill": {
    steps: [
      { name: "LIST", order: 1, description: "List existing skills", requireEvidence: ["tool:capability_list_skills"], blockUntil: null },
      { name: "TEMPLATES", order: 2, description: "List templates", requireEvidence: ["tool:capability_list_templates"], blockUntil: "LIST" },
      { name: "CREATE", order: 3, description: "Create the skill", requireEvidence: ["tool:capability_create_skill"], blockUntil: "TEMPLATES" },
    ],
  },
  "masday-create-mcp-skill": {
    steps: [
      { name: "SCAN", order: 1, description: "Scan package structure", requireEvidence: ["tool:filesystem_list"], blockUntil: null },
      { name: "TEMPLATES", order: 2, description: "List templates", requireEvidence: ["tool:capability_list_templates"], blockUntil: "SCAN" },
      { name: "WRITE", order: 3, description: "Write skill and test files", requireEvidence: ["sourceFileWritten"], blockUntil: "TEMPLATES" },
      { name: "BUILD", order: 4, description: "Build and verify", requireEvidence: ["tool:npm_run"], blockUntil: "WRITE" },
    ],
  },
  "masday-create-command": {
    steps: [
      { name: "SCAN", order: 1, description: "List existing commands", requireEvidence: ["tool:filesystem_list"], blockUntil: null },
      { name: "WRITE", order: 2, description: "Write command file", requireEvidence: ["sourceFileWritten"], blockUntil: "SCAN" },
    ],
  },

  // ═══════════════════════════════════════════════════════════════════════════
  // PARALLEL EXECUTION SKILLS
  // ═══════════════════════════════════════════════════════════════════════════
  "masday-parallel-execution": {
    steps: [
      { name: "LOAD", order: 1, description: "Load workflow and tasks", requireEvidence: ["tool:workflow_get", "tool:workflow_listTasks"], blockUntil: null },
      { name: "CONTEXT", order: 2, description: "Recall context documents", requireEvidence: ["tool:memory_recall_documents"], blockUntil: "LOAD" },
      { name: "BRANCHES", order: 3, description: "Create parallel branches", requireEvidence: ["tool:workflow_createParallelBranches"], blockUntil: "CONTEXT" },
      { name: "COMPLETE", order: 4, description: "Complete branches", requireEvidence: ["tool:workflow_completeParallelBranch"], blockUntil: "BRANCHES" },
      { name: "STORE", order: 5, description: "Store results", requireEvidence: ["tool:memory_store"], blockUntil: "COMPLETE" },
    ],
  },
  "masday-parallel-research": {
    steps: [
      { name: "CONTEXT", order: 1, description: "Get workflow context", requireEvidence: ["tool:workflow_getActive"], blockUntil: null },
      { name: "BRANCHES", order: 2, description: "Create parallel branches", requireEvidence: ["tool:workflow_createParallelBranches"], blockUntil: "CONTEXT" },
      { name: "COMPLETE", order: 3, description: "Complete branches", requireEvidence: ["tool:workflow_completeParallelBranch"], blockUntil: "BRANCHES" },
      { name: "STORE", order: 4, description: "Store research findings", requireEvidence: ["tool:memory_store_research"], blockUntil: "COMPLETE" },
    ],
  },

  // ═══════════════════════════════════════════════════════════════════════════
  // DEPLOYMENT & OPS SKILLS
  // ═══════════════════════════════════════════════════════════════════════════
  "masday-deploy-check": {
    steps: [
      { name: "INSTALL", order: 1, description: "Install dependencies", requireEvidence: ["tool:npm_install"], blockUntil: null },
      { name: "BUILD", order: 2, description: "Build project", requireEvidence: ["tool:npm_run"], blockUntil: "INSTALL" },
      { name: "TEST", order: 3, description: "Run tests", requireEvidence: ["testsRun"], blockUntil: "BUILD" },
      { name: "GIT_CHECK", order: 4, description: "Check git state", requireEvidence: ["tool:git_status", "tool:git_diff"], blockUntil: "TEST" },
      { name: "PIPELINE", order: 5, description: "Check CI/CD pipeline", requireEvidence: ["tool:cicd_pipeline_status"], blockUntil: "GIT_CHECK" },
    ],
  },
  "masday-docker-ops": {
    steps: [
      { name: "CHECK", order: 1, description: "Check running containers", requireEvidence: ["tool:docker_ps"], blockUntil: null },
      { name: "READ", order: 2, description: "Read Dockerfile", requireEvidence: ["tool:filesystem_read"], blockUntil: "CHECK" },
      { name: "BUILD", order: 3, description: "Build image", requireEvidence: ["tool:docker_build"], blockUntil: "READ" },
      { name: "RUN", order: 4, description: "Run container", requireEvidence: ["tool:docker_run"], blockUntil: "BUILD" },
    ],
  },
  "masday-cicd-ops": {
    steps: [
      { name: "STATUS", order: 1, description: "Check pipeline status", requireEvidence: ["tool:cicd_pipeline_status"], blockUntil: null },
      { name: "INSPECT", order: 2, description: "Inspect failed runs", requireEvidence: ["tool:cicd_runs_view"], blockUntil: "STATUS" },
      { name: "CONTEXT", order: 3, description: "Recall and store context", requireEvidence: ["tool:memory_recall_recent", "tool:memory_store"], blockUntil: "INSPECT" },
    ],
  },

  // ═══════════════════════════════════════════════════════════════════════════
  // GIT & GITHUB SKILLS
  // ═══════════════════════════════════════════════════════════════════════════
  "masday-git-workflow": {
    steps: [
      { name: "STATUS", order: 1, description: "Check current state", requireEvidence: ["tool:git_status"], blockUntil: null },
      { name: "DIFF", order: 2, description: "Review changes", requireEvidence: ["tool:git_diff"], blockUntil: "STATUS" },
      { name: "COMMIT", order: 3, description: "Commit with conventional format", requireEvidence: ["tool:git_commit"], blockUntil: "DIFF" },
    ],
  },
  "masday-github-flow": {
    steps: [
      { name: "STATUS", order: 1, description: "Review current changes", requireEvidence: ["tool:git_status", "tool:git_diff"], blockUntil: null },
      { name: "CONTEXT", order: 2, description: "Recall workflow context", requireEvidence: ["tool:memory_recall_documents"], blockUntil: "STATUS" },
      { name: "CHECK_PR", order: 3, description: "Check existing PRs", requireEvidence: ["tool:github_pr_list"], blockUntil: "CONTEXT" },
      { name: "COMMIT", order: 4, description: "Commit changes", requireEvidence: ["tool:git_commit"], blockUntil: "CHECK_PR" },
      { name: "PR", order: 5, description: "Create pull request", requireEvidence: ["tool:github_pr_create"], blockUntil: "COMMIT" },
    ],
  },
  "masday-github-pr": {
    steps: [
      { name: "STATUS", order: 1, description: "Review changes", requireEvidence: ["tool:git_status", "tool:git_diff"], blockUntil: null },
      { name: "CHECK_PR", order: 2, description: "Check existing PRs", requireEvidence: ["tool:github_pr_list"], blockUntil: "STATUS" },
      { name: "CONTEXT", order: 3, description: "Recall workflow context", requireEvidence: ["tool:memory_recall_documents"], blockUntil: "CHECK_PR" },
      { name: "COMMIT", order: 4, description: "Commit changes", requireEvidence: ["tool:git_commit"], blockUntil: "CONTEXT" },
      { name: "PR", order: 5, description: "Create pull request", requireEvidence: ["tool:github_pr_create"], blockUntil: "COMMIT" },
    ],
  },

  // ═══════════════════════════════════════════════════════════════════════════
  // AUTOPILOT SKILL
  // ═══════════════════════════════════════════════════════════════════════════
  "masday-autopilot": {
    steps: [
      { name: "INIT", order: 1, description: "Init .masday/ and get active workflow", requireEvidence: ["tool:local_init", "tool:workflow_getActive"], blockUntil: null },
      { name: "PLAN", order: 2, description: "Get plan and count tasks", requireEvidence: ["tool:workflow_getPlan"], blockUntil: "INIT" },
      { name: "START", order: 3, description: "Start task and load context", requireEvidence: ["tool:workflow_startTask", "tool:semantic-search_search_hybrid_context_pack"], blockUntil: "PLAN" },
      { name: "PROGRESS", order: 4, description: "Save progress after execution", requireEvidence: ["tool:workflow_saveProgress"], blockUntil: "START" },
      { name: "REVIEW", order: 5, description: "Submit review", requireEvidence: ["tool:review_submit"], blockUntil: "PROGRESS" },
      { name: "SYNC", order: 6, description: "Sync local state", requireEvidence: ["tool:local_sync"], blockUntil: "REVIEW" },
    ],
  },

  // ═══════════════════════════════════════════════════════════════════════════
  // ANALYSIS SKILLS
  // ═══════════════════════════════════════════════════════════════════════════
  "masday-sequential-thinking": {
    steps: [
      { name: "SURVEY", order: 1, description: "Survey the landscape", requireEvidence: ["tool:filesystem_list"], blockUntil: null },
      { name: "READ", order: 2, description: "Read entry points", requireEvidence: ["tool:filesystem_read"], blockUntil: "SURVEY" },
      { name: "TRACE", order: 3, description: "Trace the flow with deeper reads", requireEvidence: ["tool:filesystem_read"], blockUntil: "READ" },
    ],
  },
  "masday-e2e": {
    steps: [
      { name: "SMOKE", order: 1, description: "Smoke test — navigate to frontend", requireEvidence: ["browserNavigate"], blockUntil: null },
      { name: "SNAPSHOT", order: 2, description: "Take snapshot", requireEvidence: ["browserSnapshot"], blockUntil: "SMOKE" },
      { name: "CONSOLE", order: 3, description: "Check console errors", requireEvidence: ["consoleMessages"], blockUntil: "SNAPSHOT" },
      { name: "RESPONSIVE", order: 4, description: "Responsive testing", requireEvidence: ["browserResize"], blockUntil: "CONSOLE" },
    ],
  },
};

// ── State Management ────────────────────────────────────────────────────────

function getStateFile(skillName) {
  if (!fs.existsSync(STATE_DIR)) {
    fs.mkdirSync(STATE_DIR, { recursive: true });
  }
  return path.join(STATE_DIR, `skill-${skillName}.json`);
}

function loadState(skillName) {
  const file = getStateFile(skillName);
  if (!fs.existsSync(file)) {
    return { skillName, currentStep: null, completedSteps: [], evidence: {}, updatedAt: Date.now() };
  }
  try {
    return JSON.parse(fs.readFileSync(file, "utf-8"));
  } catch {
    return { skillName, currentStep: null, completedSteps: [], evidence: {}, updatedAt: Date.now() };
  }
}

function saveState(state) {
  if (!fs.existsSync(STATE_DIR)) {
    fs.mkdirSync(STATE_DIR, { recursive: true });
  }
  state.updatedAt = Date.now();
  fs.writeFileSync(getStateFile(state.skillName), JSON.stringify(state, null, 2));
}

// ── Skill Detection ─────────────────────────────────────────────────────────

function detectActiveSkill(toolName, toolInput) {
  const input = typeof toolInput === "string" ? toolInput : JSON.stringify(toolInput || {});

  if (toolName === "Skill") {
    if (input.includes("masday-tdd")) return "masday-tdd";
    if (input.includes("masday-workflow-new")) return "masday-workflow-new";
    if (input.includes("masday-workflow-plan")) return "masday-workflow-plan";
    if (input.includes("masday-workflow-run")) return "masday-workflow-run";
    if (input.includes("masday-workflow-init")) return "masday-workflow-init";
    if (input.includes("masday-workflow-fix")) return "masday-workflow-fix";
    if (input.includes("masday-workflow-verify")) return "masday-workflow-verify";
    if (input.includes("masday-workflow-audit")) return "masday-workflow-audit";
    if (input.includes("masday-workflow-add-task")) return "masday-workflow-add-task";
    if (input.includes("masday-workflow-discipline")) return "masday-workflow-discipline";
    if (input.includes("masday-workflow-continue")) return "masday-workflow-continue";
    if (input.includes("masday-workflow-next")) return "masday-workflow-next";
    if (input.includes("masday-research")) return "masday-research";
    if (input.includes("masday-web-research")) return "masday-web-research";
    if (input.includes("masday-code-analyze")) return "masday-code-analyze";
    if (input.includes("masday-context-retrieval")) return "masday-context-retrieval";
    if (input.includes("masday-memory-search")) return "masday-memory-search";
    if (input.includes("masday-create-agent")) return "masday-create-agent";
    if (input.includes("masday-create-skill")) return "masday-create-skill";
    if (input.includes("masday-create-command")) return "masday-create-command";
    if (input.includes("masday-create-mcp-skill")) return "masday-create-mcp-skill";
    if (input.includes("masday-parallel-execution")) return "masday-parallel-execution";
    if (input.includes("masday-parallel-research")) return "masday-parallel-research";
    if (input.includes("masday-deploy-check")) return "masday-deploy-check";
    if (input.includes("masday-docker-ops")) return "masday-docker-ops";
    if (input.includes("masday-cicd-ops")) return "masday-cicd-ops";
    if (input.includes("masday-git-workflow")) return "masday-git-workflow";
    if (input.includes("masday-github-flow")) return "masday-github-flow";
    if (input.includes("masday-github-pr")) return "masday-github-pr";
    if (input.includes("masday-autopilot")) return "masday-autopilot";
    if (input.includes("masday-sequential-thinking")) return "masday-sequential-thinking";
    if (input.includes("masday-e2e")) return "masday-e2e";
  }

  if (toolName.includes("tests_run")) return "masday-tdd";
  if (toolName.includes("workflow_create") && !toolName.includes("Plan")) return "masday-workflow-new";
  if (toolName.includes("workflow_createPlan")) return "masday-workflow-plan";

  return null;
}

// ── Evidence Collection ─────────────────────────────────────────────────────

function collectEvidence(toolName, toolInput) {
  const evidence = {};

  // File evidence from Write tool
  if (toolName === "Write" || toolName === "write") {
    const filePath = toolInput?.file_path || toolInput?.path || "";
    if (filePath.includes(".test.") || filePath.includes(".spec.")) {
      evidence.testFileWritten = filePath;
    }
    if (filePath.includes(".ts") && !filePath.includes(".test.") && !filePath.includes(".spec.")) {
      evidence.sourceFileWritten = filePath;
    }
  }

  // Edit evidence
  if (toolName === "Edit" || toolName === "edit") {
    const filePath = toolInput?.file_path || toolInput?.path || "";
    if (filePath.includes(".test.") || filePath.includes(".spec.")) {
      evidence.testFileEdited = filePath;
    } else {
      evidence.sourceFileEdited = filePath;
    }
  }

  // Test execution evidence
  if (toolName.includes("tests_run")) {
    evidence.testsRun = true;
  }

  // Bash test command evidence
  if (toolName === "Bash" || toolName === "bash") {
    const cmd = toolInput?.command || "";
    if (cmd.includes("vitest") || cmd.includes("pnpm test") || cmd.includes("npx vitest")) {
      evidence.testsRun = true;
    }
    if (cmd.includes("--coverage") || cmd.includes("coverage")) {
      evidence.coverageChecked = true;
    }
  }

  // WebSearch evidence (for web-research skill)
  if (toolName === "WebSearch" || toolName === "webSearch" || toolName.includes("web_search")) {
    evidence.webSearch = true;
  }

  // Browser tool evidence (for e2e skill)
  if (toolName.includes("browser_navigate") || toolName.includes("browser_navigate")) {
    evidence.browserNavigate = true;
  }
  if (toolName.includes("browser_snapshot") || toolName.includes("browser_snapshot") || toolName.includes("take_snapshot")) {
    evidence.browserSnapshot = true;
  }
  if (toolName.includes("console_messages") || toolName.includes("list_console")) {
    evidence.consoleMessages = true;
  }
  if (toolName.includes("browser_resize") || toolName.includes("resize_page") || toolName.includes("browser_resize")) {
    evidence.browserResize = true;
  }

  // MCP tool evidence — all patterns referenced by SKILL_STEPS
  const mcpToolPatterns = [
    "capability_system_readiness",
    "memory_search",
    "memory_recall_recent",
    "memory_recall_documents",
    "memory_recall_document_by_type",
    "memory_recall_by_task",
    "memory_store",
    "memory_store_research",
    "memory_update",
    "memory_delete",
    "memory_stats",
    "semantic-search_code_search",
    "semantic-search_search_hybrid_context_pack",
    "semantic-search_search_context_fingerprint",
    "workflow_create",
    "workflow_createPlan",
    "workflow_addTask",
    "workflow_get",
    "workflow_getStatus",
    "workflow_listTasks",
    "workflow_getPlan",
    "workflow_getActive",
    "workflow_getCurrentTask",
    "workflow_execute",
    "workflow_startTask",
    "workflow_saveProgress",
    "workflow_completeTask",
    "workflow_createParallelBranches",
    "workflow_completeParallelBranch",
    "workflow_listParallelBranches",
    "capability_list_agents",
    "capability_list_skills",
    "capability_list_templates",
    "capability_match_agent",
    "capability_create_agent",
    "capability_create_skill",
    "capability_workflow_audit",
    "capability_scaffold_feature",
    "policy_check_session_readiness",
    "policy_validate_execution",
    "policy_validate_completion",
    "policy_validate_parallel_completion",
    "policy_detect_scope_drift",
    "policy_require_context_refresh",
    "review_submit",
    "review_get_latest",
    "filesystem_list",
    "filesystem_read",
    "filesystem_write",
    "filesystem_stat",
    "filesystem_delete",
    "git_status",
    "git_diff",
    "git_commit",
    "npm_install",
    "npm_run",
    "docker_ps",
    "docker_build",
    "docker_run",
    "cicd_pipeline_status",
    "cicd_runs_view",
    "cicd_pipeline_trigger",
    "github_pr_list",
    "github_pr_create",
    "github_issue_list",
    "local_init",
    "local_sync",
    "local_save_artifact",
    "session_init_context",
    "session_get_state",
    "session_patch_state",
  ];

  for (const pattern of mcpToolPatterns) {
    if (toolName.includes(pattern)) {
      evidence[`tool:${pattern}`] = true;
    }
  }

  return evidence;
}

// ── Step Validation ──────────────────────────────────────────────────────────

function getStepDefinition(skillName, stepName) {
  const skill = SKILL_STEPS[skillName];
  if (!skill) return null;
  return skill.steps.find((s) => s.name === stepName);
}

function isStepComplete(skillName, stepName, state) {
  const step = getStepDefinition(skillName, stepName);
  if (!step) return false;

  return step.requireEvidence.every((req) => {
    if (req.startsWith("tool:")) return state.evidence[req] === true;
    return !!state.evidence[req];
  });
}

function getBlockingStep(skillName, targetStep) {
  const skill = SKILL_STEPS[skillName];
  if (!skill) return null;

  const target = skill.steps.find((s) => s.name === targetStep);
  if (!target || !target.blockUntil) return null;

  const state = loadState(skillName);
  let current = target.blockUntil;
  const visited = new Set();

  while (current && !visited.has(current)) {
    visited.add(current);
    if (!isStepComplete(skillName, current, state)) {
      return current;
    }
    const stepDef = getStepDefinition(skillName, current);
    current = stepDef?.blockUntil || null;
  }

  return null;
}

// ── Main Hook Logic ─────────────────────────────────────────────────────────

function readJsonFromStdin() {
  return new Promise((resolve, reject) => {
    const chunks = [];
    process.stdin.on("data", (chunk) => chunks.push(chunk));
    process.stdin.on("end", () => {
      if (chunks.length === 0) {
        resolve({});
        return;
      }
      const raw = Buffer.concat(chunks).toString("utf8").trim();
      try {
        resolve(raw ? JSON.parse(raw) : {});
      } catch (error) {
        reject(error);
      }
    });
    process.stdin.on("error", reject);
  });
}

async function main() {
  const input = await readJsonFromStdin();
  const toolName = input.tool_name || "";
  const toolInput = input.tool_input || {};

  // 1. Detect if a skill is being activated
  const detectedSkill = detectActiveSkill(toolName, toolInput);

  // 2. Check all active skills for state
  const activeSkills = Object.keys(SKILL_STEPS).filter((name) => {
    const stateFile = getStateFile(name);
    return fs.existsSync(stateFile);
  });

  // If a new skill is detected, initialize its state
  if (detectedSkill && !activeSkills.includes(detectedSkill)) {
    const state = loadState(detectedSkill);
    state.currentStep = SKILL_STEPS[detectedSkill].steps[0]?.name || null;
    saveState(state);
    activeSkills.push(detectedSkill);
  }

  // 3. Collect evidence from this tool call for all active skills
  for (const skillName of activeSkills) {
    const state = loadState(skillName);
    const newEvidence = collectEvidence(toolName, toolInput);

    if (Object.keys(newEvidence).length > 0) {
      Object.assign(state.evidence, newEvidence);

      // Check if current step is now complete, advance to next
      const skill = SKILL_STEPS[skillName];
      if (skill && state.currentStep) {
        if (isStepComplete(skillName, state.currentStep, state)) {
          if (!state.completedSteps.includes(state.currentStep)) {
            state.completedSteps.push(state.currentStep);
          }
          const currentIdx = skill.steps.findIndex((s) => s.name === state.currentStep);
          for (let i = currentIdx + 1; i < skill.steps.length; i++) {
            if (!state.completedSteps.includes(skill.steps[i].name)) {
              state.currentStep = skill.steps[i].name;
              break;
            }
          }
        }
      }

      saveState(state);
    }
  }

  // 4. Validate step transitions — BLOCK violations
  for (const skillName of activeSkills) {
    const skill = SKILL_STEPS[skillName];
    if (!skill) continue;

    const state = loadState(skillName);
    if (!state.currentStep) continue;

    const currentStepDef = getStepDefinition(skillName, state.currentStep);
    if (!currentStepDef) continue;

    // TDD RED-phase guard: block source code writes before tests
    if (skillName === "masday-tdd" && state.currentStep === "RED") {
      if (toolName === "Write" || toolName === "Edit") {
        const filePath = toolInput?.file_path || toolInput?.path || "";
        if (!filePath.includes(".test.") && !filePath.includes(".spec.") && filePath.endsWith(".ts")) {
          process.stdout.write(
            JSON.stringify({
              decision: "block",
              reason:
                `BLOCKED by masday-tdd RED phase guard. ` +
                `Writing source code before tests violates TDD.\n` +
                `Current step: RED (Write failing tests first)\n` +
                `Required: Write a .test.ts or .spec.ts file first.`,
            })
          );
          return;
        }
      }
    }

    // workflow_execute guard: always validate all pre-execution steps complete
    if (toolName.includes("workflow_execute")) {
      const executeStep = skill.steps.find((s) => s.isGate);
      if (executeStep) {
        const blockingStep = getBlockingStep(skillName, executeStep.name);
        if (blockingStep) {
          const blockingDef = getStepDefinition(skillName, blockingStep);
          process.stdout.write(
            JSON.stringify({
              decision: "block",
              reason:
                `BLOCKED by ${skillName} GATE. Cannot execute workflow.\n` +
                `Step "${blockingStep}" (${blockingDef?.description}) must be complete first.\n` +
                `Missing evidence: ${blockingDef?.requireEvidence.join(", ")}`,
            })
          );
          return;
        }
      }
    }

    // Gate enforcement (generic)
    if (currentStepDef.isGate) {
      const blockingStep = getBlockingStep(skillName, state.currentStep);
      if (blockingStep) {
        const blockingDef = getStepDefinition(skillName, blockingStep);
        process.stdout.write(
          JSON.stringify({
            decision: "block",
            reason:
              `BLOCKED by ${skillName} GATE. Step "${state.currentStep}" requires step ` +
              `"${blockingStep}" (${blockingDef?.description}) to be complete first.\n\n` +
              `Missing evidence: ${blockingDef?.requireEvidence.join(", ")}`,
          })
        );
        return;
      }
    }
  }

  // 5. Warnings for incomplete steps at critical transitions
  for (const skillName of activeSkills) {
    const state = loadState(skillName);
    if (!state.currentStep) continue;

    const step = getStepDefinition(skillName, state.currentStep);
    if (!step) continue;

    const missingEvidence = step.requireEvidence.filter((req) => {
      if (req.startsWith("tool:")) return !state.evidence[req];
      return !state.evidence[req];
    });

    if (
      missingEvidence.length > 0 &&
      (toolName.includes("workflow_execute") || toolName.includes("workflow_completeTask"))
    ) {
      process.stdout.write(
        JSON.stringify({
          systemMessage:
            `[${skillName}] Step "${state.currentStep}" incomplete. ` +
            `Missing: ${missingEvidence.join(", ")}. ` +
            `Step: ${step.description}`,
        })
      );
      return;
    }
  }

  process.stdout.write(JSON.stringify({}));
}

// ── Cleanup ─────────────────────────────────────────────────────────────────

function clearAllStates() {
  if (fs.existsSync(STATE_DIR)) {
    const files = fs.readdirSync(STATE_DIR);
    for (const f of files) {
      fs.unlinkSync(path.join(STATE_DIR, f));
    }
  }
}

if (require.main === module) {
  main().catch((error) => {
    process.stdout.write(
      JSON.stringify({
        systemMessage: "skill-step-guard hook error: " + (error instanceof Error ? error.message : String(error)),
      })
    );
    process.exitCode = 0;
  });
}

module.exports = { SKILL_STEPS, loadState, saveState, clearAllStates };
