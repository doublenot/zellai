//! Status file writer for `zellai run`.
//!
//! This is the write-side counterpart to the plugin's `status_bridge.rs` (read-side).
//! Writes agent status JSON files atomically (write to `.tmp`, then rename).

use std::env;
use std::fs;
use std::io;
use std::path::PathBuf;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

/// Writes agent status files to the sessions directory.
pub struct StatusWriter {
    session_id: String,
    agent: String,
    sessions_dir: PathBuf,
}

impl StatusWriter {
    pub fn new(session_id: String, agent: String, sessions_dir: PathBuf) -> Self {
        Self {
            session_id,
            agent,
            sessions_dir,
        }
    }

    /// Write a status file atomically (write to `.tmp`, then rename).
    pub fn write_status(
        &self,
        status: &str,
        last_message: Option<&str>,
        needs_attention: bool,
    ) -> io::Result<()> {
        fs::create_dir_all(&self.sessions_dir)?;

        let (git_branch, git_dirty) = collect_git_info();
        let working_dir = get_working_dir();
        let updated_at = epoch_secs();

        let json = serde_json::json!({
            "version": 1,
            "session_id": self.session_id,
            "agent": self.agent,
            "status": status,
            "git_branch": git_branch,
            "git_dirty": git_dirty,
            "working_dir": working_dir,
            "last_message": last_message,
            "ports": [],
            "needs_attention": needs_attention,
            "updated_at": updated_at
        });

        let content = serde_json::to_string_pretty(&json).map_err(io::Error::other)?;

        // Atomic write: write to tmp file, then rename
        let tmp_path = self.status_file_path().with_extension("tmp");
        fs::write(&tmp_path, content)?;
        fs::rename(&tmp_path, self.status_file_path())?;

        Ok(())
    }

    /// Remove the status file.
    pub fn cleanup(&self) -> io::Result<()> {
        let path = self.status_file_path();
        if path.exists() {
            fs::remove_file(&path)?;
        }
        // Also clean up any stale tmp file
        let tmp_path = path.with_extension("tmp");
        if tmp_path.exists() {
            let _ = fs::remove_file(&tmp_path);
        }
        Ok(())
    }

    pub fn status_file_path(&self) -> PathBuf {
        self.sessions_dir.join(format!("{}.json", self.session_id))
    }

    pub fn session_id(&self) -> &str {
        &self.session_id
    }
}

/// Run `git rev-parse --abbrev-ref HEAD` and `git diff --quiet` to get branch and dirty state.
fn collect_git_info() -> (Option<String>, bool) {
    let branch = Command::new("git")
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .filter(|s| !s.is_empty());

    let dirty = Command::new("git")
        .args(["diff", "--quiet"])
        .status()
        .map(|s| !s.success()) // exit 1 = dirty, exit 0 = clean
        .unwrap_or(false);

    (branch, dirty)
}

/// Get the current working directory as a string.
fn get_working_dir() -> String {
    env::current_dir()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|_| "/unknown".to_string())
}

/// Get current Unix epoch seconds.
fn epoch_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Detect agent kind from the command name.
/// Maps known command names to agent identifiers matching SCHEMA.md.
pub fn detect_agent(command: &str) -> &str {
    // Extract the base command name (strip path prefixes)
    let base = command.rsplit('/').next().unwrap_or(command);
    match base {
        "claude" => "claude",
        "codex" => "codex",
        "gemini" => "gemini",
        "aider" => "aider",
        "opencode" => "opencode",
        _ => "unknown",
    }
}

/// Generate a session ID from `$ZELLAI_SESSION_ID` env, or `hostname-PID`.
pub fn generate_session_id() -> String {
    if let Ok(id) = env::var("ZELLAI_SESSION_ID")
        && !id.is_empty()
    {
        return id;
    }

    let hostname = Command::new("hostname")
        .output()
        .ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "localhost".to_string());

    let pid = std::process::id();
    format!("{hostname}-{pid}")
}

/// Resolve the sessions directory path.
/// Checks `$ZELLAI_SESSIONS_DIR`, then `$XDG_DATA_HOME/zellai/sessions`,
/// then `$HOME/.local/share/zellai/sessions`.
pub fn resolve_sessions_dir() -> PathBuf {
    if let Ok(dir) = env::var("ZELLAI_SESSIONS_DIR")
        && !dir.is_empty()
    {
        return PathBuf::from(dir);
    }

    if let Ok(xdg) = env::var("XDG_DATA_HOME")
        && !xdg.is_empty()
    {
        return PathBuf::from(xdg).join("zellai").join("sessions");
    }

    if let Ok(home) = env::var("HOME") {
        return PathBuf::from(home)
            .join(".local")
            .join("share")
            .join("zellai")
            .join("sessions");
    }

    // Fallback
    PathBuf::from("/tmp/zellai/sessions")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_agent_known() {
        assert_eq!(detect_agent("claude"), "claude");
        assert_eq!(detect_agent("codex"), "codex");
        assert_eq!(detect_agent("gemini"), "gemini");
        assert_eq!(detect_agent("aider"), "aider");
        assert_eq!(detect_agent("opencode"), "opencode");
    }

    #[test]
    fn test_detect_agent_unknown() {
        assert_eq!(detect_agent("vim"), "unknown");
        assert_eq!(detect_agent("echo"), "unknown");
        assert_eq!(detect_agent(""), "unknown");
    }

    #[test]
    fn test_detect_agent_with_path() {
        assert_eq!(detect_agent("/usr/bin/claude"), "claude");
        assert_eq!(detect_agent("/home/user/.local/bin/aider"), "aider");
    }
}
