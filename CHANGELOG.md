# Changelog

All notable changes to masday-workflow-rebuild.

## [0.1.0] - 2026-05-18

### Added
- Initial unified codebase merging msd-mcp and masday-workflow-reborn
- 12 packages under @mcp-rebuild/* scope
- 6 MCP server apps with 70+ total tools
- 4-layer memory system (working, episodic, long-term, graph)
- 3-tier workflow engine (basic, enhanced, orchestrating)
- Multi-platform support: Claude Code, Codex CLI, Gemini CLI, Continue, GitHub Copilot
- Prisma + PostgreSQL + pgvector database layer
- Official MCP SDK pattern with McpServer
- 26 specialist agents
- 32 skills
- 9 hooks (5 executable JS, 3 advisory MD, 1 runner MJS)
- Docker Compose for PostgreSQL + pgvector
- CLI templates for scaffolding agents, skills, commands
- Vitest test suite with registry validation
- Setup scripts for bash and PowerShell
