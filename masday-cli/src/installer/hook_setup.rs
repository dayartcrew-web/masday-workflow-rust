use std::fs;
use std::path::Path;
use anyhow::{Result, Context};
use super::templates;

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
        let hook_path = hooks_dir.join(&name);
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
        let hook_path = hooks_dir.join(&name);
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
        let hook_path = hooks_dir.join(&name);
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
        let hook_path = hooks_dir.join(&name);
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
    fn test_uninstall_project_hooks() {
        let temp_dir = TempDir::new().unwrap();
        let project_dir = temp_dir.path();

        install_project_hooks(project_dir).unwrap();
        let report = uninstall_project_hooks(project_dir).unwrap();

        assert_eq!(report.platform, "project");
    }
}
