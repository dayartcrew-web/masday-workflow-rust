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
  "masday-tdd": {
    steps: [
      {
        name: "RED",
        order: 1,
        description: "Write failing tests",
        requireEvidence: ["testFileWritten"],
        blockUntil: null,
      },
      {
        name: "RED_VERIFY",
        order: 2,
        description: "Verify tests fail (RED confirmation)",
        requireEvidence: ["testsRun"],
        blockUntil: "RED",
      },
      {
        name: "GREEN",
        order: 3,
        description: "Implement minimum code to pass tests",
        requireEvidence: ["sourceFileEdited"],
        blockUntil: "RED_VERIFY",
      },
      {
        name: "GREEN_VERIFY",
        order: 4,
        description: "Verify tests pass (GREEN confirmation)",
        requireEvidence: ["testsPass"],
        blockUntil: "GREEN",
      },
      {
        name: "REFACTOR",
        order: 5,
        description: "Clean up code while tests stay green",
        requireEvidence: ["sourceFileEdited"],
        blockUntil: "GREEN_VERIFY",
      },
      {
        name: "COVERAGE",
        order: 6,
        description: "Verify 80%+ coverage",
        requireEvidence: ["coverageChecked"],
        blockUntil: "REFACTOR",
      },
    ],
  },
  "masday-workflow-new": {
    steps: [
      {
        name: "READINESS",
        order: 1,
        description: "Check system readiness",
        requireEvidence: ["tool:capability_system_readiness"],
        blockUntil: null,
      },
      {
        name: "CONTEXT",
        order: 2,
        description: "Search context (memory + code)",
        requireEvidence: ["tool:memory_search", "tool:memory_recall_recent", "tool:semantic-search_code_search"],
        blockUntil: "READINESS",
      },
      {
        name: "CREATE",
        order: 3,
        description: "Create workflow",
        requireEvidence: ["tool:workflow_create"],
        blockUntil: "CONTEXT",
      },
      {
        name: "CONTEXT_PACK",
        order: 4,
        description: "Build hybrid context pack",
        requireEvidence: ["tool:semantic-search_search_hybrid_context_pack", "tool:memory_recall_documents"],
        blockUntil: "CREATE",
      },
      {
        name: "AGENT_MATCH",
        order: 5,
        description: "Match best agent for tasks",
        requireEvidence: ["tool:capability_list_agents", "tool:capability_match_agent"],
        blockUntil: "CONTEXT_PACK",
      },
      {
        name: "SKILL_VERIFY",
        order: 6,
        description: "Verify skill exists",
        requireEvidence: ["tool:capability_list_skills"],
        blockUntil: "AGENT_MATCH",
      },
      {
        name: "EXECUTE",
        order: 7,
        description: "Execute workflow (GATE)",
        requireEvidence: [],
        blockUntil: "SKILL_VERIFY",
        isGate: true,
      },
      {
        name: "STORE",
        order: 8,
        description: "Store artifacts in memory",
        requireEvidence: ["tool:memory_store"],
        blockUntil: "EXECUTE",
      },
    ],
  },
  "masday-workflow-plan": {
    steps: [
      {
        name: "ANALYZE",
        order: 1,
        description: "Analyze codebase for planning",
        requireEvidence: ["tool:semantic-search_code_search"],
        blockUntil: null,
      },
      {
        name: "MEMORY",
        order: 2,
        description: "Search memory for past patterns",
        requireEvidence: ["tool:memory_search"],
        blockUntil: "ANALYZE",
      },
      {
        name: "PLAN",
        order: 3,
        description: "Create execution plan",
        requireEvidence: ["tool:workflow_createPlan"],
        blockUntil: "MEMORY",
      },
      {
        name: "TASKS",
        order: 4,
        description: "Add tasks to plan",
        requireEvidence: ["tool:workflow_addTask"],
        blockUntil: "PLAN",
      },
    ],
  },
  "masday-research": {
    steps: [
      {
        name: "SEARCH",
        order: 1,
        description: "Search for existing knowledge",
        requireEvidence: ["tool:memory_search"],
        blockUntil: null,
      },
      {
        name: "CODEBASE",
        order: 2,
        description: "Search codebase for related code",
        requireEvidence: ["tool:semantic-search_code_search"],
        blockUntil: "SEARCH",
      },
      {
        name: "STORE",
        order: 3,
        description: "Store research findings",
        requireEvidence: ["tool:memory_store"],
        blockUntil: "CODEBASE",
      },
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
    if (input.includes("masday-research")) return "masday-research";
  }

  if (toolName.includes("tests_run")) return "masday-tdd";
  if (toolName.includes("workflow_create") && !toolName.includes("Plan")) return "masday-workflow-new";
  if (toolName.includes("workflow_createPlan")) return "masday-workflow-plan";

  return null;
}

// ── Evidence Collection ─────────────────────────────────────────────────────

function collectEvidence(toolName, toolInput, state) {
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

  // MCP tool evidence
  const mcpToolPatterns = [
    "capability_system_readiness",
    "memory_search",
    "memory_recall_recent",
    "semantic-search_code_search",
    "workflow_create",
    "semantic-search_search_hybrid_context_pack",
    "memory_recall_documents",
    "capability_list_agents",
    "capability_match_agent",
    "capability_list_skills",
    "memory_store",
    "workflow_createPlan",
    "workflow_addTask",
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
    const newEvidence = collectEvidence(toolName, toolInput, state);

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
