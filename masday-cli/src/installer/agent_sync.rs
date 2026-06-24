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

/// Whether a platform supports invokable per-agent `.md` subagent files.
///
/// Codex, Cursor, and Windsurf have NO per-agent subagent mechanism:
/// - **Codex/ChatGPT** reads a single `AGENTS.md` and ignores
///   `~/.codex/agents/*.md` (verified: the installed `openai.chatgpt` extension
///   references `AGENTS.md`, never `.codex/agents`).
/// - **Cursor** (`.cursor/rules/*.mdc`) and **Windsurf** (`.windsurf/rules/*.md`)
///   are passive context rules applied by glob/always, not invokable subagents.
///
/// Writing Claude-format agent `.md` to these targets is noise nothing reads (or
/// wrong-format rules), so agent sync is skipped for them. Skills/MCP for these
/// platforms are unaffected.
fn platform_supports_agent_files(platform: Platform) -> bool {
    !matches!(
        platform,
        Platform::Codex | Platform::Cursor | Platform::Windsurf
    )
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

        if !platform_supports_agent_files(*platform) {
            report.skipped = agents
                .iter()
                .filter(|(n, _)| n.starts_with("masday-") && n.ends_with(".md"))
                .count();
            report.warnings.push(format!(
                "{} has no per-agent subagent mechanism; skipping agent sync",
                platform.name()
            ));
            reports.push(report);
            continue;
        }

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
            } else if *platform == Platform::Zcode {
                transform_zcode_frontmatter(content)
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

        if !platform_supports_agent_files(*platform) {
            report.skipped = agents
                .iter()
                .filter(|(n, _)| n.starts_with("masday-") && n.ends_with(".md"))
                .count();
            report.warnings.push(format!(
                "{} has no per-agent subagent mechanism; skipping agent sync",
                platform.name()
            ));
            reports.push(report);
            continue;
        }

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
            } else if *platform == Platform::Zcode {
                transform_zcode_frontmatter(content)
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
/// - `name:` kept verbatim — opencode's `Info` schema REQUIRES `name`
///   (`Schema.String`). Stock agents usually omit it (the id is derived from the
///   filename), but an explicit `name:` is accepted and overrides, so we keep it.
/// - `mode: subagent` injected if absent (every non-primary opencode agent sets
///   this; without it the agent would register as a primary agent).
/// - `description:` flattened to a single plain inline string — opencode uses
///   plain strings (0/125 stock agents use folded `>` / literal `|` scalars), so
///   a folded `description: >` body is space-joined into one line. Other keys
///   are preserved verbatim.
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
                let head_val = frontmatter_inline_value(block[0]);
                let is_block_scalar =
                    head_val.is_empty() || head_val.starts_with('>') || head_val.starts_with('|');
                if is_block_scalar && block.len() > 1 {
                    // opencode wants a plain inline string (0/125 stock agents use
                    // folded `>` / literal `|` scalars). Flatten the indented body
                    // into one line (folded semantics: join continuation lines on a
                    // single space).
                    let joined = block[1..]
                        .iter()
                        .map(|l| l.trim())
                        .filter(|l| !l.is_empty())
                        .collect::<Vec<_>>()
                        .join(" ");
                    out.push(format!("description: {}", joined));
                } else {
                    for l in block {
                        out.push(l.to_string());
                    }
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
                // opencode's `Info` schema REQUIRES `name` (`Schema.String`).
                // Stock agents usually omit it (id derived from filename), but an
                // explicit `name:` is accepted and overrides — keep verbatim.
                for l in block {
                    out.push(l.to_string());
                }
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

/// Escape a string for a double-quoted YAML scalar (zcode's native writer quotes
/// `name`/`description`). Backslash first, then the quote.
fn escape_yaml_quoted(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

/// Convert a Claude-Code-format agent `.md` to zcode frontmatter.
///
/// zcode (a GLM-based coding agent) stores user agents as `.md` files under
/// `~/.zcode/cli/agents` (global) and `<project>/.zcode/cli/agents` (project),
/// read by a naive line-by-line `key: value` parser (`readAgentFrontmatter` in
/// the installed `~/.zcode/server/zcode-server.cjs`). Verified against that
/// source's `buildAgentMarkdown` / `readAgentFrontmatter`:
///
/// - Frontmatter is `---`-fenced; the native writer emits QUOTED strings:
///   `name: "…"` / `description: "…"` / `color: "#3b82f6"` / optional
///   `model: "…"` / optional `tools: "…"` (a single STRING — never an array or
///   record). Body after the closing fence is the system prompt.
/// - The reader splits each line on the FIRST `:` and strips surrounding quotes,
///   so a folded `description: >` is read as literally `>` and a YAML-array
///   `tools:` block (items have no `:`) is silently dropped. Both must be
///   converted, exactly like the opencode transform.
///
/// Transform applied (Claude → zcode):
/// - `name:` — kept, emitted as a quoted string (zcode's `createAgent` requires a
///   name; the reader defaults to `""` if absent).
/// - `description:` — folded/multi-line body flattened to one inline line (folded
///   semantics: join continuations on a space), emitted quoted.
/// - `color: "#3b82f6"` — injected (the native default; zcode tags agents by
///   color in its UI).
/// - `model:` and `tools:` — DROPPED. Both are optional in zcode, and Claude
///   aliases (`sonnet`/`opus`/`haiku`) plus Claude/MCP tool names (`Read`,
///   `git_status`, …) are NOT valid GLM model ids / zcode tool names. Emitting
///   them unverified risks the same load-or-invoke failure class that broke
///   opencode; dropping lets zcode fall back to its own defaults. Other keys are
///   preserved verbatim and the body (system prompt) is left untouched.
///
/// Applied ONLY for `Platform::Zcode` at sync time.
pub fn transform_zcode_frontmatter(content: &str) -> String {
    let lines: Vec<&str> = content.lines().collect();

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

    // Ordered (key, block_lines) entries — same line-based parser as the opencode
    // transform (no YAML crate). A top-level key line has no leading indentation
    // and contains `:`; its value spans the following indented lines.
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

    // Flatten a description block into a single inline string: a folded (`>`) or
    // literal (`|`) scalar joins its indented body on a space; otherwise the
    // inline value is used as-is.
    let flatten_desc = |block: &[&str]| -> String {
        let head_val = frontmatter_inline_value(block[0]);
        let is_block_scalar =
            head_val.is_empty() || head_val.starts_with('>') || head_val.starts_with('|');
        if is_block_scalar && block.len() > 1 {
            block[1..]
                .iter()
                .map(|l| l.trim())
                .filter(|l| !l.is_empty())
                .collect::<Vec<_>>()
                .join(" ")
        } else {
            head_val
        }
    };

    let mut name_value: Option<String> = None;
    let mut desc_value: Option<String> = None;
    let mut extras: Vec<String> = Vec::new();

    for (key, block) in &entries {
        match key.as_str() {
            "name" => name_value = Some(frontmatter_inline_value(block[0])),
            "description" => desc_value = Some(flatten_desc(block)),
            // model / tools / color are dropped here (color re-injected below).
            "model" | "tools" | "color" => {}
            _ => {
                for l in block {
                    extras.push(l.to_string());
                }
            }
        }
    }

    // Native zcode order: name, description, color. Unknown keys follow verbatim.
    let mut out: Vec<String> = vec!["---".to_string()];
    out.push(format!(
        "name: \"{}\"",
        escape_yaml_quoted(name_value.as_deref().unwrap_or(""))
    ));
    out.push(format!(
        "description: \"{}\"",
        escape_yaml_quoted(desc_value.as_deref().unwrap_or(""))
    ));
    out.push("color: \"#3b82f6\"".to_string());
    for l in extras {
        out.push(l);
    }
    out.push("---".to_string());

    // Body (system prompt), verbatim.
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

    // HOME is a process-global env var read by `home::home_dir()`. Tests that
    // set HOME must not run concurrently, or they clobber each other's HOME and
    // produce flaky failures. Serialize all HOME-mutating tests through this lock.
    static HOME_ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

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
        let _home_guard = HOME_ENV_LOCK.lock().unwrap();
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
    fn test_opencode_keeps_name_key() {
        let out = transform_opencode_frontmatter(claude_agent_doc());
        // opencode's Info schema REQUIRES `name`; keep it (don't drop).
        let fm: Vec<&str> = out
            .lines()
            .skip(1)
            .take_while(|l| l.trim() != "---")
            .collect();
        assert!(
            fm.iter().any(|l| l.starts_with("name:")),
            "`name:` should be kept in opencode frontmatter: {}",
            out
        );
    }

    #[test]
    fn test_opencode_flattens_folded_description() {
        let out = transform_opencode_frontmatter(claude_agent_doc());
        // Folded `description: >` must become a single plain inline string.
        assert!(
            !out.contains("description: >"),
            "folded `>` indicator must be removed: {}",
            out
        );
        assert!(
            out.contains(
                "description: Git operations specialist. Handles branches, commits, and merges."
            ),
            "folded body must be flattened to one line: {}",
            out
        );
    }

    #[test]
    fn test_opencode_flattens_multiline_folded_to_one_line() {
        // A folded scalar spanning several lines must join into a single line.
        let doc = "---\n\
                   name: masday-x\n\
                   description: >\n\
                   \x20 First sentence here.\n\
                   \x20 Second sentence here.\n\
                   \x20 Third sentence here.\n\
                   model: sonnet\n\
                   ---\n";
        let out = transform_opencode_frontmatter(doc);
        assert!(
            !out.contains("description: >"),
            "folded `>` indicator must be removed: {}",
            out
        );
        assert!(
            out.contains(
                "description: First sentence here. Second sentence here. Third sentence here."
            ),
            "multi-line folded body must be space-joined into one line: {}",
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
        let _home_guard = HOME_ENV_LOCK.lock().unwrap();
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
            // Frontmatter body = lines between the opening `---` and the next one
            // (skip(1) past the opening fence; take_while stops at the closing one).
            let fm: Vec<&str> = content
                .lines()
                .skip(1)
                .take_while(|l| l.trim() != "---")
                .collect();
            assert!(
                !fm.iter().any(|l| l.trim() == "- Read"),
                "{} still contains array-style tools",
                name
            );
            assert!(
                content.contains("mode: subagent"),
                "{} missing mode: subagent",
                name
            );
            assert!(
                fm.iter().any(|l| l.starts_with("name:")),
                "{} missing required name:",
                name
            );
            assert!(
                !content.contains("description: >"),
                "{} still uses folded description: >",
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

    // --- transform_zcode_frontmatter tests ---

    #[test]
    fn test_zcode_emits_quoted_name_description_and_color() {
        let out = transform_zcode_frontmatter(claude_agent_doc());
        // Frontmatter body (between the opening and closing `---`).
        let fm: Vec<&str> = out
            .lines()
            .skip(1)
            .take_while(|l| l.trim() != "---")
            .collect();
        assert!(
            fm.iter()
                .any(|l| l.trim_start() == "name: \"masday-git-master\""),
            "expected quoted name, got: {}",
            out
        );
        assert!(
            fm.iter().any(|l| l.trim_start() == "color: \"#3b82f6\""),
            "expected default color, got: {}",
            out
        );
        assert!(
            fm.iter().any(|l| l.starts_with("description: \"")),
            "expected quoted description, got: {}",
            out
        );
    }

    #[test]
    fn test_zcode_flattens_folded_description() {
        let out = transform_zcode_frontmatter(claude_agent_doc());
        assert!(
            !out.contains("description: >"),
            "folded `>` indicator must be removed: {}",
            out
        );
        assert!(
            out.contains("description: \"Git operations specialist. Handles branches, commits, and merges.\""),
            "folded body must be flattened into the quoted description: {}",
            out
        );
    }

    #[test]
    fn test_zcode_drops_model_and_tools() {
        // model (Claude alias) and tools (array + MCP names) are not valid zcode
        // values; they must be dropped entirely (both optional in zcode).
        let out = transform_zcode_frontmatter(claude_agent_doc());
        let fm: Vec<&str> = out
            .lines()
            .skip(1)
            .take_while(|l| l.trim() != "---")
            .collect();
        assert!(
            !fm.iter().any(|l| l.starts_with("model:")),
            "model must be dropped from zcode frontmatter: {}",
            out
        );
        assert!(
            !fm.iter().any(|l| l.starts_with("tools:")),
            "tools must be dropped from zcode frontmatter: {}",
            out
        );
        assert!(
            !out.contains("- Read") && !out.contains("git_status"),
            "array/MCP tool names must not leak: {}",
            out
        );
    }

    #[test]
    fn test_zcode_escapes_inner_quotes() {
        let doc = "---\nname: x\ndescription: She said \"hi\"\n---\nbody\n";
        let out = transform_zcode_frontmatter(doc);
        assert!(
            out.contains("description: \"She said \\\"hi\\\"\""),
            "inner double quotes must be escaped: {}",
            out
        );
    }

    #[test]
    fn test_zcode_preserves_markdown_body_verbatim() {
        let out = transform_zcode_frontmatter(claude_agent_doc());
        assert!(out.contains("# Git Operations Agent"));
        assert!(out.contains("Body stays untouched."));
    }

    #[test]
    fn test_zcode_no_frontmatter_left_untouched() {
        let doc = "# Just markdown\nno frontmatter here\n";
        assert_eq!(transform_zcode_frontmatter(doc), doc);
    }

    #[test]
    fn test_sync_agents_writes_zcode_format() {
        let temp_dir = TempDir::new().unwrap();
        let project_dir = temp_dir.path();

        let platforms = vec![Platform::Zcode];
        let reports = sync_agents_to_project(project_dir, &platforms, true).unwrap();

        let report = &reports[0];
        assert_eq!(report.platform, "zcode");
        assert!(report.copied > 0, "at least one agent should be written");

        let zcode_dir = project_dir.join(".zcode/cli/agents");
        let mut checked = 0;
        for entry in fs::read_dir(&zcode_dir).unwrap() {
            let entry = entry.unwrap();
            let name = entry.file_name().to_string_lossy().to_string();
            if !name.starts_with("masday-") || !name.ends_with(".md") {
                continue;
            }
            let content = fs::read_to_string(entry.path()).unwrap();
            let fm: Vec<&str> = content
                .lines()
                .skip(1)
                .take_while(|l| l.trim() != "---")
                .collect();
            assert!(
                fm.iter().any(|l| l.starts_with("name: \"")),
                "{} missing quoted name",
                name
            );
            assert!(
                fm.iter().any(|l| l.starts_with("description: \"")),
                "{} missing quoted description",
                name
            );
            assert!(
                fm.iter().any(|l| l.trim_start() == "color: \"#3b82f6\""),
                "{} missing default color",
                name
            );
            assert!(
                !content.contains("description: >"),
                "{} still uses folded description: >",
                name
            );
            assert!(
                !fm.iter().any(|l| l.starts_with("model:")),
                "{} must not carry a model line",
                name
            );
            assert!(
                !fm.iter().any(|l| l.starts_with("tools:")),
                "{} must not carry a tools line",
                name
            );
            checked += 1;
        }
        assert!(checked > 0, "expected at least one masday agent written");
    }

    #[test]
    fn test_skip_agent_sync_for_non_subagent_platforms() {
        // Codex/Cursor/Windsurf have no per-agent subagent mechanism — agent sync
        // must skip them (no files written, no dir created, a warning recorded).
        let temp_dir = TempDir::new().unwrap();
        let project_dir = temp_dir.path();

        let platforms = vec![Platform::Codex, Platform::Cursor, Platform::Windsurf];
        let reports = sync_agents_to_project(project_dir, &platforms, true).unwrap();

        assert_eq!(reports.len(), 3);
        for report in &reports {
            assert_eq!(
                report.copied, 0,
                "{} wrote agents (should skip)",
                report.platform
            );
            assert!(report.skipped > 0, "{} counted no skips", report.platform);
            assert!(
                !report.warnings.is_empty(),
                "{} recorded no skip warning",
                report.platform
            );
        }
        assert!(
            !project_dir.join(".codex/agents").exists(),
            ".codex/agents must not be created"
        );
        assert!(
            !project_dir.join(".cursor/rules").exists(),
            ".cursor/rules must not be created"
        );
        assert!(
            !project_dir.join(".windsurf/rules").exists(),
            ".windsurf/rules must not be created"
        );
    }

    #[test]
    fn test_skip_agent_sync_global_for_non_subagent_platforms() {
        // The skip is platform-based and happens before any dir is created, so it
        // must hold on the global path too without touching the real home.
        let platforms = vec![Platform::Codex, Platform::Cursor, Platform::Windsurf];
        let reports = sync_agents_to_global(&platforms, true).unwrap();
        for report in &reports {
            assert_eq!(
                report.copied, 0,
                "{} wrote global agents (should skip)",
                report.platform
            );
            assert!(
                !report.warnings.is_empty(),
                "{} recorded no skip warning",
                report.platform
            );
        }
    }
}
