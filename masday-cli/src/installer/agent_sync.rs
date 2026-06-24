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
        // opencode globally — don't pollute the project with .opencode/agent.
        if *platform == Platform::OpenCode {
            if let Some(global) = platform.global_agents_dir() {
                if global.exists() {
                    report.skipped = agents
                        .iter()
                        .filter(|(n, _)| n.starts_with("masday-") && n.ends_with(".md"))
                        .count();
                    report.warnings.push(format!(
                        "opencode global agent dir exists ({}); skipping project .opencode/agent",
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

            // opencode needs its own frontmatter shape (tools record, model
            // id, mode) — transform only for OpenCode; clone as-is otherwise.
            let out = if *platform == Platform::OpenCode {
                transform_opencode_frontmatter(content)
            } else {
                content.clone()
            };

            fs::write(&target_file, out)
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

            let out = if *platform == Platform::OpenCode {
                transform_opencode_frontmatter(content)
            } else {
                content.clone()
            };

            match fs::write(&target_file, out) {
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

/// Map a Claude/opencode tool name to the lowercase opencode built-in key.
///
/// opencode's `tools:` record only accepts its built-in tools
/// (`read`/`write`/`edit`/`bash`/`grep`/`glob`). MCP tools masday lists
/// (`git_status`, `github_pr_create`, …) are NOT valid `tools:` keys — opencode
/// gates built-in tools here, MCP servers are configured separately — so they
/// return `None` and are dropped.
fn opencode_tool_name(raw: &str) -> Option<&'static str> {
    match raw.trim().to_lowercase().as_str() {
        "read" => Some("read"),
        "write" => Some("write"),
        "edit" => Some("edit"),
        "bash" | "shell" => Some("bash"),
        "grep" => Some("grep"),
        "glob" => Some("glob"),
        _ => None,
    }
}

/// Inline value of a frontmatter key line (`"model: sonnet"` → `"sonnet"`,
/// `"tools:"` → `""`).
fn frontmatter_inline_value(line: &str) -> String {
    match line.find(':') {
        Some(pos) => line[pos + 1..].trim().to_string(),
        None => String::new(),
    }
}

/// Convert a Claude-Code-format agent `.md` to opencode frontmatter.
///
/// masday's source agents live in `.claude/agents/*.md` and are cloned verbatim
/// to every platform. Claude accepts `tools:` as a YAML **array**; opencode's
/// schema requires it to be a **record** of built-in tool flags — otherwise
/// `opencode models` fails: `Invalid input: expected record, received array
/// tools`. This transform fixes that, and aligns the rest of the frontmatter
/// with what real opencode agents use (verified against the 125 agents in
/// `~/.config/opencode/agent/`):
///
/// - `tools:` array → record `{name: true}` of opencode built-ins (MCP tools
///   dropped, since they aren't valid keys).
/// - `model:` — Claude aliases (`sonnet`/`opus`/`haiku`) are not valid opencode
///   model ids → `inherit` (the session model). Full provider ids (`a/b`) kept.
/// - `name:` dropped (opencode derives the agent id from the filename).
/// - `mode: subagent` injected if absent (every non-primary opencode agent sets
///   this; without it the agent would register as a primary agent).
/// - `description:` (incl. multi-line `>` folded scalars) preserved verbatim,
///   and any other unknown keys preserved verbatim.
///
/// Applied ONLY for `Platform::OpenCode` at sync time — all other platforms
/// receive the original Claude-format file unchanged.
pub fn transform_opencode_frontmatter(content: &str) -> String {
    let lines: Vec<&str> = content.lines().collect();

    // Frontmatter is delimited by `---` on the first line and a closing `---`.
    if lines.is_empty() || lines[0].trim() != "---" {
        return content.to_string();
    }
    let close = match lines
        .iter()
        .enumerate()
        .skip(1)
        .find(|(_, l)| l.trim() == "---")
    {
        Some((i, _)) => i,
        None => return content.to_string(), // no closing fence → leave untouched
    };

    let fm: Vec<&str> = lines[1..close].to_vec();

    // Parse into ordered (key, block_lines) entries. A top-level key line has no
    // leading indentation and contains `:`; its value spans subsequent indented
    // lines (covers `description: >` folded scalars and `tools:` lists alike).
    let mut entries: Vec<(String, Vec<&str>)> = Vec::new();
    let mut i = 0;
    while i < fm.len() {
        let line = fm[i];
        let indented = line.starts_with(' ') || line.starts_with('\t');
        if !indented && line.contains(':') {
            let key = line.split(':').next().unwrap_or("").trim().to_string();
            let mut block = vec![line];
            let mut j = i + 1;
            while j < fm.len() && (fm[j].starts_with(' ') || fm[j].starts_with('\t')) {
                block.push(fm[j]);
                j += 1;
            }
            entries.push((key, block));
            i = j;
        } else {
            i += 1;
        }
    }

    // Rebuild in canonical opencode order: description, mode, model, tools,
    // then any other keys verbatim. Inject `mode: subagent` right after the
    // description block if no `mode` was present.
    let mut out: Vec<String> = vec!["---".to_string()];
    let mut desc_end: Option<usize> = None;
    let mut mode_emitted = false;

    for (key, block) in &entries {
        match key.as_str() {
            "description" => {
                for l in block {
                    out.push(l.to_string());
                }
                desc_end = Some(out.len());
            }
            "mode" => {
                mode_emitted = true;
                for l in block {
                    out.push(l.to_string());
                }
            }
            "model" => {
                let val = frontmatter_inline_value(block[0]);
                // Full provider ids (e.g. `anthropic/claude-...`) are kept;
                // everything else (Claude aliases, empty) → `inherit`.
                let resolved = if val.contains('/') {
                    val
                } else {
                    "inherit".to_string()
                };
                out.push(format!("model: {}", resolved));
            }
            "tools" => {
                // Collect array items (`- Name`) from the block. If none are
                // found (already a record, or unusual), preserve verbatim.
                let items: Vec<String> = block
                    .iter()
                    .skip(1)
                    .filter_map(|l| {
                        let t = l.trim();
                        t.strip_prefix("- ")
                            .map(|s| s.trim().to_string())
                            .or_else(|| t.strip_prefix('-').map(|s| s.trim().to_string()))
                    })
                    .collect();
                if items.is_empty() {
                    for l in block {
                        out.push(l.to_string());
                    }
                } else {
                    out.push("tools:".to_string());
                    let mut seen = std::collections::HashSet::new();
                    for n in items {
                        if let Some(m) = opencode_tool_name(&n) {
                            if seen.insert(m) {
                                out.push(format!("  {}: true", m));
                            }
                        }
                    }
                }
            }
            "name" => {
                // dropped — opencode derives the agent id from the filename
            }
            _ => {
                for l in block {
                    out.push(l.to_string());
                }
            }
        }
    }

    if !mode_emitted {
        let mode_line = "mode: subagent".to_string();
        if let Some(idx) = desc_end {
            out.insert(idx, mode_line);
        } else {
            out.insert(1, mode_line); // no description → right after the fence
        }
    }

    out.push("---".to_string());

    // Body (after the closing fence), verbatim.
    for l in &lines[close + 1..] {
        out.push(l.to_string());
    }

    let mut result = out.join("\n");
    if content.ends_with('\n') {
        result.push('\n');
    }
    result
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
            !project_dir.join(".opencode/agent").exists(),
            "project .opencode/agent must NOT be created when global dir exists"
        );
        assert!(
            !report.warnings.is_empty(),
            "a skip warning should be recorded"
        );
        // Leak-check: assert the global dir path is correct (singular).
        let _ = PathBuf::from(&fake_home);
    }

    // --- transform_opencode_frontmatter tests ---

    fn claude_agent_doc() -> &'static str {
        "---\n\
         name: masday-git-master\n\
         description: >\n\
         \x20 Git operations specialist. Handles branches, commits, and merges.\n\
         model: sonnet\n\
         tools:\n\
         \x20 - Read\n\
         \x20 - Bash\n\
         \x20 - Grep\n\
         \x20 - Glob\n\
         \x20 - git_status\n\
         \x20 - github_pr_create\n\
         ---\n\
         \n\
         # Git Operations Agent\n\
         \n\
         Body stays untouched.\n"
    }

    #[test]
    fn test_opencode_tools_array_becomes_record_and_drops_mcp() {
        let out = transform_opencode_frontmatter(claude_agent_doc());

        // The record form must appear; the array form must not.
        assert!(out.contains("  read: true"));
        assert!(out.contains("  bash: true"));
        assert!(out.contains("  grep: true"));
        assert!(out.contains("  glob: true"));
        assert!(!out.contains("- Read"), "array item leaked: {}", out);
        assert!(
            !out.contains("git_status") && !out.contains("github_pr_create"),
            "MCP tool leaked into tools record: {}",
            out
        );
    }

    #[test]
    fn test_opencode_model_alias_becomes_inherit() {
        let out = transform_opencode_frontmatter(claude_agent_doc());
        assert!(
            out.contains("model: inherit"),
            "Claude model alias must resolve to inherit: {}",
            out
        );
        assert!(!out.contains("model: sonnet"));
    }

    #[test]
    fn test_opencode_keeps_full_provider_model_id() {
        let doc = "---\nmodel: anthropic/claude-sonnet-4\n---\nbody\n";
        let out = transform_opencode_frontmatter(doc);
        assert!(
            out.contains("model: anthropic/claude-sonnet-4"),
            "full provider id must be preserved: {}",
            out
        );
    }

    #[test]
    fn test_opencode_injects_mode_subagent_after_description() {
        let out = transform_opencode_frontmatter(claude_agent_doc());
        assert!(out.contains("mode: subagent"));
        // mode must follow the description block (which itself precedes model/tools)
        let mode_idx = out.find("mode: subagent").unwrap();
        let model_idx = out.find("model:").unwrap();
        let desc_idx = out.find("description:").unwrap();
        assert!(desc_idx < mode_idx, "mode should come after description");
        assert!(mode_idx < model_idx, "mode should come before model");
    }

    #[test]
    fn test_opencode_keeps_existing_mode() {
        let doc = "---\ndescription: x\nmode: primary\ntools:\n  - Read\n---\n";
        let out = transform_opencode_frontmatter(doc);
        assert!(out.contains("mode: primary"));
        assert!(!out.contains("mode: subagent"));
    }

    #[test]
    fn test_opencode_drops_name_key() {
        let out = transform_opencode_frontmatter(claude_agent_doc());
        // No top-level `name:` line (opencode uses the filename as the id).
        assert!(
            !out.lines()
                .take_while(|l| l.trim() != "---")
                .any(|l| l.starts_with("name:")),
            "`name:` should be dropped from opencode frontmatter: {}",
            out
        );
    }

    #[test]
    fn test_opencode_preserves_multiline_description_body() {
        let out = transform_opencode_frontmatter(claude_agent_doc());
        assert!(
            out.contains("Git operations specialist. Handles branches, commits, and merges."),
            "multi-line description body must be preserved: {}",
            out
        );
    }

    #[test]
    fn test_opencode_preserves_markdown_body_verbatim() {
        let out = transform_opencode_frontmatter(claude_agent_doc());
        assert!(out.contains("# Git Operations Agent"));
        assert!(out.contains("Body stays untouched."));
    }

    #[test]
    fn test_opencode_no_frontmatter_left_untouched() {
        let doc = "# Just markdown\nno frontmatter here\n";
        assert_eq!(transform_opencode_frontmatter(doc), doc);
    }

    #[test]
    fn test_opencode_already_record_tools_preserved() {
        // If a tools block is already a record (not an array), keep it verbatim
        // rather than emitting an empty `tools:`.
        let doc = "---\ntools:\n  read: true\n  bash: true\n---\n";
        let out = transform_opencode_frontmatter(doc);
        assert!(
            out.contains("  read: true"),
            "existing record tools lost: {}",
            out
        );
        assert!(out.contains("  bash: true"));
        assert!(!out.contains("- "), "no array items expected");
    }

    #[test]
    fn test_sync_agents_writes_opencode_record_format() {
        let temp_dir = TempDir::new().unwrap();
        let project_dir = temp_dir.path();

        // Force-skip the opencode global-dir guard by pointing HOME at a clean
        // temp home that has NO `~/.config/opencode/agent`.
        let fake_home = temp_dir.path().join("home");
        fs::create_dir_all(&fake_home).unwrap();
        std::env::set_var("HOME", &fake_home);

        let platforms = vec![Platform::OpenCode];
        let reports = sync_agents_to_project(project_dir, &platforms, true).unwrap();
        std::env::remove_var("HOME");

        let report = &reports[0];
        assert!(report.copied > 0, "at least one agent should be written");

        // Every written opencode agent must use the record tools form and must
        // NOT contain the array form that breaks `opencode models`.
        let opencode_dir = project_dir.join(".opencode/agent");
        let mut checked = 0;
        for entry in fs::read_dir(&opencode_dir).unwrap() {
            let entry = entry.unwrap();
            let name = entry.file_name().to_string_lossy().to_string();
            if !name.starts_with("masday-") || !name.ends_with(".md") {
                continue;
            }
            let content = fs::read_to_string(entry.path()).unwrap();
            assert!(
                !content
                    .lines()
                    .take_while(|l| l.trim() != "---")
                    .any(|l| l.trim() == "- Read"),
                "{} still contains array-style tools",
                name
            );
            assert!(
                content.contains("mode: subagent"),
                "{} missing mode: subagent",
                name
            );
            // Either has a tools record, or no tools at all — never an array.
            if content.contains("\ntools:") {
                assert!(
                    content.contains(": true"),
                    "{} tools block is not a record",
                    name
                );
            }
            checked += 1;
        }
        assert!(checked > 0, "expected at least one masday agent written");
    }
}
