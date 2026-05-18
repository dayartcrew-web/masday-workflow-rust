# Agent Routing Reference

## Routing Table

| Task Type | Agent | Model | Tools |
|-----------|-------|-------|-------|
| Workflow coordination | orchestrator | sonnet | workflow.*, policy.*, memory.* |
| Planning/analysis | planner | sonnet | Read, Grep, Glob, Bash |
| Code implementation | executor | sonnet | Read, Write, Edit, Bash |
| Code review | reviewer | sonnet | Read, Grep, Glob, Bash |
| Final validation | verifier | sonnet | Read, Grep, Glob, Bash |
| Bug investigation | debugger | sonnet | Read, Write, Edit, Bash |
| Branch merging | synthesizer | sonnet | Read, Write, Edit, Bash |
| External research | researcher | sonnet | WebSearch, WebFetch, Context7 |
| Backend development | backend | sonnet | filesystem.*, npm.*, git.*, docker.* |
| Frontend development | frontend | sonnet | filesystem.*, npm.*, code.search |
| Testing/QA | qa | sonnet | tests.*, git.*, cicd.*, github.* |

## Dispatch Patterns

1. **Automatic delegation**: Claude reads agent descriptions and delegates
2. **Natural language**: "Use the planner agent to break this down"
3. **@-mention**: `@"planner (agent)" analyze this feature`
4. **Foreground vs Background**: Foreground blocks; background runs concurrently

## Tool Namespace Patterns

- `workflow.*` - Workflow CRUD operations
- `filesystem.*` - File operations (read, write, list, delete, stat)
- `search.*` - Code search and context packs
- `memory.*` - Memory store/recall operations
- `policy.*` - Validation and compliance
- `capability.*` - Agent/skill registry
