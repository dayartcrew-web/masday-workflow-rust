//! Command execution utilities for CLI tool hardening
//!
//! Provides timeout and output truncation for external command execution.

use std::process::Output as StdOutput;
use std::time::Duration;
use tokio::process::Command;

/// Default wall-clock cap for a wrapped CLI command.
pub const DEFAULT_CMD_TIMEOUT: Duration = Duration::from_secs(300);

/// Hard cap on captured stdout/stderr we return (prevents OOM / huge MCP messages).
pub const MAX_OUTPUT_BYTES: usize = 1 << 20; // 1 MiB

/// Run a command with a DEFAULT_CMD_TIMEOUT. Errors on spawn failure or timeout.
pub async fn run(cmd: &mut Command) -> Result<StdOutput, String> {
    tokio::time::timeout(DEFAULT_CMD_TIMEOUT, cmd.output())
        .await
        .map_err(|_| "command timed out after 300s".to_string())?
        .map_err(|e| format!("failed to spawn command: {e}"))
}

/// Truncate a captured string to ~MAX_OUTPUT_BYTES on a UTF-8 char boundary, noting the original size.
pub fn truncate_output(s: &str) -> String {
    if s.len() <= MAX_OUTPUT_BYTES {
        return s.to_string();
    }
    let mut end = MAX_OUTPUT_BYTES;
    while end > 0 && !s.is_char_boundary(end) {
        end -= 1;
    }
    format!("{}\n…[truncated: original {} bytes]", &s[..end], s.len())
}

/// Reject a user-supplied CLI argument that looks like a flag.
///
/// These MCP tools invoke CLIs (`docker`, `cargo`, `pnpm`) via `Command::new`
/// (exec, not a shell), so there is **no shell injection**. The residual risk
/// is *flag injection*: a user value passed as a **bare positional** — i.e.
/// placed after a subcommand rather than bound by a value-taking option such
/// as `-m`/`-t` — is parsed by the underlying CLI as an option when it starts
/// with `-`. For example `docker_run { image: "--privileged" }` would run
/// `docker run --privileged`, and `npm_run { script: "--version" }` would run
/// `pnpm --version`. None of the values guarded here (image, script name,
/// package name, test pattern) legitimately starts with `-`, so reject such
/// inputs up front. (Value-taking option arguments — `git_commit -m <msg>`,
/// `docker_build -t <tag>` — are NOT vectors: the option binds the next token
/// as its value regardless of a leading dash, and commit messages legitimately
/// begin with `-`.)
pub fn reject_flag_like(value: &str, field: &str) -> Result<(), String> {
    if value.starts_with('-') {
        Err(format!(
            "{field} must not start with '-' (would be parsed as a CLI flag): got {value:?}"
        ))
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_truncate_output_small() {
        let small = "hello";
        assert_eq!(truncate_output(small), "hello");
    }

    #[test]
    fn test_truncate_output_exact() {
        let exact = "x".repeat(MAX_OUTPUT_BYTES);
        assert_eq!(truncate_output(&exact).len(), MAX_OUTPUT_BYTES);
    }

    #[test]
    fn test_truncate_output_large() {
        let large = "x".repeat(MAX_OUTPUT_BYTES + 100);
        let truncated = truncate_output(&large);
        assert!(truncated.len() < large.len());
        assert!(truncated.contains("truncated: original"));
        assert!(truncated.ends_with(&format!("{} bytes]", MAX_OUTPUT_BYTES + 100)));
    }

    #[test]
    fn test_truncate_output_utf8_boundary() {
        // Test with multi-byte UTF-8 character near boundary
        let mut s = "x".repeat(MAX_OUTPUT_BYTES - 2);
        s.push('é'); // 2-byte UTF-8 character
        let truncated = truncate_output(&s);
        // Should not cut in the middle of the UTF-8 character
        assert!(!truncated.contains('�'));
    }

    #[test]
    fn test_reject_flag_like_accepts_plain_values() {
        assert!(reject_flag_like("nginx:latest", "image").is_ok());
        assert!(reject_flag_like("build", "script").is_ok());
        assert!(reject_flag_like("react", "package").is_ok());
        // Empty string is not flag-like (emptiness is the caller's concern).
        assert!(reject_flag_like("", "image").is_ok());
    }

    #[test]
    fn test_reject_flag_like_rejects_leading_dash() {
        // Single dash — the classic flag-injection payload.
        let err = reject_flag_like("-privileged", "image").unwrap_err();
        assert!(err.contains("image"));
        assert!(
            err.contains("flag"),
            "err should explain flag injection: {err}"
        );

        // Double dash — e.g. would-be `docker run --privileged` / `pnpm --version`.
        let err = reject_flag_like("--privileged", "image").unwrap_err();
        assert!(err.contains("flag"));

        // A value-looking string is still rejected when it starts with '-'.
        assert!(reject_flag_like("-x", "package").is_err());
    }
}
