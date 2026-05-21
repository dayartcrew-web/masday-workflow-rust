# PRD: Claude Code Ecosystem Compatibility Analysis

**Date:** 2026-05-17
**Status:** Research Complete
**Purpose:** Deep analysis of Claude Code's Skills, Agents, Hooks, and Plugins ecosystem vs masday-workflow-reborn current implementation

---

## Table of Contents

1. [Executive Summary](#executive-summary)
2. [Branch 1: PDF Skills Guide vs Official Sub-Agents Docs](#branch-1-pdf-skills-guide-vs-official-sub-agents-docs)
3. [Branch 2: .claude Directory Comparison](#branch-2-claude-directory-comparison)
4. [Branch 3: Claude Code Sub-Agents Documentation](#branch-3-claude-code-sub-agents-documentation)
5. [Branch 4: Claude Code Hooks Guide](#branch-4-claude-code-hooks-guide)
6. [Branch 5: Claude Code Plugins Documentation](#branch-5-claude-code-plugins-documentation)
7. [Gap Analysis and Recommendations](#gap-analysis--recommendations)
8. [Migration Roadmap](#migration-roadmap)

---

## Executive Summary

This PRD documents findings from 5 parallel research branches analyzing Claude Code's extension ecosystem (Skills, Sub-Agents, Hooks, Plugins) and comparing two project configurations (masday-workflow-reborn vs msd-mcp).

### Key Findings

| Area | Current State | Gap | Priority |
|------|---------------|-----|----------|
| **Skills** | 11 skills in SKILL.md format | Missing manifest.json, no progressive disclosure, descriptions lack trigger phrases | HIGH |
| **Agents** | 5 generalist agents | Need 20+ specialist agents, missing frontmatter fields (model, tools, permissionMode) | HIGH |
| **Hooks** | 3 markdown advisory hooks | Need executable JavaScript hooks with settings.json configuration | CRITICAL |
| **Plugins** | Not implemented | Entire plugin infrastructure missing (.claude-plugin format) | MEDIUM |
| **Registry** | None | Missing registry.json for tracking all components | HIGH |

---

## Branch 1: PDF Skills Guide vs Official Sub-Agents Docs

### Comparison Matrix

| Aspect | PDF Skills Guide | Official Sub-Agents Docs |
|--------|-----------------|--------------------------|
| **Topic** | How to build Skills | How to build Sub-Agents |
| **Format** | SKILL.md + folder structure | Agent .md with YAML frontmatter |
| **Loading** | Progressive disclosure (3 levels) | Full context window per agent |
| **Context** | Runs in main conversation | Isolated context window |
| **Tool Access** | `allowed-tools` in frontmatter | `tools` allowlist + `disallowedTools` denylist |
| **Model Selection** | Not specified | `model` field (sonnet/opus/haiku) |
| **Memory** | Not specified | `memory` field (user/project/local) |
| **Nesting** | Can reference other skills | Cannot spawn other sub-agents |
| **Isolation** | None | `isolation: worktree` option |
| **Distribution** | Zip upload, GitHub | .claude/agents/, plugins, CLI flag |

### Key Insight: Skills vs Agents Are Complementary

- **Skills** = instruction sets loaded into the main conversation (lightweight, composable)
- **Sub-Agents** = isolated workers with own context, tools, and permissions (heavyweight, focused)
- **Best practice**: Skills provide knowledge; agents provide specialized execution capacity

### What masday-workflow-reborn Is Missing (from PDF)

1. **Progressive Disclosure**: Skills should use 3-level loading:
   - Level 1: YAML frontmatter always in system prompt
   - Level 2: SKILL.md body loaded on-demand
   - Level 3: references/ scripts/ assets/ loaded as needed

2. **Description Quality**: Current descriptions are too vague. Must include:
   - WHAT the skill does
   - WHEN to use it (trigger phrases)
   - Key capabilities

3. **Skill Categories**: Should organize into:
   - Document and Asset Creation
   - Workflow Automation
   - MCP Enhancement

4. **Workflow Patterns**: Missing implementation of:
   - Sequential Workflow Orchestration
   - Multi-MCP Coordination
   - Iterative Refinement
   - Context-Aware Tool Selection
   - Domain-Specific Intelligence

5. **Testing**: No skill testing framework (triggering tests, functional tests, performance comparison)

---

## Branch 2: .claude Directory Comparison

### Structural Overview

| Aspect | masday-workflow-reborn (A) | msd-mcp (B) |
|--------|-------------------------|-------------|
| **Agents** | 5 generalist | 35 specialist |
| **Skills** | 11 workflow-oriented | 9 specialized (with design library) |
| **Commands** | 16 | 16 |
| **Hooks** | 3 markdown (advisory) | 9 JS + 1 MJS (enforced) |
| **Settings** | None | settings.json with hook config |
| **Registry** | None | registry.json (master index) |
| **Manifests** | None | manifest.json per skill |
| **UI Library** | None | 7 images + design tokens |

### Agent Architecture Comparison

**Project A: Generalist Model (5 agents)**
```
orchestrator  -> coordinates all workflows
backend       -> all backend work
frontend      -> all frontend work
qa            -> all testing
researcher    -> all research
```

**Project B: Specialist Model (35 agents)**
```
msd-orchestrator     -> 6-phase lifecycle coordinator
msd-planner          -> task decomposition and planning
msd-executor         -> code implementation
msd-reviewer         -> quality gate enforcement
msd-verifier         -> final validation
msd-synthesizer      -> parallel branch merging
msd-researcher       -> external information gathering
msd-analyzer         -> codebase pattern analysis
msd-debugger         -> root cause investigation
msd-frontend         -> UI implementation
msd-backend-tester   -> API and database testing
msd-e2e-tester       -> end-to-end validation
msd-security         -> vulnerability scanning
msd-pen-tester       -> penetration testing
msd-performance      -> optimization analysis
msd-refactor         -> code quality improvement
msd-linter           -> code style enforcement
msd-docs             -> documentation generation
msd-doc-verifier     -> documentation accuracy
msd-git-master       -> version control operations
msd-ci-cd-pipeline   -> deployment automation
msd-codebase-mapper  -> architecture documentation
msd-intel-updater    -> intelligence file management
msd-context-manager  -> state preservation
msd-task-decomp      -> complex goal breakdown
msd-roadmapper       -> project planning
msd-assumptions      -> assumption analysis
msd-advisor          -> decision research
msd-ideation         -> feature brainstorming
msd-seo-auditor      -> SEO optimization
msd-config           -> configuration management
msd-database-arch    -> schema design
msd-integrator       -> cross-module validation
msd-nyquist-auditor  -> test coverage gaps
msd-ui-ux-expert     -> design system expertise
```

### Critical Differences

| Aspect | Project A | Project B | Recommendation |
|--------|-----------|-----------|----------------|
| **Enforcement** | Advisory (markdown) | Enforced (JavaScript hooks) | Adopt B's enforced model |
| **MCP Tool Names** | Short names (`workflow_create`) | Fully qualified (`mcp__workflow-orchestrator__workflow_create`) | Use fully qualified names |
| **Hook Format** | .md instruction files | .js executable scripts | Convert to executable hooks |
| **Registry** | None | registry.json tracking all components | Add registry.json |
| **Settings** | None | settings.json with hook matchers | Add settings.json |
| **UI/UX** | Single frontend agent | Dedicated agent + 4 skills + design library | Expand UI coverage |

---

## Branch 3: Claude Code Sub-Agents Documentation

### Agent File Format

```markdown
---
name: my-agent
description: When to delegate to this agent
tools: Read, Grep, Glob, Bash
model: sonnet
permissionMode: default
maxTurns: 50
skills: [skill-1, skill-2]
mcpServers: {}
hooks: {}
memory: project
background: false
effort: high
isolation: worktree
color: blue
---
System prompt instructions here...
```

### All Frontmatter Fields

| Field | Required | Description |
|-------|----------|-------------|
| `name` | Yes | Unique identifier (lowercase, hyphens) |
| `description` | Yes | Delegation trigger description |
| `tools` | No | Tool allowlist |
| `disallowedTools` | No | Tool denylist |
| `model` | No | sonnet, opus, haiku, inherit |
| `permissionMode` | No | default, acceptEdits, auto, dontAsk, bypassPermissions, plan |
| `maxTurns` | No | Maximum agentic turns |
| `skills` | No | Skills to preload at startup |
| `mcpServers` | No | MCP servers available to agent |
| `hooks` | No | Lifecycle hooks scoped to agent |
| `memory` | No | user, project, or local persistent memory |
| `background` | No | Always run as background task |
| `effort` | No | low, medium, high, xhigh, max |
| `isolation` | No | worktree for isolated git copy |
| `color` | No | Display color |
| `initialPrompt` | No | Auto-submitted first turn |

### Dispatch Patterns

1. **Automatic**: Claude reads `description` and delegates
2. **Natural language**: "Use the code-reviewer subagent"
3. **@-mention**: `@"code-reviewer (agent)" look at auth changes`
4. **Session-wide**: `claude --agent code-reviewer`

### Scope Priority (highest to lowest)

1. Managed settings (org-wide)
2. CLI `--agents` flag (session-only)
3. `.claude/agents/` (project-level, committable)
4. `~/.claude/agents/` (user-level)
5. Plugin `agents/` directory

### What masday-workflow-reborn Agents Are Missing

- `model` field for cost optimization (Haiku for read-only, Sonnet for implementation)
- `tools` allowlist for security
- `permissionMode` for controlled execution
- `memory` field for cross-session learning
- `maxTurns` to prevent runaway agents
- `isolation: worktree` for safe parallel execution

---

## Branch 4: Claude Code Hooks Guide

### Hook Handler Types (5 types)

| Type | Description | Use Case |
|------|-------------|----------|
| `command` | Shell command (JSON on stdin) | File validation, formatting |
| `http` | POST to URL | External integrations |
| `mcp_tool` | Call MCP server tool | Dynamic policy enforcement |
| `prompt` | LLM yes/no decision | Complex conditional logic |
| `agent` | Sub-agent verification (multi-turn) | Deep condition checking |

### Lifecycle Events (30 events)

Key events for masday-workflow-reborn:

| Event | Purpose | Implementation Priority |
|-------|---------|------------------------|
| `SessionStart` | Load context, validate environment | HIGH |
| `PreToolUse` | Block operations, validate inputs | CRITICAL |
| `PostToolUse` | Auto-format, track changes, trigger builds | HIGH |
| `Stop` | Verify completion before session ends | HIGH |
| `SubagentStart` | Inject context into sub-agents | MEDIUM |
| `SubagentStop` | Validate sub-agent output | MEDIUM |
| `Notification` | Desktop/push notifications | LOW |
| `ConfigChange` | Audit configuration changes | LOW |

### Hook Configuration Format

```json
{
  "hooks": {
    "PreToolUse": [
      {
        "matcher": "Edit|Write|MultiEdit",
        "hooks": [
          {
            "type": "command",
            "command": "node .claude/hooks/msd-pre-tool-use.js",
            "timeout": 30
          }
        ]
      }
    ],
    "PostToolUse": [
      {
        "matcher": "Edit|Write|MultiEdit",
        "hooks": [
          {
            "type": "command",
            "command": "node .claude/hooks/msd-post-tool-use.js",
            "timeout": 30
          }
        ]
      }
    ],
    "Stop": [
      {
        "matcher": "",
        "hooks": [
          {
            "type": "command",
            "command": "node .claude/hooks/msd-on-stop.js",
            "timeout": 60
          }
        ]
      }
    ]
  }
}
```

### Hook Input/Output Protocol

**Input (JSON on stdin):**
```json
{
  "session_id": "...",
  "tool_name": "Edit",
  "tool_input": { "file_path": "/path/to/file", "old_string": "...", "new_string": "..." },
  "hook_event_name": "PreToolUse"
}
```

**Output (JSON on stdout):**
```json
{
  "hookSpecificOutput": {
    "permissionDecision": "deny",
    "permissionDecisionReason": "No workflow context loaded"
  }
}
```

**Exit Codes:**
- `0` = Success (stdout parsed as JSON)
- `2` = Blocking error (stderr shown to user)
- Other = Non-blocking error (execution continues)

### What masday-workflow-reborn Hooks Must Become

Current (advisory markdown):
```
hooks/pre-workflow-execute.md  -> "Check workflow state before executing"
hooks/post-task-complete.md    -> "Verify task completion"
hooks/pre-commit.md            -> "Run quality checks"
```

Target (enforced JavaScript):
```
hooks/msd-pre-tool-use.js     -> Block edits without loaded context
hooks/msd-tdd-guard.js        -> Block source edits without test file
hooks/msd-verify-build.js     -> Remind to build/test after edits
hooks/msd-post-tool-use.js    -> Track file changes
hooks/msd-post-agent.js       -> Notify after agent completion
hooks/msd-on-stop.js          -> Verify session completion
hooks/msd-on-error.js         -> Handle errors
hooks/msd-pre-command.js      -> Validate before commands
hooks/run-hook.mjs            -> Universal hook runner
```

---

## Branch 5: Claude Code Plugins Documentation

### Plugin Structure

```
my-plugin/
  .claude-plugin/
    plugin.json              # Required manifest
    settings.json            # Optional default settings
    skills/                  # Skills
    commands/                # Slash commands
    agents/                  # Agent definitions
    hooks/
      hooks.json             # Hook definitions
    .mcp.json                # MCP server config
    .lsp.json                # LSP server config
    monitors/
      monitors.json          # Background monitors
    themes/                  # Visual themes
    bin/                     # Executable scripts
```

### Plugin Manifest (plugin.json)

```json
{
  "name": "my-plugin",
  "description": "Plugin description",
  "version": "1.0.0",
  "author": { "name": "Author", "email": "email@example.com" },
  "license": "MIT",
  "repository": "https://github_com/user/plugin",
  "dependencies": { "other-plugin": "^1.0.0" },
  "userConfig": [
    {
      "key": "apiKey",
      "label": "API Key",
      "type": "string",
      "required": true
    }
  ]
}
```

### Plugin vs Standalone .claude/

| Aspect | `.claude/` (standalone) | `.claude-plugin/` (plugin) |
|--------|------------------------|---------------------------|
| Scope | Single project | Shareable across projects |
| Distribution | Local only | Marketplace, Git, tarball |
| Namespacing | Not namespaced | `/plugin-name:skill-name` |
| Versioning | None | Semver in manifest |
| Dependencies | Not supported | Declared in manifest |
| MCP Servers | In .claude.json | In .mcp.json |
| User Config | Not prompted | Prompted at enable time |
| Caching | None | `~/.claude/plugins/cache/` |

### Plugin CLI Commands

```bash
claude plugin add <source>       # Install
claude plugin remove <name>      # Uninstall
claude plugin list               # List installed
claude plugin enable <name>      # Enable
claude plugin disable <name>     # Disable
claude plugin update <name>      # Update
claude plugin logs <name>        # View logs
```

### MCP Server Integration in Plugins

```json
{
  "mcpServers": {
    "plugin-database": {
      "command": "${CLAUDE_PLUGIN_ROOT}/servers/db-server",
      "args": ["--config", "${CLAUDE_PLUGIN_ROOT}/config.json"],
      "env": {
        "DB_PATH": "${CLAUDE_PLUGIN_ROOT}/data"
      }
    }
  }
}
```

### masday-workflow-reborn as a Plugin

The project could be packaged as a Claude Code plugin to enable:
- One-command installation: `claude plugin add https://github_com/user/masday-workflow-reborn`
- Automatic MCP server startup
- Namespaced commands: `/masday:workflow-plan`
- Version management
- User configuration prompts for API keys

---

## Gap Analysis and Recommendations

### Priority Matrix

| # | Gap | Current State | Target State | Priority | Effort |
|---|-----|---------------|--------------|----------|--------|
| 1 | Hook enforcement | Advisory markdown | Executable JavaScript with settings.json | CRITICAL | 3 days |
| 2 | Agent specialization | 5 generalist | 20+ specialist with proper frontmatter | HIGH | 5 days |
| 3 | Skill descriptions | Vague | WHAT + WHEN + capabilities | HIGH | 1 day |
| 4 | Registry system | None | registry.json tracking all components | HIGH | 2 days |
| 5 | settings.json | None | Hook configuration with matchers | HIGH | 1 day |
| 6 | Agent frontmatter | Minimal | Full fields (model, tools, memory, etc.) | HIGH | 2 days |
| 7 | Progressive disclosure | None | 3-level skill loading with references/ | MEDIUM | 2 days |
| 8 | Plugin packaging | None | .claude-plugin format with manifest | MEDIUM | 3 days |
| 9 | MCP tool names | Short names | Fully qualified names | MEDIUM | 1 day |
| 10 | Skill testing | None | Triggering + functional + performance tests | LOW | 2 days |

### Recommended Architecture

```
masday-workflow-reborn/
  .claude/
    settings.json              # Hook configuration
    registry.json              # Master component index
    agents/
      orchestrator.md          # Full frontmatter with model, tools, memory
      planner.md
      executor.md
      reviewer.md
      verifier.md
      synthesizer.md
      researcher.md
      analyzer.md
      debugger.md
      security.md
      ... (20+ specialists)
    skills/
      workflow-plan/
        SKILL.md               # Progressive disclosure format
        manifest.json           # Skill metadata
        references/             # Additional context (loaded on demand)
      workflow-run/
        SKILL.md
        manifest.json
      workflow-verify/
        SKILL.md
        manifest.json
      ... (all skills with proper format)
    hooks/
      run-hook.mjs             # Universal hook runner
      pre-tool-use.js          # Blocks edits without context
      tdd-guard.js             # Blocks source edits without tests
      post-tool-use.js         # Tracks changes, reminds to build
      on-stop.js               # Session completion verification
    commands/
      ... (16 commands, unchanged)
```

---

## Migration Roadmap

### Phase 1: Foundation (Week 1)

1. **Create settings.json** with hook configuration
2. **Convert hooks** from markdown to executable JavaScript
3. **Create registry.json** tracking all components
4. **Add manifest.json** to each skill directory

### Phase 2: Agent Overhaul (Week 2)

1. **Decompose 5 generalists** into 20+ specialist agents
2. **Add full frontmatter** (model, tools, permissionMode, memory, etc.)
3. **Update descriptions** with proper trigger phrases
4. **Test delegation** with various prompt patterns

### Phase 3: Skill Enhancement (Week 3)

1. **Rewrite skill descriptions** with WHAT + WHEN + capabilities
2. **Add references/** directories for progressive disclosure
3. **Ensure SKILL.md under 5,000 words** each
4. **Add skill testing** (triggering, functional, performance)

### Phase 4: Plugin Packaging (Week 4)

1. **Create .claude-plugin/ structure** alongside .claude/
2. **Write plugin.json manifest** with full metadata
3. **Create .mcp.json** for MCP server configuration
4. **Test installation** via `claude plugin add --local`
5. **Document installation process** for users

### Phase 5: Distribution (Week 5)

1. **Publish to Claude Code marketplace** (or Git distribution)
2. **Write comprehensive README** with installation instructions
3. **Create example workflows** demonstrating all capabilities
4. **Set up CI/CD** for plugin validation

---

## References

- PDF: "The Complete Guide to Building Skill for Claude" (33 pages)
- Official docs: https://code.claude.com/docs/en/sub-agents.md
- Official docs: https://code.claude.com/docs/en/hooks-guide.md
- Official docs: https://code.claude.com/docs/en/plugins.md
- Project A: `masday-workflow-reborn/.claude/`
- Project B: `msd-mcp/.claude/`
