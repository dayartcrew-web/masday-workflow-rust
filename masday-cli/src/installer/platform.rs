use home::home_dir;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Platform {
    ClaudeCode,
    GeminiCli,
    VsCodeCopilot,
    OpenCode,
}

impl Platform {
    pub fn name(&self) -> &'static str {
        match self {
            Platform::ClaudeCode => "claude-code",
            Platform::GeminiCli => "gemini",
            Platform::VsCodeCopilot => "vscode",
            Platform::OpenCode => "opencode",
        }
    }

    pub fn project_agents_dir(&self, project_dir: &Path) -> PathBuf {
        match self {
            Platform::ClaudeCode => project_dir.join(".claude/agents"),
            Platform::GeminiCli => project_dir.join(".gemini/agents"),
            Platform::VsCodeCopilot => project_dir.join(".agents"),
            Platform::OpenCode => project_dir.join(".opencode/agents"),
        }
    }

    pub fn project_skills_dir(&self, project_dir: &Path) -> PathBuf {
        match self {
            Platform::ClaudeCode => project_dir.join(".claude/skills"),
            Platform::GeminiCli => project_dir.join(".gemini/skills"),
            Platform::VsCodeCopilot => project_dir.join(".continue/skills"),
            Platform::OpenCode => project_dir.join(".opencode/skills"),
        }
    }

    pub fn global_agents_dir(&self) -> Option<PathBuf> {
        match self {
            Platform::ClaudeCode => Some(home::home_dir()?.join(".claude/agents")),
            Platform::GeminiCli => Some(home::home_dir()?.join(".gemini/agents")),
            Platform::VsCodeCopilot => Some(home::home_dir()?.join(".continue/agents")),
            Platform::OpenCode => Some(home::home_dir()?.join(".config/opencode/agents")),
        }
    }

    pub fn global_skills_dir(&self) -> Option<PathBuf> {
        let home = home_dir()?;
        match self {
            Platform::ClaudeCode => Some(home.join(".claude/skills")),
            Platform::GeminiCli => Some(home.join(".gemini/config/skills")),
            Platform::VsCodeCopilot => None, // No global skills directory
            Platform::OpenCode => Some(home.join(".config/opencode/skills")),
        }
    }

    pub fn global_hooks_dir(&self) -> Option<PathBuf> {
        let home = home_dir()?;
        match self {
            Platform::ClaudeCode => Some(home.join(".claude/hooks")),
            Platform::GeminiCli => Some(home.join(".gemini/config/hooks")),
            Platform::VsCodeCopilot => None,
            Platform::OpenCode => None,
        }
    }

    pub fn project_mcp_config_path(&self, project_dir: &Path) -> PathBuf {
        match self {
            Platform::ClaudeCode => project_dir.join(".mcp.json"),
            Platform::GeminiCli => project_dir.join(".gemini/settings.json"),
            Platform::VsCodeCopilot => project_dir.join(".vscode/mcp.json"),
            Platform::OpenCode => project_dir.join(".opencode/mcp.json"),
        }
    }

    /// Global MCP config path — user-level config that applies to all projects.
    pub fn global_mcp_config_path(&self) -> Option<PathBuf> {
        match self {
            Platform::ClaudeCode => home::home_dir().map(|h| h.join(".claude/settings.json")),
            Platform::GeminiCli => home::home_dir().map(|h| h.join(".gemini/settings.json")),
            Platform::VsCodeCopilot => {
                // VS Code uses project-level only; global is in User/settings.json
                None
            }
            Platform::OpenCode => None,
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
        Platform::GeminiCli,
        Platform::VsCodeCopilot,
        Platform::OpenCode,
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
        if home.join(".gemini").exists() {
            platforms.push(Platform::GeminiCli);
        }
        if home.join(".continue").exists() {
            platforms.push(Platform::VsCodeCopilot);
        }
        if home.join(".config/opencode").exists() {
            platforms.push(Platform::OpenCode);
        }
    }

    if platforms.is_empty() {
        platforms.push(Platform::ClaudeCode);
    }

    platforms
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_platform_names() {
        assert_eq!(Platform::ClaudeCode.name(), "claude-code");
        assert_eq!(Platform::GeminiCli.name(), "gemini");
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
    }
}
