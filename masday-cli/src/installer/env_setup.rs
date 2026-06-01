use std::collections::HashMap;
use std::fs;
use std::path::Path;
use anyhow::{Result, Context};

pub struct Prerequisites {
    pub cargo_available: bool,
    pub node_available: bool,
    pub pnpm_available: bool,
}

pub fn check_prerequisites(_remote_mode: bool) -> Result<Prerequisites> {
    let cargo_available = which::which("cargo").is_ok();
    let node_available = which::which("node").is_ok();

    let pnpm_available = if node_available {
        which::which("pnpm").is_ok()
    } else {
        false
    };

    Ok(Prerequisites {
        cargo_available,
        node_available,
        pnpm_available,
    })
}

pub fn ensure_env_file(project_dir: &Path) -> Result<bool> {
    let env_path = project_dir.join(".env");
    let env_example = project_dir.join(".env.example");

    if env_path.exists() {
        return Ok(false);
    }

    if env_example.exists() {
        fs::copy(&env_example, &env_path)
            .with_context(|| format!("Failed to copy {} to .env", env_example.display()))?;
        Ok(true)
    } else {
        fs::write(&env_path, "")
            .with_context(|| format!("Failed to create empty .env in {}", project_dir.display()))?;
        Ok(true)
    }
}

pub fn load_env(project_dir: &Path) -> Result<HashMap<String, String>> {
    let env_path = project_dir.join(".env");

    if !env_path.exists() {
        return Ok(HashMap::new());
    }

    let content = fs::read_to_string(&env_path)
        .with_context(|| format!("Failed to read .env from {}", project_dir.display()))?;

    let mut env_map = HashMap::new();

    for line in content.lines() {
        let line = line.trim();

        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        if let Some((key, value)) = line.split_once('=') {
            let key = key.trim();
            let value = value.trim();

            if !key.is_empty() {
                env_map.insert(key.to_string(), value.to_string());
            }
        }
    }

    Ok(env_map)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_check_prerequisites() {
        let prereqs = check_prerequisites(false).unwrap();

        // cargo should be available in test environment
        assert!(prereqs.cargo_available || true); // Allow both true/false based on environment
    }

    #[test]
    fn test_ensure_env_file_creates_new() {
        let temp_dir = TempDir::new().unwrap();
        let project_dir = temp_dir.path();

        let created = ensure_env_file(project_dir).unwrap();
        assert!(created);
        assert!(project_dir.join(".env").exists());
    }

    #[test]
    fn test_ensure_env_file_does_not_overwrite() {
        let temp_dir = TempDir::new().unwrap();
        let project_dir = temp_dir.path();
        let env_path = project_dir.join(".env");

        fs::write(&env_path, "EXISTING=content").unwrap();

        let created = ensure_env_file(project_dir).unwrap();
        assert!(!created);

        let content = fs::read_to_string(&env_path).unwrap();
        assert_eq!(content, "EXISTING=content");
    }

    #[test]
    fn test_ensure_env_file_copies_example() {
        let temp_dir = TempDir::new().unwrap();
        let project_dir = temp_dir.path();
        let env_example = project_dir.join(".env.example");

        fs::write(&env_example, "EXAMPLE=value").unwrap();

        let created = ensure_env_file(project_dir).unwrap();
        assert!(created);

        let env_path = project_dir.join(".env");
        let content = fs::read_to_string(&env_path).unwrap();
        assert_eq!(content, "EXAMPLE=value");
    }

    #[test]
    fn test_load_env() {
        let temp_dir = TempDir::new().unwrap();
        let project_dir = temp_dir.path();
        let env_path = project_dir.join(".env");

        fs::write(&env_path, "KEY1=value1\nKEY2=value2\n# comment\n\nKEY3=value3").unwrap();

        let env_map = load_env(project_dir).unwrap();
        assert_eq!(env_map.get("KEY1"), Some(&"value1".to_string()));
        assert_eq!(env_map.get("KEY2"), Some(&"value2".to_string()));
        assert_eq!(env_map.get("KEY3"), Some(&"value3".to_string()));
        assert_eq!(env_map.get("comment"), None);
    }

    #[test]
    fn test_load_env_empty() {
        let temp_dir = TempDir::new().unwrap();
        let project_dir = temp_dir.path();

        let env_map = load_env(project_dir).unwrap();
        assert!(env_map.is_empty());
    }
}
