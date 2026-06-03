use super::templates;
use anyhow::{Context, Result};
use std::fs;
use std::path::Path;

pub struct SyncReport {
    pub platform: String,
    pub copied: usize,
    pub skipped: usize,
}

pub fn install_global_hooks(home_dir: &Path) -> Result<SyncReport> {
    let hooks = templates::extract_global_hooks();
    let hooks_dir = home_dir.join(".claude/hooks");

    fs::create_dir_all(&hooks_dir)
        .with_context(|| format!("Failed to create directory {}", hooks_dir.display()))?;

    let mut report = SyncReport {
        platform: "global".to_string(),
        copied: 0,
        skipped: 0,
    };

    for (name, content) in hooks.iter() {
        let hook_path = hooks_dir.join(name);
        fs::write(&hook_path, content)
            .with_context(|| format!("Failed to write hook {}", hook_path.display()))?;

        if name.ends_with(".sh") {
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let mut perms = fs::metadata(&hook_path)?.permissions();
                perms.set_mode(0o755);
                fs::set_permissions(&hook_path, perms)?;
            }
        }

        report.copied += 1;
    }

    Ok(report)
}

pub fn install_project_hooks(project_dir: &Path) -> Result<SyncReport> {
    let hooks = templates::extract_project_hooks();
    let hooks_dir = project_dir.join(".claude/hooks");

    fs::create_dir_all(&hooks_dir)
        .with_context(|| format!("Failed to create directory {}", hooks_dir.display()))?;

    let mut report = SyncReport {
        platform: "project".to_string(),
        copied: 0,
        skipped: 0,
    };

    for (name, content) in hooks.iter() {
        let hook_path = hooks_dir.join(name);
        fs::write(&hook_path, content)
            .with_context(|| format!("Failed to write hook {}", hook_path.display()))?;

        if name.ends_with(".sh") {
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let mut perms = fs::metadata(&hook_path)?.permissions();
                perms.set_mode(0o755);
                fs::set_permissions(&hook_path, perms)?;
            }
        }

        report.copied += 1;
    }

    Ok(report)
}

pub fn uninstall_global_hooks(home_dir: &Path) -> Result<SyncReport> {
    let hooks = templates::extract_global_hooks();
    let hooks_dir = home_dir.join(".claude/hooks");

    let mut report = SyncReport {
        platform: "global".to_string(),
        copied: 0,
        skipped: 0,
    };

    for (name, _) in hooks.iter() {
        let hook_path = hooks_dir.join(name);
        if hook_path.exists() {
            fs::remove_file(&hook_path)
                .with_context(|| format!("Failed to remove hook {}", hook_path.display()))?;
            report.copied += 1;
        } else {
            report.skipped += 1;
        }
    }

    Ok(report)
}

pub fn uninstall_project_hooks(project_dir: &Path) -> Result<SyncReport> {
    let hooks = templates::extract_project_hooks();
    let hooks_dir = project_dir.join(".claude/hooks");

    let mut report = SyncReport {
        platform: "project".to_string(),
        copied: 0,
        skipped: 0,
    };

    for (name, _) in hooks.iter() {
        let hook_path = hooks_dir.join(name);
        if hook_path.exists() {
            fs::remove_file(&hook_path)
                .with_context(|| format!("Failed to remove hook {}", hook_path.display()))?;
            report.copied += 1;
        } else {
            report.skipped += 1;
        }
    }

    Ok(report)
}

/// Register masday hooks in Claude Code settings.json (or Gemini equivalent).
///
/// Hook mapping:
///   Status           → masday-statusline.js
///   PostToolUse      → masday-context-monitor.js
///   UserPromptSubmit → masday-context-warning.js
///   SessionStart     → masday-session-start.js
///   PreToolUse       → masday-pre-bash-guard.js
///   PreCompact       → masday-pre-compact.js
///   PostCompact      → masday-post-compact.js
///
/// Merges with existing hooks — does not remove other hooks.
pub fn register_hooks_in_settings(settings_path: &Path, home_dir: &Path) -> Result<()> {
    let hooks_dir = home_dir.join(".claude/hooks");

    // Hook definitions: (event, hook_file)
    // Note: statusline is NOT a hook — it's a separate settings field
    let hook_defs: &[(&str, &str)] = &[
        ("PostToolUse", "masday-context-monitor.js"),
        ("UserPromptSubmit", "masday-context-warning.js"),
        ("SessionStart", "masday-session-start.js"),
        ("PreToolUse", "masday-pre-bash-guard.js"),
        ("PreCompact", "masday-pre-compact.js"),
        ("PostCompact", "masday-post-compact.js"),
    ];

    // Read existing settings or create new
    let mut json = if settings_path.exists() {
        let content = fs::read_to_string(settings_path)
            .with_context(|| format!("Failed to read {}", settings_path.display()))?;
        serde_json::from_str::<serde_json::Value>(&content)
            .unwrap_or_else(|_| serde_json::json!({}))
    } else {
        serde_json::json!({})
    };

    let root = json
        .as_object_mut()
        .ok_or_else(|| anyhow::anyhow!("Root should be an object"))?;

    // Ensure "hooks" key exists
    let hooks_obj = root
        .entry("hooks")
        .or_insert_with(|| serde_json::json!({}))
        .as_object_mut()
        .ok_or_else(|| anyhow::anyhow!("hooks should be an object"))?;

    for (event, hook_file) in hook_defs {
        let hook_path = hooks_dir.join(hook_file);

        // Build the hook command
        let command = format!("node \"{}\"", hook_path.display());

        // Build hook entry: [{ "hooks": [{ "type": "command", "command": "..." }] }]
        let hook_entry = serde_json::json!({
            "hooks": [{
                "type": "command",
                "command": command
            }]
        });

        // Get or create the event array
        let event_arr = hooks_obj
            .entry(*event)
            .or_insert_with(|| serde_json::json!([]))
            .as_array_mut()
            .ok_or_else(|| anyhow::anyhow!("{} should be an array", event))?;

        // Check if masday hook already exists for this event
        let exists = event_arr.iter().any(|entry| {
            if let Some(hooks) = entry.get("hooks").and_then(|h| h.as_array()) {
                hooks.iter().any(|h| {
                    h.get("command")
                        .and_then(|c| c.as_str())
                        .map(|c| c.contains(hook_file))
                        .unwrap_or(false)
                })
            } else {
                false
            }
        });

        if !exists {
            event_arr.push(hook_entry);
        }
    }

    // Write back
    let content = serde_json::to_string_pretty(&json).context("Failed to serialize settings")?;
    if let Some(parent) = settings_path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(settings_path, content)
        .with_context(|| format!("Failed to write {}", settings_path.display()))?;

    // Register statusline (separate field, NOT a hook event)
    let statusline_script = hooks_dir.join("masday-statusline.js");
    let statusline_command = format!("node \"{}\"", statusline_script.display());

    // Re-read settings (we just wrote it)
    let content = fs::read_to_string(settings_path)?;
    let mut json = serde_json::from_str::<serde_json::Value>(&content)
        .unwrap_or_else(|_| serde_json::json!({}));

    let root = json
        .as_object_mut()
        .ok_or_else(|| anyhow::anyhow!("Root should be an object"))?;

    // Only set statusLine if not already configured by user
    if !root.contains_key("statusLine") {
        root.insert(
            "statusLine".to_string(),
            serde_json::json!({
                "type": "command",
                "command": statusline_command
            }),
        );
        let content =
            serde_json::to_string_pretty(&json).context("Failed to serialize settings")?;
        fs::write(settings_path, content)?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_install_global_hooks() {
        let temp_dir = TempDir::new().unwrap();
        let home_dir = temp_dir.path();

        let report = install_global_hooks(home_dir).unwrap();
        assert_eq!(report.platform, "global");

        let hooks_dir = home_dir.join(".claude/hooks");
        assert!(hooks_dir.exists());
    }

    #[test]
    fn test_install_project_hooks() {
        let temp_dir = TempDir::new().unwrap();
        let project_dir = temp_dir.path();

        let report = install_project_hooks(project_dir).unwrap();
        assert_eq!(report.platform, "project");

        let hooks_dir = project_dir.join(".claude/hooks");
        assert!(hooks_dir.exists());
    }

    #[test]
    fn test_register_hooks_in_settings() {
        let temp_dir = TempDir::new().unwrap();
        let settings_path = temp_dir.path().join("settings.json");

        register_hooks_in_settings(&settings_path, temp_dir.path()).unwrap();

        let content = fs::read_to_string(&settings_path).unwrap();
        let json: serde_json::Value = serde_json::from_str(&content).unwrap();

        assert!(json["statusLine"].is_object());
        assert!(json["hooks"]["PostToolUse"].is_array());
        assert!(json["hooks"]["UserPromptSubmit"].is_array());
        assert!(json["hooks"]["SessionStart"].is_array());
        assert!(json["hooks"]["PreToolUse"].is_array());
        assert!(json["hooks"]["PreCompact"].is_array());
        assert!(json["hooks"]["PostCompact"].is_array());
    }
}
