# Frontmatter Field Reference

## Skill Frontmatter Fields

| Field | Required | Description |
|-------|----------|-------------|
| `name` | Yes | kebab-case identifier, must match folder name |
| `description` | Yes | WHAT it does + WHEN to use it (trigger phrases). Under 1024 chars. |
| `allowed-tools` | No | Space-separated list of tools |
| `disable-model-invocation` | No | If true, skill content used directly |
| `context` | No | `fork` to run in sub-agent context |

## Agent Frontmatter Fields

| Field | Required | Description |
|-------|----------|-------------|
| `name` | Yes | kebab-case identifier |
| `description` | Yes | When to delegate to this agent |
| `model` | No | sonnet, opus, haiku, or inherit |
| `tools` | No | Tool allowlist (YAML list) |
| `disallowedTools` | No | Tool denylist |
| `permissionMode` | No | default, acceptEdits, auto, dontAsk, bypassPermissions, plan |
| `maxTurns` | No | Maximum agentic turns |
| `skills` | No | Skills to preload at startup |
| `mcpServers` | No | MCP servers available to agent |
| `memory` | No | user, project, or local persistent memory scope |
| `background` | No | Always run as background task |
| `effort` | No | low, medium, high, xhigh, max |
| `isolation` | No | worktree for isolated git copy |

## Command Frontmatter Fields

| Field | Required | Description |
|-------|----------|-------------|
| `name` | Yes | kebab-case command name |
| `description` | Yes | What the command does |
| `argument-hint` | No | Usage hint shown to user |
| `disable-model-invocation` | No | If true, command content used directly |
| `allowed-tools` | No | Space-separated tool list |
| `context` | No | `fork` for sub-agent context |

## Description Best Practice

Structure: `[What it does] + [When to use it] + [Key capabilities]`

Good: `Analyzes Figma design files and generates developer handoff documentation. Use when user uploads .fig files, asks for "design specs", or "design-to-code handoff".`

Bad: `Helps with projects.`
