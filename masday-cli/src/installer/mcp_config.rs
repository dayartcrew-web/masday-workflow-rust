use super::platform::Platform;
use anyhow::{Context, Result};
use serde_json::Value as JsonValue;
use std::path::Path;

pub struct McpConfig {
    pub mcp_binary_path: std::path::PathBuf,
    pub api_url: String,
    pub api_key: String,
    pub database_url: Option<String>,
}

pub fn generate_mcp_config(
    platform: &Platform,
    project_dir: &Path,
    config: &McpConfig,
) -> Result<()> {
    // 1. Write to project-level config (.mcp.json, .gemini/settings.json, etc.)
    let project_config_path = platform.project_mcp_config_path(project_dir);
    match platform {
        Platform::ClaudeCode => {
            write_claude_code_config(&project_config_path, config)?;
        }
        Platform::GeminiCli => {
            update_gemini_config(&project_config_path, config)?;
        }
        Platform::VsCodeCopilot => {
            write_vscode_config(&project_config_path, config)?;
        }
        Platform::OpenCode => {
            write_opencode_config(&project_config_path, config)?;
        }
    }

    // 2. Also write to global config if available
    if let Some(global_path) = platform.global_mcp_config_path() {
        match platform {
            Platform::ClaudeCode => {
                update_claude_global_settings(&global_path, config)?;
            }
            Platform::GeminiCli => {
                update_gemini_config(&global_path, config)?;
            }
            _ => {}
        }
    }

    Ok(())
}

pub fn remove_mcp_config(platform: &Platform, project_dir: &Path) -> Result<()> {
    let config_path = platform.project_mcp_config_path(project_dir);

    if !config_path.exists() {
        return Ok(());
    }

    match platform {
        Platform::GeminiCli | Platform::ClaudeCode => {
            remove_server_from_json(&config_path, "masday")?;
        }
        _ => {
            if config_path.exists() {
                std::fs::remove_file(&config_path)
                    .with_context(|| format!("Failed to remove {}", config_path.display()))?;
            }
        }
    }

    Ok(())
}

// ─── Build server object ─────────────────────────────────────────────────

/// Build the masday MCP server JSON object (reused by all platforms)
fn build_server_object(config: &McpConfig) -> JsonValue {
    let mut env_map = serde_json::Map::new();
    if !config.api_url.is_empty() {
        env_map.insert(
            "MASDAY_API_URL".to_string(),
            JsonValue::String(config.api_url.clone()),
        );
    }
    if !config.api_key.is_empty() {
        env_map.insert(
            "MASDAY_API_KEY".to_string(),
            JsonValue::String(config.api_key.clone()),
        );
    }

    let mut server = serde_json::Map::new();
    server.insert("type".to_string(), JsonValue::String("stdio".to_string()));
    server.insert(
        "command".to_string(),
        JsonValue::String(config.mcp_binary_path.display().to_string()),
    );
    server.insert(
        "args".to_string(),
        JsonValue::Array(vec![JsonValue::String("mcp".to_string())]),
    );
    server.insert("env".to_string(), JsonValue::Object(env_map));

    JsonValue::Object(server)
}

// ─── Platform-specific writers ───────────────────────────────────────────

/// Write .mcp.json (project-level, Claude Code)
fn write_claude_code_config(path: &Path, config: &McpConfig) -> Result<()> {
    let mut servers = serde_json::Map::new();
    servers.insert("masday".to_string(), build_server_object(config));

    let mut root = serde_json::Map::new();
    root.insert("mcpServers".to_string(), JsonValue::Object(servers));

    write_json_file(path, serde_json::Value::Object(root))
}

/// Update ~/.claude/settings.json → mcpServers.masday (global, Claude Code)
fn update_claude_global_settings(path: &Path, config: &McpConfig) -> Result<()> {
    if !path.exists() {
        // Create minimal settings.json
        let mut servers = serde_json::Map::new();
        servers.insert("masday".to_string(), build_server_object(config));

        let mut root = serde_json::Map::new();
        root.insert("mcpServers".to_string(), JsonValue::Object(servers));

        write_json_file(path, serde_json::Value::Object(root))?;
        return Ok(());
    }

    // Read existing settings, merge masday server
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read {}", path.display()))?;
    let mut json =
        serde_json::from_str::<JsonValue>(&content).unwrap_or_else(|_| serde_json::json!({}));

    let root_obj = json
        .as_object_mut()
        .ok_or_else(|| anyhow::anyhow!("Root should be an object"))?;

    let mcp_servers = root_obj
        .entry("mcpServers")
        .or_insert_with(|| serde_json::json!({}))
        .as_object_mut()
        .ok_or_else(|| anyhow::anyhow!("mcpServers should be an object"))?;

    mcp_servers.insert("masday".to_string(), build_server_object(config));

    write_json_file(path, json)
}

/// Update .gemini/settings.json (project or global)
fn update_gemini_config(path: &Path, config: &McpConfig) -> Result<()> {
    let existing_json = if path.exists() {
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read {}", path.display()))?;
        Some(
            serde_json::from_str::<JsonValue>(&content)
                .unwrap_or_else(|_| JsonValue::Object(serde_json::Map::new())),
        )
    } else {
        None
    };

    let mut root = existing_json.unwrap_or_else(|| JsonValue::Object(serde_json::Map::new()));

    let mcp_servers = root
        .as_object_mut()
        .ok_or_else(|| anyhow::anyhow!("Root should be an object"))?
        .entry("mcpServers")
        .or_insert_with(|| serde_json::json!({}))
        .as_object_mut()
        .ok_or_else(|| anyhow::anyhow!("mcpServers should be an object"))?;

    mcp_servers.insert("masday".to_string(), build_server_object(config));

    write_json_file(path, root)
}

/// Write .vscode/mcp.json or .opencode/mcp.json
fn write_vscode_config(path: &Path, config: &McpConfig) -> Result<()> {
    let mut servers = serde_json::Map::new();
    servers.insert("masday".to_string(), build_server_object(config));

    let mut root = serde_json::Map::new();
    root.insert("servers".to_string(), JsonValue::Object(servers));

    write_json_file(path, serde_json::Value::Object(root))
}

fn write_opencode_config(path: &Path, config: &McpConfig) -> Result<()> {
    write_vscode_config(path, config)
}

// ─── Remove helpers ──────────────────────────────────────────────────────

fn remove_server_from_json(path: &Path, server_name: &str) -> Result<()> {
    if !path.exists() {
        return Ok(());
    }

    let content = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read {}", path.display()))?;
    let mut json = serde_json::from_str::<JsonValue>(&content)
        .unwrap_or_else(|_| JsonValue::Object(serde_json::Map::new()));

    // Try mcpServers key (Claude Code, Gemini)
    if let Some(obj) = json.as_object_mut() {
        if let Some(servers) = obj.get_mut("mcpServers").and_then(|v| v.as_object_mut()) {
            servers.remove(server_name);
        }
        if let Some(servers) = obj.get_mut("servers").and_then(|v| v.as_object_mut()) {
            servers.remove(server_name);
        }
    }

    write_json_file(path, json)
}

// ─── File I/O ────────────────────────────────────────────────────────────

fn write_json_file(path: &Path, json: JsonValue) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create directory {}", parent.display()))?;
    }

    let content = serde_json::to_string_pretty(&json).context("Failed to serialize JSON")?;
    std::fs::write(path, content).with_context(|| format!("Failed to write {}", path.display()))?;

    Ok(())
}

// ─── Tests ───────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_write_claude_code_config() {
        let temp_dir = TempDir::new().unwrap();
        let project_dir = temp_dir.path();
        let config_path = project_dir.join(".mcp.json");

        let config = McpConfig {
            mcp_binary_path: "/path/to/masday".into(),
            api_url: masday_core::constants::ports::api_base_url(),
            api_key: "***".to_string(),
            database_url: Some("postgresql://localhost/db".to_string()),
        };

        write_claude_code_config(&config_path, &config).unwrap();

        let content = std::fs::read_to_string(&config_path).unwrap();
        let json: JsonValue = serde_json::from_str(&content).unwrap();

        assert!(json["mcpServers"]["masday"]["type"] == "stdio");
        assert!(json["mcpServers"]["masday"]["env"]["MASDAY_API_URL"] == "http://localhost:30101");
        // Args must include "mcp" subcommand — single binary contains both CLI and MCP
        assert_eq!(
            json["mcpServers"]["masday"]["args"],
            serde_json::json!(["mcp"])
        );
    }

    #[test]
    fn test_write_vscode_config() {
        let temp_dir = TempDir::new().unwrap();
        let project_dir = temp_dir.path();
        let config_path = project_dir.join(".vscode/mcp.json");

        let config = McpConfig {
            mcp_binary_path: "/path/to/masday".into(),
            api_url: String::new(),
            api_key: String::new(),
            database_url: None,
        };

        write_vscode_config(&config_path, &config).unwrap();

        let content = std::fs::read_to_string(&config_path).unwrap();
        let json: JsonValue = serde_json::from_str(&content).unwrap();

        assert!(json["servers"]["masday"]["command"] == "/path/to/masday");
        // Args must include "mcp" subcommand
        assert_eq!(
            json["servers"]["masday"]["args"],
            serde_json::json!(["mcp"])
        );
    }

    #[test]
    fn test_update_claude_global_settings_creates_new() {
        let temp_dir = TempDir::new().unwrap();
        let settings_path = temp_dir.path().join("settings.json");

        let config = McpConfig {
            mcp_binary_path: "/home/user/.masday/bin/masday".into(),
            api_url: "http://localhost:30101".to_string(),
            api_key: "***".to_string(),
            database_url: None,
        };

        update_claude_global_settings(&settings_path, &config).unwrap();

        let content = std::fs::read_to_string(&settings_path).unwrap();
        let json: JsonValue = serde_json::from_str(&content).unwrap();

        assert!(json["mcpServers"]["masday"]["command"] == "/home/user/.masday/bin/masday");
        // Args must include "mcp" subcommand — single binary contains both CLI and MCP
        assert_eq!(
            json["mcpServers"]["masday"]["args"],
            serde_json::json!(["mcp"])
        );
    }

    #[test]
    fn test_update_claude_global_settings_merges_existing() {
        let temp_dir = TempDir::new().unwrap();
        let settings_path = temp_dir.path().join("settings.json");

        // Write existing settings with other servers
        let existing = serde_json::json!({
            "env": {"FOO": "bar"},
            "mcpServers": {
                "other-server": {"command": "other"}
            }
        });
        std::fs::write(
            &settings_path,
            serde_json::to_string_pretty(&existing).unwrap(),
        )
        .unwrap();

        let config = McpConfig {
            mcp_binary_path: "/home/user/.masday/bin/masday".into(),
            api_url: "http://localhost:30101".to_string(),
            api_key: "***".to_string(),
            database_url: None,
        };

        update_claude_global_settings(&settings_path, &config).unwrap();

        let content = std::fs::read_to_string(&settings_path).unwrap();
        let json: JsonValue = serde_json::from_str(&content).unwrap();

        // Existing server preserved
        assert!(json["mcpServers"]["other-server"]["command"] == "other");
        // New server added with correct args
        assert_eq!(
            json["mcpServers"]["masday"]["args"],
            serde_json::json!(["mcp"])
        );
        // Other settings preserved
        assert!(json["env"]["FOO"] == "bar");
    }
}
