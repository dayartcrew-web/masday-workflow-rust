use super::platform::Platform;
use super::templates;
use anyhow::{Context, Result};
use std::fs;
use std::path::Path;

pub struct SyncReport {
    pub platform: String,
    pub copied: usize,
    pub skipped: usize,
    pub warnings: Vec<String>,
}

pub fn sync_skills_to_project(
    project_dir: &Path,
    platforms: &[Platform],
    force: bool,
) -> Result<Vec<SyncReport>> {
    let skill_names = templates::extract_skill_names();
    let mut reports = Vec::new();

    for platform in platforms {
        let mut report = SyncReport {
            platform: platform.name().to_string(),
            copied: 0,
            skipped: 0,
            warnings: Vec::new(),
        };

        let target_dir = platform.project_skills_dir(project_dir);

        // OpenCode: if the global agent dir already exists, the user manages
        // opencode globally — don't pollute the project with .opencode/skills.
        if *platform == Platform::OpenCode {
            if let Some(global) = platform.global_agents_dir() {
                if global.exists() {
                    report.skipped = skill_names
                        .iter()
                        .filter(|n| n.starts_with("masday-"))
                        .count();
                    report.warnings.push(format!(
                        "opencode global agent dir exists ({}); skipping project .opencode/skills",
                        global.display()
                    ));
                    reports.push(report);
                    continue;
                }
            }
        }

        fs::create_dir_all(&target_dir)
            .with_context(|| format!("Failed to create directory {}", target_dir.display()))?;

        for skill_name in &skill_names {
            if !skill_name.starts_with("masday-") {
                report.skipped += 1;
                continue;
            }

            let skill_files = templates::extract_skill_files(skill_name);
            let skill_target_dir = target_dir.join(skill_name);

            if !force && skill_target_dir.exists() {
                report.skipped += 1;
                continue;
            }

            fs::create_dir_all(&skill_target_dir).with_context(|| {
                format!(
                    "Failed to create skill directory {}",
                    skill_target_dir.display()
                )
            })?;

            let mut copied_skill = false;
            for (file_name, content) in skill_files.iter() {
                let file_path = skill_target_dir.join(file_name);
                if let Some(parent) = file_path.parent() {
                    fs::create_dir_all(parent).with_context(|| {
                        format!("Failed to create directory {}", parent.display())
                    })?;
                }
                fs::write(&file_path, content).with_context(|| {
                    format!("Failed to write skill file {}", file_path.display())
                })?;
                copied_skill = true;
            }

            if copied_skill {
                report.copied += 1;
            }
        }

        reports.push(report);
    }

    Ok(reports)
}

pub fn sync_skills_to_global(platforms: &[Platform], _force: bool) -> Result<Vec<SyncReport>> {
    // Masday skills are project-scoped — do not install to global.
    // Global skill dirs should only contain non-masday skills (bmad, etc).
    // This avoids duplicate skills when Claude Code loads both global + project.
    let skill_names = templates::extract_skill_names();
    let mut reports = Vec::new();

    for platform in platforms {
        let report = SyncReport {
            platform: platform.name().to_string(),
            copied: 0,
            skipped: skill_names.len(),
            warnings: vec!["Masday skills are project-scoped; skipped global install.".to_string()],
        };

        let _global_dir = platform.global_skills_dir();
        reports.push(report);
    }

    Ok(reports)
}

/// Legacy implementation kept for reference (unused).
#[allow(dead_code)]
fn _sync_skills_to_global_legacy(platforms: &[Platform], force: bool) -> Result<Vec<SyncReport>> {
    let skill_names = templates::extract_skill_names();
    let mut reports = Vec::new();

    for platform in platforms {
        let mut report = SyncReport {
            platform: platform.name().to_string(),
            copied: 0,
            skipped: 0,
            warnings: Vec::new(),
        };

        let global_dir = if let Some(dir) = platform.global_skills_dir() {
            dir
        } else {
            report.skipped = skill_names.len();
            reports.push(report);
            continue;
        };

        for skill_name in &skill_names {
            if !skill_name.starts_with("masday-") {
                report.skipped += 1;
                continue;
            }

            let skill_target_dir = global_dir.join(skill_name);

            if !force && skill_target_dir.exists() {
                report.skipped += 1;
                continue;
            }

            let skill_files = templates::extract_skill_files(skill_name);

            if !is_dir_writable(&global_dir) {
                report
                    .warnings
                    .push(format!("Directory not writable: {}", global_dir.display()));
                report.skipped += 1;
                continue;
            }

            if let Err(e) = fs::create_dir_all(&skill_target_dir) {
                report.warnings.push(format!(
                    "Cannot create {}: {}",
                    skill_target_dir.display(),
                    e
                ));
                report.skipped += 1;
                continue;
            }

            let mut copied_skill = false;
            for (file_name, content) in skill_files.iter() {
                let file_path = skill_target_dir.join(file_name);
                if let Some(parent) = file_path.parent() {
                    if let Err(e) = fs::create_dir_all(parent) {
                        report
                            .warnings
                            .push(format!("Cannot create {}: {}", parent.display(), e));
                        continue;
                    }
                }
                if let Err(e) = fs::write(&file_path, content) {
                    report
                        .warnings
                        .push(format!("Cannot write {}: {}", file_path.display(), e));
                    report.skipped += 1;
                    continue;
                }
                copied_skill = true;
            }

            if copied_skill {
                report.copied += 1;
            }
        }

        reports.push(report);
    }

    Ok(reports)
}

fn is_dir_writable(dir: &Path) -> bool {
    if !dir.exists() {
        if let Some(parent) = dir.parent() {
            return parent.exists() && parent.metadata().ok().map(|m| m.is_dir()).unwrap_or(false);
        }
        return false;
    }

    !dir.metadata()
        .ok()
        .map(|m| m.permissions().readonly())
        .unwrap_or(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_sync_skills_to_project() {
        let temp_dir = TempDir::new().unwrap();
        let project_dir = temp_dir.path();

        let platforms = vec![Platform::ClaudeCode];
        let reports = sync_skills_to_project(project_dir, &platforms, true).unwrap();

        assert!(!reports.is_empty());
        let report = &reports[0];
        assert_eq!(report.platform, "claude-code");
    }

    #[test]
    fn test_is_dir_writable() {
        let temp_dir = TempDir::new().unwrap();
        assert!(is_dir_writable(temp_dir.path()));
    }

    #[test]
    fn test_is_dir_writable_nonexistent() {
        let temp_dir = TempDir::new().unwrap();
        let nonexistent = temp_dir.path().join("nonexistent/subdir");
        assert!(!is_dir_writable(&nonexistent));
    }
}
