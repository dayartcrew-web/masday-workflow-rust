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
    let config_path = platform.mcp_config_path(project_dir);

    match platform {
        Platform::ClaudeCode => {
            write_claude_code_config(&config_path, config)?;
        }
        Platform::GeminiCli => {
            update_gemini_config(&config_path, config)?;
        }
        Platform::VsCodeCopilot => {
            write_vscode_config(&config_path, config)?;
        }
        Platform::OpenCode => {
            write_opencode_config(&config_path, config)?;
        }
    }

    Ok(())
}

pub fn remove_mcp_config(platform: &Platform, project_dir: &Path) -> Result<()> {
    let config_path = platform.mcp_config_path(project_dir);

    if !config_path.exists() {
        return Ok(());
    }

    match platform {
        Platform::GeminiCli => {
            remove_from_gemini_config(&config_path)?;
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

fn write_claude_code_config(path: &Path, config: &McpConfig) -> Result<()> {
    let mut env_map = serde_json::Map::new();
    env_map.insert(
        "MASDAY_API_URL".to_string(),
        JsonValue::String(config.api_url.clone()),
    );
    env_map.insert(
        "MASDAY_API_KEY".to_string(),
        JsonValue::String(config.api_key.clone()),
    );

    if let Some(ref db_url) = config.database_url {
        env_map.insert(
            "DATABASE_URL".to_string(),
            JsonValue::String(db_url.clone()),
        );
    }

    let mut server = serde_json::Map::new();
    server.insert("type".to_string(), JsonValue::String("stdio".to_string()));
    server.insert(
        "command".to_string(),
        JsonValue::String(config.mcp_binary_path.display().to_string()),
    );
    server.insert("env".to_string(), JsonValue::Object(env_map));

    let mut mcp_servers = serde_json::Map::new();
    mcp_servers.insert("masday".to_string(), JsonValue::Object(server));

    let json = serde_json::json!({
        "mcpServers": mcp_servers
    });

    write_json_file(path, json)
}

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

    let mut root = existing_json
        .clone()
        .unwrap_or_else(|| JsonValue::Object(serde_json::Map::new()));

    let mcp_servers = root
        .as_object_mut()
        .ok_or_else(|| anyhow::anyhow!("Root should be an object"))?
        .entry("mcpServers".to_string())
        .or_insert_with(|| JsonValue::Object(serde_json::Map::new()))
        .as_object_mut()
        .ok_or_else(|| anyhow::anyhow!("mcpServers should be an object"))?
        .clone();

    let mut env_map = serde_json::Map::new();
    env_map.insert(
        "MASDAY_API_URL".to_string(),
        JsonValue::String(config.api_url.clone()),
    );
    env_map.insert(
        "MASDAY_API_KEY".to_string(),
        JsonValue::String(config.api_key.clone()),
    );

    if let Some(ref db_url) = config.database_url {
        env_map.insert(
            "DATABASE_URL".to_string(),
            JsonValue::String(db_url.clone()),
        );
    }

    let mut server = serde_json::Map::new();
    server.insert("type".to_string(), JsonValue::String("stdio".to_string()));
    server.insert(
        "command".to_string(),
        JsonValue::String(config.mcp_binary_path.display().to_string()),
    );
    server.insert("env".to_string(), JsonValue::Object(env_map));

    let mcp_servers_obj = mcp_servers;
    let mut new_mcp_servers = mcp_servers_obj.clone();
    new_mcp_servers.insert("masday".to_string(), JsonValue::Object(server));

    if let Some(obj) = root.as_object_mut() {
        obj.insert("mcpServers".to_string(), JsonValue::Object(new_mcp_servers));
    }

    write_json_file(path, root)
}

fn remove_from_gemini_config(path: &Path) -> Result<()> {
    if !path.exists() {
        return Ok(());
    }

    let content = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read {}", path.display()))?;
    let mut json = serde_json::from_str::<JsonValue>(&content)
        .unwrap_or_else(|_| JsonValue::Object(serde_json::Map::new()));

    if let Some(obj) = json.as_object_mut() {
        if let Some(mcp_servers) = obj.get_mut("mcpServers").and_then(|v| v.as_object_mut()) {
            mcp_servers.remove("masday");
        }
    }

    write_json_file(path, json)
}

fn write_vscode_config(path: &Path, config: &McpConfig) -> Result<()> {
    let mut env_map = serde_json::Map::new();
    env_map.insert(
        "MASDAY_API_URL".to_string(),
        JsonValue::String(config.api_url.clone()),
    );
    env_map.insert(
        "MASDAY_API_KEY".to_string(),
        JsonValue::String(config.api_key.clone()),
    );

    if let Some(ref db_url) = config.database_url {
        env_map.insert(
            "DATABASE_URL".to_string(),
            JsonValue::String(db_url.clone()),
        );
    }

    let mut server = serde_json::Map::new();
    server.insert(
        "command".to_string(),
        JsonValue::String(config.mcp_binary_path.display().to_string()),
    );
    server.insert("env".to_string(), JsonValue::Object(env_map));

    let mut servers = serde_json::Map::new();
    servers.insert("masday".to_string(), JsonValue::Object(server));

    let json = serde_json::json!({
        "servers": servers
    });

    write_json_file(path, json)
}

fn write_opencode_config(path: &Path, config: &McpConfig) -> Result<()> {
    write_vscode_config(path, config)
}

fn write_json_file(path: &Path, json: JsonValue) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create directory {}", parent.display()))?;
    }

    let content = serde_json::to_string_pretty(&json).context("Failed to serialize JSON")?;
    std::fs::write(path, content).with_context(|| format!("Failed to write {}", path.display()))?;

    Ok(())
}

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
            mcp_binary_path: "/path/to/masday-mcp".into(),
            api_url: masday_core::constants::ports::api_base_url(),
            api_key: "test-key".to_string(),
            database_url: Some("postgresql://localhost/db".to_string()),
        };

        write_claude_code_config(&config_path, &config).unwrap();

        let content = std::fs::read_to_string(&config_path).unwrap();
        let json: JsonValue = serde_json::from_str(&content).unwrap();

        assert!(json["mcpServers"]["masday"]["type"] == "stdio");
        assert!(json["mcpServers"]["masday"]["env"]["MASDAY_API_URL"] == "http://localhost:30101");
    }

    #[test]
    fn test_write_vscode_config() {
        let temp_dir = TempDir::new().unwrap();
        let project_dir = temp_dir.path();
        let config_path = project_dir.join(".vscode/mcp.json");

        let config = McpConfig {
            mcp_binary_path: "/path/to/masday-mcp".into(),
            api_url: masday_core::constants::ports::api_base_url(),
            api_key: "test-key".to_string(),
            database_url: None,
        };

        write_vscode_config(&config_path, &config).unwrap();

        let content = std::fs::read_to_string(&config_path).unwrap();
        let json: JsonValue = serde_json::from_str(&content).unwrap();

        assert!(json["servers"]["masday"]["command"] == "/path/to/masday-mcp");
    }
}
