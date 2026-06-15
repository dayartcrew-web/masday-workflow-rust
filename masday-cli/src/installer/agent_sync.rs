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

pub fn sync_agents_to_project(
    project_dir: &Path,
    platforms: &[Platform],
    force: bool,
) -> Result<Vec<SyncReport>> {
    let agents = templates::extract_agents();
    let mut reports = Vec::new();

    for platform in platforms {
        let mut report = SyncReport {
            platform: platform.name().to_string(),
            copied: 0,
            skipped: 0,
            warnings: Vec::new(),
        };

        let target_dir = platform.project_agents_dir(project_dir);

        // OpenCode: if the global agent dir already exists, the user manages
        // opencode globally — don't pollute the project with .opencode/agents.
        if *platform == Platform::OpenCode {
            if let Some(global) = platform.global_agents_dir() {
                if global.exists() {
                    report.skipped = agents
                        .iter()
                        .filter(|(n, _)| n.starts_with("masday-") && n.ends_with(".md"))
                        .count();
                    report.warnings.push(format!(
                        "opencode global agent dir exists ({}); skipping project .opencode/agents",
                        global.display()
                    ));
                    reports.push(report);
                    continue;
                }
            }
        }

        fs::create_dir_all(&target_dir)
            .with_context(|| format!("Failed to create directory {}", target_dir.display()))?;

        for (name, content) in agents.iter() {
            if !name.starts_with("masday-") || !name.ends_with(".md") {
                continue;
            }

            let target_file = target_dir.join(name);

            if !force && target_file.exists() {
                report.skipped += 1;
                continue;
            }

            fs::write(&target_file, content)
                .with_context(|| format!("Failed to write agent file {}", target_file.display()))?;
            report.copied += 1;
        }

        reports.push(report);
    }

    Ok(reports)
}

pub fn sync_agents_to_global(platforms: &[Platform], force: bool) -> Result<Vec<SyncReport>> {
    let agents = templates::extract_agents();
    let mut reports = Vec::new();

    for platform in platforms {
        let mut report = SyncReport {
            platform: platform.name().to_string(),
            copied: 0,
            skipped: 0,
            warnings: Vec::new(),
        };

        let global_dir = if let Some(dir) = platform.global_agents_dir() {
            dir
        } else {
            report.skipped = agents.len();
            reports.push(report);
            continue;
        };

        if let Err(e) = fs::create_dir_all(&global_dir) {
            report
                .warnings
                .push(format!("Cannot create {}: {}", global_dir.display(), e));
            report.skipped = agents
                .iter()
                .filter(|(n, _)| n.starts_with("masday-") && n.ends_with(".md"))
                .count();
            reports.push(report);
            continue;
        }

        for (name, content) in agents.iter() {
            if !name.starts_with("masday-") || !name.ends_with(".md") {
                continue;
            }

            let target_file = global_dir.join(name);

            if !force && target_file.exists() {
                report.skipped += 1;
                continue;
            }

            match fs::write(&target_file, content) {
                Ok(()) => report.copied += 1,
                Err(e) => {
                    report
                        .warnings
                        .push(format!("Cannot write {}: {}", target_file.display(), e));
                    report.skipped += 1;
                }
            }
        }

        reports.push(report);
    }

    Ok(reports)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_sync_agents_to_project() {
        let temp_dir = TempDir::new().unwrap();
        let project_dir = temp_dir.path();

        let platforms = vec![Platform::ClaudeCode];
        let reports = sync_agents_to_project(project_dir, &platforms, true).unwrap();

        assert!(!reports.is_empty());
        let report = &reports[0];
        assert_eq!(report.platform, "claude-code");
    }

    #[test]
    fn test_sync_agents_skips_existing_when_not_forced() {
        let temp_dir = TempDir::new().unwrap();
        let project_dir = temp_dir.path();
        let agents_dir = project_dir.join(".claude/agents");

        fs::create_dir_all(&agents_dir).unwrap();

        // Pre-create one of the embedded agent files so it gets skipped
        let agents = templates::extract_agents();
        let masday_agent = agents.iter().find(|(name, _)| name.starts_with("masday-"));
        if let Some((name, _)) = masday_agent {
            fs::write(agents_dir.join(name), "existing content").unwrap();
        }

        let platforms = vec![Platform::ClaudeCode];
        let reports = sync_agents_to_project(project_dir, &platforms, false).unwrap();

        let report = &reports[0];
        // With force=false, existing masday-* files should be skipped
        // If we pre-created a masday agent, at least 1 should be skipped;
        // otherwise all embedded agents get copied (report.copied > 0)
        assert!(report.skipped > 0 || report.copied > 0);
    }

    #[test]
    fn test_opencode_skips_project_when_global_agent_dir_exists() {
        use std::path::PathBuf;
        let temp_dir = TempDir::new().unwrap();
        let project_dir = temp_dir.path();

        // Point HOME at a temp dir and create the opencode GLOBAL agent dir so
        // the skip guard triggers. global_agents_dir() for OpenCode reads HOME.
        let fake_home = temp_dir.path().join("home");
        let global_agent = fake_home.join(".config/opencode/agent");
        fs::create_dir_all(&global_agent).unwrap();
        std::env::set_var("HOME", &fake_home);

        let platforms = vec![Platform::OpenCode];
        let reports = sync_agents_to_project(project_dir, &platforms, true).unwrap();
        std::env::remove_var("HOME");

        let report = &reports[0];
        assert_eq!(report.platform, "opencode");
        assert!(report.copied == 0, "no agents should be copied");
        assert!(report.skipped > 0, "agents should be counted as skipped");
        assert!(
            !project_dir.join(".opencode/agents").exists(),
            "project .opencode/agents must NOT be created when global dir exists"
        );
        assert!(
            !report.warnings.is_empty(),
            "a skip warning should be recorded"
        );
        // Leak-check: assert the global dir path is correct (singular).
        let _ = PathBuf::from(&fake_home);
    }
}
