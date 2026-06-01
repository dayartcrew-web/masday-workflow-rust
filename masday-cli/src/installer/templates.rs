//! Template embedding and extraction
//!
//! This module uses `include_dir!` to embed Masday templates at compile time,
//! allowing the CLI to distribute agents, skills, and hooks as a standalone binary.

use include_dir::{include_dir, Dir};
use std::path::Path;

// Embed the entire templates directory
// This is populated by build.rs which copies from source locations
static TEMPLATES: Dir = include_dir!("$OUT_DIR/templates");

/// Get a reference to the embedded templates directory
pub fn get_templates() -> &'static Dir<'static> {
    &TEMPLATES
}

/// Extract all agent .md files as (filename, content) tuples
pub fn extract_agents() -> Vec<(String, String)> {
    let mut agents = Vec::new();

    if let Some(agents_dir) = TEMPLATES.get_dir("agents") {
        for entry in agents_dir.entries() {
            if let Some(file) = entry.as_file() {
                if let Some(name) = file.path().file_name().and_then(|n| n.to_str()) {
                    if let Some(content_utf8) = file.contents_utf8() {
                        agents.push((name.to_string(), content_utf8.to_string()));
                    }
                }
            }
        }
    }

    agents.sort_by(|a, b| a.0.cmp(&b.0));
    agents
}

/// Extract all skill directory names
pub fn extract_skill_names() -> Vec<String> {
    let mut skill_names = Vec::new();

    if let Some(skills_dir) = TEMPLATES.get_dir("skills") {
        for entry in skills_dir.entries() {
            if entry.as_dir().is_some() {
                if let Some(name) = entry.path().file_name().and_then(|n| n.to_str()) {
                    skill_names.push(name.to_string());
                }
            }
        }
    }

    skill_names.sort();
    skill_names
}

/// Extract all files for a specific skill as (relative_path, content) tuples
pub fn extract_skill_files(name: &str) -> Vec<(String, String)> {
    let mut files = Vec::new();
    let skill_path = Path::new("skills").join(name);

    if let Some(skill_dir) = TEMPLATES.get_dir(&skill_path) {
        extract_files_recursive(skill_dir, &skill_path, &mut files);
    }

    files.sort_by(|a, b| a.0.cmp(&b.0));
    files
}

/// Extract all global hook files as (filename, content) tuples
pub fn extract_global_hooks() -> Vec<(String, String)> {
    let mut hooks = Vec::new();

    if let Some(hooks_dir) = TEMPLATES.get_dir("global-hooks") {
        for entry in hooks_dir.entries() {
            if let Some(file) = entry.as_file() {
                if let Some(name) = file.path().file_name().and_then(|n| n.to_str()) {
                    if let Some(content_utf8) = file.contents_utf8() {
                        hooks.push((name.to_string(), content_utf8.to_string()));
                    }
                }
            }
        }
    }

    hooks.sort_by(|a, b| a.0.cmp(&b.0));
    hooks
}

/// Extract all project hook files as (filename, content) tuples
pub fn extract_project_hooks() -> Vec<(String, String)> {
    let mut hooks = Vec::new();

    if let Some(hooks_dir) = TEMPLATES.get_dir("project-hooks") {
        for entry in hooks_dir.entries() {
            if let Some(file) = entry.as_file() {
                if let Some(name) = file.path().file_name().and_then(|n| n.to_str()) {
                    if let Some(content_utf8) = file.contents_utf8() {
                        hooks.push((name.to_string(), content_utf8.to_string()));
                    }
                }
            }
        }
    }

    hooks.sort_by(|a, b| a.0.cmp(&b.0));
    hooks
}

/// Helper to recursively extract files from a directory
fn extract_files_recursive(dir: &Dir, base_path: &Path, output: &mut Vec<(String, String)>) {
    for entry in dir.entries() {
        let entry_path = entry.path();

        if let Some(sub_dir) = entry.as_dir() {
            extract_files_recursive(sub_dir, base_path, output);
        } else if let Some(file) = entry.as_file() {
            if let Some(content_utf8) = file.contents_utf8() {
                // Get relative path from base
                let relative = match entry_path.strip_prefix(base_path) {
                    Ok(p) => p.to_str().unwrap_or(""),
                    Err(_) => "",
                }.to_string();

                // Convert to forward slashes for consistency
                let normalized = relative.replace('\\', "/");
                output.push((normalized, content_utf8.to_string()));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_templates() {
        let templates = get_templates();
        // Should have at least some directories
        let entry_count = templates.entries().len();
        assert!(entry_count > 0, "Templates directory should not be empty");
    }

    #[test]
    fn test_extract_agents() {
        let agents = extract_agents();
        // Should have some agent files (at least masday-executor.md)
        assert!(!agents.is_empty(), "Should have at least one agent");

        // Check format
        for (name, content) in &agents {
            assert!(!name.is_empty(), "Agent name should not be empty");
            assert!(!content.is_empty(), "Agent content should not be empty");
            assert!(
                name.ends_with(".md"),
                "Agent file should be .md: {}",
                name
            );
        }
    }

    #[test]
    fn test_extract_skill_names() {
        let skills = extract_skill_names();
        // Should have skill directories
        assert!(!skills.is_empty(), "Should have at least one skill");

        // All should start with "masday-"
        for name in &skills {
            assert!(
                name.starts_with("masday-"),
                "Skill should start with 'masday-': {}",
                name
            );
        }
    }

    #[test]
    fn test_extract_skill_files() {
        // Test with a known skill
        let skill_names = extract_skill_names();
        if let Some(first_skill) = skill_names.first() {
            let files = extract_skill_files(first_skill);

            // Should have at least SKILL.md or similar
            assert!(!files.is_empty(), "Skill '{}' should have files", first_skill);

            // Check format
            for (path, content) in &files {
                assert!(!path.is_empty(), "Skill file path should not be empty");
                assert!(!content.is_empty(), "Skill file content should not be empty");
            }
        }
    }

    #[test]
    fn test_extract_global_hooks() {
        let hooks = extract_global_hooks();

        // Should have global hooks (masday-statusline.js, etc.)
        assert!(!hooks.is_empty(), "Should have at least one global hook");

        // Check that we have expected hooks
        let hook_names: Vec<&str> = hooks.iter().map(|(name, _)| name.as_str()).collect();
        assert!(
            hook_names.iter().any(|n| n.contains("statusline") || n.contains("session")),
            "Should have statusline or session hook"
        );
    }

    #[test]
    fn test_extract_project_hooks() {
        let hooks = extract_project_hooks();

        // Should have project hooks (skill-step-guard.cjs, etc.)
        assert!(!hooks.is_empty(), "Should have at least one project hook");

        // Check format
        for (name, content) in &hooks {
            assert!(!name.is_empty(), "Hook name should not be empty");
            assert!(!content.is_empty(), "Hook content should not be empty");

            let is_js_or_cjs = name.ends_with(".js") || name.ends_with(".cjs");
            let is_run_sh = name == "run.sh";
            assert!(
                is_js_or_cjs || is_run_sh,
                "Hook should be .js, .cjs, or run.sh: {}",
                name
            );
        }
    }

    #[test]
    fn test_agents_sorted() {
        let agents = extract_agents();
        let mut sorted = agents.clone();
        sorted.sort_by(|a, b| a.0.cmp(&b.0));
        assert_eq!(agents, sorted, "Agents should be sorted alphabetically");
    }

    #[test]
    fn test_skill_names_sorted() {
        let skills = extract_skill_names();
        let mut sorted = skills.clone();
        sorted.sort();
        assert_eq!(skills, sorted, "Skill names should be sorted alphabetically");
    }
}
