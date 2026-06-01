use std::path::Path;
use anyhow::{Result, Context};
use serde_json::Value as JsonValue;

#[derive(Debug, Default)]
pub struct SettingsUpdates {
    pub statusline_cmd: Option<String>,
    pub auto_compact: Option<bool>,
    pub auto_compact_threshold: Option<f64>,
    pub mcp_server: Option<McpServerConfig>,
}

#[derive(Debug)]
pub struct McpServerConfig {
    pub command: String,
    pub env: Vec<(String, String)>,
}

pub fn update_global_settings(settings_path: &Path, updates: &SettingsUpdates) -> Result<()> {
    let mut json = if settings_path.exists() {
        let content = std::fs::read_to_string(settings_path)
            .with_context(|| format!("Failed to read settings from {}", settings_path.display()))?;
        serde_json::from_str(&content).unwrap_or_else(|_| JsonValue::Object(serde_json::Map::new()))
    } else {
        JsonValue::Object(serde_json::Map::new())
    };

    let obj = json.as_object_mut()
        .expect("Root should be an object");

    if let Some(ref cmd) = updates.statusline_cmd {
        obj.insert("statusLine".to_string(), serde_json::json!({
            "type": "command",
            "command": cmd
        }));
    }

    if let Some(auto_compact) = updates.auto_compact {
        obj.insert("autoCompact".to_string(), JsonValue::Bool(auto_compact));
    }

    if let Some(threshold) = updates.auto_compact_threshold {
        obj.insert("autoCompactThreshold".to_string(), JsonValue::Number(
            serde_json::Number::from_f64(threshold).ok_or_else(|| anyhow::anyhow!("Invalid threshold value"))?
        ));
    }

    if let Some(ref mcp_config) = updates.mcp_server {
        let mcp_servers = obj.entry("mcpServers".to_string())
            .or_insert_with(|| JsonValue::Object(serde_json::Map::new()))
            .as_object_mut()
            .ok_or_else(|| anyhow::anyhow!("mcpServers should be an object"))?;

        let mut server_obj = serde_json::Map::new();
        server_obj.insert("type".to_string(), JsonValue::String("stdio".to_string()));
        server_obj.insert("command".to_string(), JsonValue::String(mcp_config.command.clone()));

        let mut env_obj = serde_json::Map::new();
        for (key, value) in &mcp_config.env {
            env_obj.insert(key.clone(), JsonValue::String(value.clone()));
        }
        server_obj.insert("env".to_string(), JsonValue::Object(env_obj));

        mcp_servers.insert("masday".to_string(), JsonValue::Object(server_obj));
    }

    let parent = settings_path.parent()
        .ok_or_else(|| anyhow::anyhow!("Settings path has no parent"))?;
    std::fs::create_dir_all(parent)
        .with_context(|| format!("Failed to create directory {}", parent.display()))?;

    let content = serde_json::to_string_pretty(&json)
        .context("Failed to serialize settings")?;
    std::fs::write(settings_path, content)
        .with_context(|| format!("Failed to write settings to {}", settings_path.display()))?;

    Ok(())
}

pub fn remove_masday_entries(settings_path: &Path) -> Result<()> {
    if !settings_path.exists() {
        return Ok(());
    }

    let content = std::fs::read_to_string(settings_path)
        .with_context(|| format!("Failed to read settings from {}", settings_path.display()))?;
    let mut json = serde_json::from_str::<JsonValue>(&content)
        .unwrap_or_else(|_| JsonValue::Object(serde_json::Map::new()));

    if let Some(obj) = json.as_object_mut() {
        obj.remove("statusLine");
        obj.remove("autoCompact");
        obj.remove("autoCompactThreshold");

        if let Some(mcp_servers) = obj.get_mut("mcpServers").and_then(|v| v.as_object_mut()) {
            mcp_servers.remove("masday");
        }
    }

    let content = serde_json::to_string_pretty(&json)
        .context("Failed to serialize settings")?;
    std::fs::write(settings_path, content)
        .with_context(|| format!("Failed to write settings to {}", settings_path.display()))?;

    Ok(())
}

pub fn update_json_config(path: &Path, key: &str, value: JsonValue) -> Result<()> {
    let mut json = if path.exists() {
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read config from {}", path.display()))?;
        serde_json::from_str(&content).unwrap_or_else(|_| JsonValue::Object(serde_json::Map::new()))
    } else {
        JsonValue::Object(serde_json::Map::new())
    };

    let keys: Vec<&str> = key.split('.').collect();
    let mut current = &mut json;

    for (i, &k) in keys.iter().enumerate() {
        if i == keys.len() - 1 {
            if let Some(obj) = current.as_object_mut() {
                obj.insert(k.to_string(), value.clone());
            }
        } else {
            let key_exists = current.as_object()
                .and_then(|o| o.get(k))
                .is_some();

            if !key_exists {
                if let Some(obj) = current.as_object_mut() {
                    obj.insert(k.to_string(), JsonValue::Object(serde_json::Map::new()));
                }
            }

            current = current.as_object_mut()
                .and_then(|o| o.get_mut(k))
                .ok_or_else(|| anyhow::anyhow!("Failed to navigate to key {}", k))?;
        }
    }

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create directory {}", parent.display()))?;
    }

    let content = serde_json::to_string_pretty(&json)
        .context("Failed to serialize config")?;
    std::fs::write(path, content)
        .with_context(|| format!("Failed to write config to {}", path.display()))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_update_global_settings_new_file() {
        let temp_dir = TempDir::new().unwrap();
        let settings_path = temp_dir.path().join("settings.json");

        let mut updates = SettingsUpdates::default();
        updates.statusline_cmd = Some("/path/to/hook.js".to_string());
        updates.auto_compact = Some(true);
        updates.auto_compact_threshold = Some(0.9);

        update_global_settings(&settings_path, &updates).unwrap();

        let content = std::fs::read_to_string(&settings_path).unwrap();
        let json: JsonValue = serde_json::from_str(&content).unwrap();

        assert!(json.get("statusLine").is_some());
        assert_eq!(json["autoCompact"], true);
        assert_eq!(json["autoCompactThreshold"], 0.9);
    }

    #[test]
    fn test_remove_masday_entries() {
        let temp_dir = TempDir::new().unwrap();
        let settings_path = temp_dir.path().join("settings.json");

        let initial_json = serde_json::json!({
            "statusLine": {"type": "command", "command": "test.js"},
            "autoCompact": true,
            "mcpServers": {
                "masday": {"command": "/path/to/mcp"},
                "other": {"command": "/other"}
            }
        });

        std::fs::write(&settings_path, serde_json::to_string_pretty(&initial_json).unwrap()).unwrap();

        remove_masday_entries(&settings_path).unwrap();

        let content = std::fs::read_to_string(&settings_path).unwrap();
        let json: JsonValue = serde_json::from_str(&content).unwrap();

        assert!(json.get("statusLine").is_none());
        assert!(json.get("autoCompact").is_none());
        assert!(json["mcpServers"]["masday"].is_null());
        assert!(json["mcpServers"]["other"].is_object());
    }
}
