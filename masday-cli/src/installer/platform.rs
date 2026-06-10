use home::home_dir;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Platform {
    ClaudeCode,
    ClaudeDesktop,
    GeminiCli,
    VsCodeCopilot,
    OpenCode,
    Cursor,
    Windsurf,
    Codex,
}

impl Platform {
    pub fn name(&self) -> &'static str {
        match self {
            Platform::ClaudeCode => "claude-code",
            Platform::ClaudeDesktop => "claude-desktop",
            Platform::GeminiCli => "gemini",
            Platform::VsCodeCopilot => "vscode",
            Platform::OpenCode => "opencode",
            Platform::Cursor => "cursor",
            Platform::Windsurf => "windsurf",
            Platform::Codex => "codex",
        }
    }

    pub fn project_agents_dir(&self, project_dir: &Path) -> PathBuf {
        match self {
            Platform::ClaudeCode => project_dir.join(".claude/agents"),
            Platform::ClaudeDesktop => project_dir.join(".claude/agents"),
            Platform::GeminiCli => project_dir.join(".gemini/agents"),
            Platform::VsCodeCopilot => project_dir.join(".agents"),
            Platform::OpenCode => project_dir.join(".opencode/agents"),
            Platform::Cursor => project_dir.join(".cursor/rules"),
            Platform::Windsurf => project_dir.join(".windsurf/rules"),
            Platform::Codex => project_dir.join(".codex/agents"),
        }
    }

    pub fn project_skills_dir(&self, project_dir: &Path) -> PathBuf {
        match self {
            Platform::ClaudeCode => project_dir.join(".claude/skills"),
            Platform::ClaudeDesktop => project_dir.join(".claude/skills"),
            Platform::GeminiCli => project_dir.join(".gemini/skills"),
            Platform::VsCodeCopilot => project_dir.join(".continue/skills"),
            Platform::OpenCode => project_dir.join(".opencode/skills"),
            Platform::Cursor => project_dir.join(".cursor/skills"),
            Platform::Windsurf => project_dir.join(".windsurf/skills"),
            Platform::Codex => project_dir.join(".codex/skills"),
        }
    }

    pub fn global_agents_dir(&self) -> Option<PathBuf> {
        match self {
            Platform::ClaudeCode => Some(home::home_dir()?.join(".claude/agents")),
            Platform::ClaudeDesktop => None, // Desktop app — no global agents dir
            Platform::GeminiCli => Some(home::home_dir()?.join(".gemini/agents")),
            Platform::VsCodeCopilot => Some(home::home_dir()?.join(".continue/agents")),
            Platform::OpenCode => Some(home::home_dir()?.join(".config/opencode/agents")),
            Platform::Cursor => None, // Cursor uses .cursor/rules/ project-level only
            Platform::Windsurf => None, // No agent folder
            Platform::Codex => Some(home::home_dir()?.join(".codex/agents")),
        }
    }

    pub fn global_skills_dir(&self) -> Option<PathBuf> {
        let home = home_dir()?;
        match self {
            Platform::ClaudeCode => Some(home.join(".claude/skills")),
            Platform::ClaudeDesktop => None,
            Platform::GeminiCli => Some(home.join(".gemini/config/skills")),
            Platform::VsCodeCopilot => None, // No global skills directory
            Platform::OpenCode => Some(home.join(".config/opencode/skills")),
            Platform::Cursor => None,
            Platform::Windsurf => None,
            Platform::Codex => None,
        }
    }

    pub fn global_hooks_dir(&self) -> Option<PathBuf> {
        let home = home_dir()?;
        match self {
            Platform::ClaudeCode => Some(home.join(".claude/hooks")),
            Platform::ClaudeDesktop => None,
            Platform::GeminiCli => Some(home.join(".gemini/config/hooks")),
            Platform::VsCodeCopilot => None,
            Platform::OpenCode => None,
            Platform::Cursor => None,
            Platform::Windsurf => None,
            Platform::Codex => None,
        }
    }

    pub fn project_mcp_config_path(&self, project_dir: &Path) -> PathBuf {
        match self {
            Platform::ClaudeCode => project_dir.join(".mcp.json"),
            Platform::ClaudeDesktop => project_dir.join(".mcp.json"), // Same as Claude Code
            Platform::GeminiCli => project_dir.join(".gemini/settings.json"),
            Platform::VsCodeCopilot => project_dir.join(".vscode/mcp.json"),
            Platform::OpenCode => project_dir.join(".opencode/mcp.json"),
            Platform::Cursor => project_dir.join(".cursor/mcp.json"),
            Platform::Windsurf => project_dir.join(".windsurf/mcp.json"),
            Platform::Codex => project_dir.join(".codex/mcp.json"),
        }
    }

    /// Global MCP config path — user-level config that applies to all projects.
    pub fn global_mcp_config_path(&self) -> Option<PathBuf> {
        match self {
            Platform::ClaudeCode => home::home_dir().map(|h| h.join(".claude.json")),
            Platform::ClaudeDesktop => {
                // OS-specific: macOS ~/Library/Application Support/Claude/claude_desktop_config.json
                // Linux ~/.config/Claude/claude_desktop_config.json
                // Windows %APPDATA%/Claude/claude_desktop_config.json
                let home = home::home_dir()?;
                if cfg!(target_os = "macos") {
                    Some(home.join("Library/Application Support/Claude/claude_desktop_config.json"))
                } else if cfg!(target_os = "windows") {
                    std::env::var("APPDATA").ok().map(|appdata| {
                        PathBuf::from(appdata).join("Claude/claude_desktop_config.json")
                    })
                } else {
                    // Linux
                    Some(home.join(".config/Claude/claude_desktop_config.json"))
                }
            }
            Platform::GeminiCli => home::home_dir().map(|h| h.join(".gemini/settings.json")),
            Platform::VsCodeCopilot => None,
            Platform::OpenCode => None,
            Platform::Cursor => home::home_dir().map(|h| h.join(".cursor/mcp.json")),
            Platform::Windsurf => {
                home::home_dir().map(|h| h.join(".codeium/windsurf/mcp_config.json"))
            }
            Platform::Codex => None, // No global config
        }
    }

    // Backward compatibility alias
    pub fn mcp_config_path(&self, project_dir: &Path) -> PathBuf {
        self.project_mcp_config_path(project_dir)
    }
}

pub fn all_platforms() -> Vec<Platform> {
    vec![
        Platform::ClaudeCode,
        Platform::ClaudeDesktop,
        Platform::GeminiCli,
        Platform::VsCodeCopilot,
        Platform::OpenCode,
        Platform::Cursor,
        Platform::Windsurf,
        Platform::Codex,
    ]
}

pub fn detect_active_platforms(project_dir: &Path) -> Vec<Platform> {
    all_platforms()
        .into_iter()
        .filter(|platform| {
            let config_path = platform.project_mcp_config_path(project_dir);
            config_path.exists()
        })
        .collect()
}

/// Backward compatibility - detect from home directory
#[allow(dead_code)]
pub fn detect_active_platforms_from_home() -> Vec<Platform> {
    let mut platforms = Vec::new();

    if let Some(home) = home_dir() {
        if home.join(".claude").exists() {
            platforms.push(Platform::ClaudeCode);
        }
        // Claude Desktop detection — check OS-specific config path
        {
            let desktop_config = if cfg!(target_os = "macos") {
                home.join("Library/Application Support/Claude/claude_desktop_config.json")
            } else if cfg!(target_os = "windows") {
                std::env::var("APPDATA")
                    .map(|appdata| PathBuf::from(appdata).join("Claude/claude_desktop_config.json"))
                    .unwrap_or_else(|_| home.join(".config/Claude/claude_desktop_config.json"))
            } else {
                home.join(".config/Claude/claude_desktop_config.json")
            };
            if desktop_config.exists() {
                platforms.push(Platform::ClaudeDesktop);
            }
        }
        if home.join(".gemini").exists() {
            platforms.push(Platform::GeminiCli);
        }
        if home.join(".continue").exists() {
            platforms.push(Platform::VsCodeCopilot);
        }
        if home.join(".config/opencode").exists() {
            platforms.push(Platform::OpenCode);
        }
        if home.join(".cursor").exists() {
            platforms.push(Platform::Cursor);
        }
        if home.join(".codeium").exists() {
            platforms.push(Platform::Windsurf);
        }
        if home.join(".codex").exists() {
            platforms.push(Platform::Codex);
        }
    }

    platforms
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_platform_names() {
        assert_eq!(Platform::ClaudeCode.name(), "claude-code");
        assert_eq!(Platform::ClaudeDesktop.name(), "claude-desktop");
        assert_eq!(Platform::GeminiCli.name(), "gemini");
        assert_eq!(Platform::Cursor.name(), "cursor");
        assert_eq!(Platform::Windsurf.name(), "windsurf");
        assert_eq!(Platform::Codex.name(), "codex");
    }

    #[test]
    fn test_project_paths() {
        let project = Path::new("/test/project");

        assert_eq!(
            Platform::ClaudeCode.project_agents_dir(project),
            PathBuf::from("/test/project/.claude/agents")
        );
        assert_eq!(
            Platform::GeminiCli.project_skills_dir(project),
            PathBuf::from("/test/project/.gemini/skills")
        );
        assert_eq!(
            Platform::Cursor.project_mcp_config_path(project),
            PathBuf::from("/test/project/.cursor/mcp.json")
        );
        assert_eq!(
            Platform::Windsurf.project_mcp_config_path(project),
            PathBuf::from("/test/project/.windsurf/mcp.json")
        );
        assert_eq!(
            Platform::Codex.project_mcp_config_path(project),
            PathBuf::from("/test/project/.codex/mcp.json")
        );
    }
}
