# masday-stack-detector

## Overview

Global stack detection skill that automatically identifies the current technology stack and configures agents with appropriate defaults. This enables masday agents to work seamlessly across different technology stacks (Rust, TypeScript, Python, Go, etc.) without requiring manual configuration.

## Usage

```bash
/masday-stack-detector
```

## What It Does

1. **Automatically detects** the current technology stack
2. **Configures agents** with stack-appropriate defaults
3. **Maps file patterns** to the detected stack
4. **Sets build commands** specific to the detected language
5. **Adapts MCP tools** for the current environment
6. **Persists stack configuration** across sessions

## Supported Stacks

- **Rust** (Cargo.toml, .rs files)
- **TypeScript/Node.js** (package.json, .ts/.tsx files)
- **Python** (pyproject.toml, .py files)
- **Go** (go.mod, .go files)
- **Mixed/Hybrid** projects with multiple stacks

## Detection Priority

1. **Primary signals**:
   - Configuration files (Cargo.toml, package.json, etc.)
   - Source code file extensions (.rs, .ts, .py, .go)
   - Build tool availability (cargo, npm, pip, go)

2. **Secondary signals**:
   - Framework-specific patterns (Express, FastAPI, Axum, etc.)
   - Database patterns (SQLx, GORM, Prisma, etc.)
   - Testing framework patterns (Jest, Pytest, GoTest, etc.)

## Agent Adaptation

When a stack is detected, the following agents are automatically configured:

| Agent | Rust Adaptation | TypeScript Adaptation | Python Adaptation |
|-------|-----------------|----------------------|------------------|
| **masday-executor** | Rust code standards, unsafe checks | TS strict mode, no any | Python PEP8, type hints |
| **masday-qa** | cargo test, clippy | npm test, jest | pytest, coverage |
| **masday-tdd** | Rust test patterns | Jest/Vitest patterns | Pytest patterns |
| **masday-config** | Cargo.toml, rustfmt | tsconfig.json | pyproject.toml |
| **masday-integrator** | Cargo workspace packages | npm workspaces | Python packages |

## Examples

### Rust Project Detection
```yaml
Detected Stack: Rust
Evidence: 
  - 15 Cargo.toml files
  - 200+ .rs files
  - cargo build/test commands
Configuration:
  build: cargo build
  test: cargo test
  check: cargo check
  fmt: cargo fmt
File Patterns:
  source: "**/*.rs"
  test: "**/*test.rs"
  lib: "**/src/lib.rs"
```

### TypeScript Project Detection
```yaml
Detected Stack: TypeScript
Evidence:
  - 8 package.json files
  - 150+ .ts/.tsx files
  - npm scripts available
Configuration:
  build: npm run build
  test: npm test
  lint: npm run lint
  type_check: tsc --noEmit
File Patterns:
  source: "**/*.ts"
  test: "**/*.test.ts"
  lib: "**/src/index.ts"
```

## Cross-Stack Support

### Migration Support
When migrating stacks, the skill helps map:
- Dependencies: Cargo.toml ↔ package.json ↔ pyproject.toml
- Testing: cargo test ↔ npm test ↔ pytest
- Build: cargo build ↔ npm build ↔ python -m build

### Hybrid Projects
For projects with multiple stacks, the skill:
- Identifies primary vs secondary stacks
- Configures agents for the dominant stack
- Provides special handling for mixed files
- Maintains stack-specific configurations

## Integration with Workflows

The stack detector integrates with all masday workflows:

1. **Workflow Initialization**: Auto-detect stack and configure initial agents
2. **Agent Assignment**: Route tasks to stack-appropriate agents
3. **Task Execution**: Use stack-specific build and test commands
4. **Quality Assurance**: Apply stack-specific linting and validation
5. **Code Review**: Use stack-specific quality standards

## Memory Persistence

Stack configuration is persisted and automatically recalled:
```yaml
Stored Configuration:
  current_stack: "rust-workflow"
  last_detected: "2024-01-15T10:30:00Z"
  preferences:
    auto_format: true
    test_command: "cargo test"
  custom_configurations:
    build_flags: "--release"
```

## Error Handling

- **Mixed stacks**: Use primary stack, provide hybrid configuration
- **Unknown stacks**: Fallback to generic, detect common patterns
- **Incomplete detection**: Use fallback methods, prompt user
- **Conflicting configs**: Resolve by priority, provide merged config

## Command Options

```bash
/masday-stack-detector --force          # Force re-detection
/masday-stack-detector --json           # Output detection as JSON
/masday-stack-detector --migrate       # Help migrate between stacks
/masday-stack-detector --config         # Show current stack configuration
```

## Benefits

1. **Adaptive Agents**: Automatically work with any technology stack
2. **Reduced Configuration**: Minimal manual setup required
3. **Cross-Stack Support**: Same agents work across different stacks
4. **Memory Persistence**: Remember stack preferences across sessions
5. **Migration Support**: Easier stack migration and adaptation
6. **Flexible Tooling**: Use appropriate tools for each stack

## Agent Integration

The stack detector is automatically integrated with:

- **Agent routing**: Tasks route to stack-appropriate agents
- **Tool configuration**: MCP tools adapt to detected stack
- **Build processes**: Use stack-appropriate build commands
- **Testing**: Use stack-specific test runners
- **Code quality**: Apply stack-appropriate linting rules

This creates a truly flexible agent system that can work across different technology stacks while maintaining the core masday workflow functionality.