---
name: masday-create-command
description: >
  Create a new slash command for Masday Workflow. Designs the command name, arguments,
  tool requirements, and invocation behavior. Saves to project commands directory.
  Use when the user says "create command", "new command", "add slash command", or "command definition".
allowed-tools:
  - filesystem.list
  - filesystem.write
---

# Masday Create Command

Create a new Masday slash command.

## Steps

1. **Check existing commands**
   - Call `filesystem.list` on `.claude/commands/` to see existing commands
   - Verify the proposed command name does not conflict

2. **Design the command**
   - Name: kebab-case (e.g., `masday-deploy`, `masday-status`)
   - Description: what the command does in under 250 characters
   - Arguments: required and optional argument hints
   - Auto/manual invocation: should it trigger automatically or only on slash?
   - Required tools: which MCP tools the command needs
   - Context mode: fork (for heavy tasks) or inline (for quick tasks)

3. **Generate command file** with YAML frontmatter:
   ```yaml
   ---
   name: <command-name>
   description: <what it does -- under 250 chars>
   argument-hint: [required-args] [optional-args]
   allowed-tools:
     - tool.name
   ---

   # <Command Name>

   <Instructions Claude follows when the command is invoked>

   ## Steps
   <numbered steps with tool call examples>

   ## Never
   <constraints>
   ```

4. **Save to project location**
   - Call `filesystem.write` to save:
     - `$ROOT/.claude/commands/<name>.md`

5. **Report**
   ```
   Command created: "/<name>"
   Arguments: <argument-hint>
   Tools: <list of allowed tools>
   Saved to: .claude/commands/<name>.md
   ```

## Never

- Never create a command without a clear argument specification
- Never use spaces in command names -- use kebab-case
- Never save commands outside the `.claude/commands/` directory
- Never create commands that duplicate existing skill functionality without good reason
