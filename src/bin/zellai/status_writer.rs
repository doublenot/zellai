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
        s if s.starts_with("claude") => "claude",
        "codex" => "codex",
        s if s.starts_with("gemini") => "gemini",
        "aider" => "aider",
        "opencode" => "opencode",
        _ => "unknown",
    }
}

/// Generate a session ID from `$ZELLAI_SESSION_ID` env, or `hostname-PID`.
///
/// Delegates to `generate_session_id_with_env` using the real environment.
pub fn generate_session_id() -> String {
    generate_session_id_with_env(|k| env::var(k).ok())
}

/// Testable version: accepts an env-var lookup function.
pub fn generate_session_id_with_env(env_var: impl Fn(&str) -> Option<String>) -> String {
    if let Some(id) = env_var("ZELLAI_SESSION_ID")
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
///
/// Delegates to `resolve_sessions_dir_with_env` using the real environment.
pub fn resolve_sessions_dir() -> PathBuf {
    resolve_sessions_dir_with_env(|k| env::var(k).ok())
}

/// Testable version: accepts an env-var lookup function.
pub fn resolve_sessions_dir_with_env(env_var: impl Fn(&str) -> Option<String>) -> PathBuf {
    if let Some(dir) = env_var("ZELLAI_SESSIONS_DIR")
        && !dir.is_empty()
    {
        return PathBuf::from(dir);
    }

    if let Some(xdg) = env_var("XDG_DATA_HOME")
        && !xdg.is_empty()
    {
        return PathBuf::from(xdg).join("zellai").join("sessions");
    }

    if let Some(home) = env_var("HOME") {
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
    use std::path::Path;

    // ---- detect_agent tests ----

    #[test]
    fn test_detect_agent_claude() {
        assert_eq!(detect_agent("claude"), "claude");
        assert_eq!(detect_agent("/usr/bin/claude"), "claude");
        assert_eq!(detect_agent("claude-code"), "claude");
    }

    #[test]
    fn test_detect_agent_codex() {
        assert_eq!(detect_agent("codex"), "codex");
        assert_eq!(detect_agent("/usr/local/bin/codex"), "codex");
    }

    #[test]
    fn test_detect_agent_gemini() {
        assert_eq!(detect_agent("gemini"), "gemini");
        assert_eq!(detect_agent("/opt/gemini"), "gemini");
    }

    #[test]
    fn test_detect_agent_aider() {
        assert_eq!(detect_agent("aider"), "aider");
        assert_eq!(detect_agent("/home/user/.local/bin/aider"), "aider");
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
        assert_eq!(detect_agent("/snap/bin/codex"), "codex");
    }

    // ---- generate_session_id tests ----

    #[test]
    fn test_generate_session_id_from_env() {
        let id = generate_session_id_with_env(|key| {
            if key == "ZELLAI_SESSION_ID" {
                Some("my-custom-session-42".to_string())
            } else {
                None
            }
        });
        assert_eq!(id, "my-custom-session-42");
    }

    #[test]
    fn test_generate_session_id_default() {
        // No ZELLAI_SESSION_ID set — should return hostname-PID format
        let id = generate_session_id_with_env(|_| None);
        // Should contain a hyphen separating hostname and PID
        assert!(
            id.contains('-'),
            "session id should be hostname-PID, got: {id}"
        );
        // The part after the last hyphen should be our PID
        let pid_str = std::process::id().to_string();
        assert!(
            id.ends_with(&format!("-{pid_str}")),
            "session id should end with PID, got: {id}"
        );
    }

    #[test]
    fn test_generate_session_id_empty_env_var() {
        // Empty env var should fall through to hostname-PID
        let id = generate_session_id_with_env(|key| {
            if key == "ZELLAI_SESSION_ID" {
                Some(String::new())
            } else {
                None
            }
        });
        assert!(
            id.contains('-'),
            "empty env var should fall through to hostname-PID, got: {id}"
        );
    }

    // ---- resolve_sessions_dir tests ----

    #[test]
    fn test_resolve_sessions_dir_from_env() {
        let dir = resolve_sessions_dir_with_env(|key| {
            if key == "ZELLAI_SESSIONS_DIR" {
                Some("/custom/path/sessions".to_string())
            } else {
                None
            }
        });
        assert_eq!(dir, PathBuf::from("/custom/path/sessions"));
    }

    #[test]
    fn test_resolve_sessions_dir_xdg() {
        let dir = resolve_sessions_dir_with_env(|key| match key {
            "ZELLAI_SESSIONS_DIR" => None,
            "XDG_DATA_HOME" => Some("/home/testuser/.data".to_string()),
            _ => None,
        });
        assert_eq!(dir, PathBuf::from("/home/testuser/.data/zellai/sessions"));
    }

    #[test]
    fn test_resolve_sessions_dir_default() {
        let dir = resolve_sessions_dir_with_env(|key| match key {
            "ZELLAI_SESSIONS_DIR" => None,
            "XDG_DATA_HOME" => None,
            "HOME" => Some("/home/testuser".to_string()),
            _ => None,
        });
        assert_eq!(
            dir,
            PathBuf::from("/home/testuser/.local/share/zellai/sessions")
        );
    }

    #[test]
    fn test_resolve_sessions_dir_fallback() {
        // No env vars at all — should fall back to /tmp
        let dir = resolve_sessions_dir_with_env(|_| None);
        assert_eq!(dir, PathBuf::from("/tmp/zellai/sessions"));
    }

    #[test]
    fn test_resolve_sessions_dir_empty_env_vars() {
        // Empty env vars should be treated as absent
        let dir = resolve_sessions_dir_with_env(|key| match key {
            "ZELLAI_SESSIONS_DIR" => Some(String::new()),
            "XDG_DATA_HOME" => Some(String::new()),
            "HOME" => Some("/home/user".to_string()),
            _ => None,
        });
        assert_eq!(
            dir,
            PathBuf::from("/home/user/.local/share/zellai/sessions")
        );
    }

    // ---- StatusWriter: write_status tests ----

    /// Helper: create a temp directory for test isolation.
    fn test_sessions_dir(test_name: &str) -> PathBuf {
        let dir = env::temp_dir().join("zellai-test").join(format!(
            "{}-{}",
            test_name,
            std::process::id()
        ));
        let _ = fs::create_dir_all(&dir);
        dir
    }

    /// Helper: clean up a test temp directory.
    fn cleanup_test_dir(dir: &Path) {
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn test_write_status_creates_valid_json() {
        let dir = test_sessions_dir("write_status_json");
        let writer = StatusWriter::new(
            "test-session-1".to_string(),
            "claude".to_string(),
            dir.clone(),
        );

        writer
            .write_status("thinking", Some("Reading files..."), false)
            .unwrap();

        // Read and parse the JSON
        let content = fs::read_to_string(writer.status_file_path()).unwrap();
        let json: serde_json::Value = serde_json::from_str(&content).unwrap();

        // Verify all SCHEMA.md required fields are present and correctly typed
        assert_eq!(json["version"], 1);
        assert_eq!(json["session_id"], "test-session-1");
        assert_eq!(json["agent"], "claude");
        assert_eq!(json["status"], "thinking");
        // git_branch is string or null — just verify it exists
        assert!(json.get("git_branch").is_some(), "git_branch field missing");
        // git_dirty is boolean
        assert!(
            json["git_dirty"].is_boolean(),
            "git_dirty should be boolean"
        );
        // working_dir is a string
        assert!(
            json["working_dir"].is_string(),
            "working_dir should be string"
        );
        assert_eq!(json["last_message"], "Reading files...");
        // ports is an array
        assert!(json["ports"].is_array(), "ports should be array");
        assert_eq!(json["ports"].as_array().unwrap().len(), 0);
        // needs_attention is boolean
        assert_eq!(json["needs_attention"], false);
        // updated_at is a positive integer
        assert!(json["updated_at"].is_u64(), "updated_at should be u64");
        assert!(json["updated_at"].as_u64().unwrap() > 0);

        cleanup_test_dir(&dir);
    }

    #[test]
    fn test_write_status_null_last_message() {
        let dir = test_sessions_dir("write_status_null_msg");
        let writer = StatusWriter::new(
            "test-null-msg".to_string(),
            "codex".to_string(),
            dir.clone(),
        );

        writer.write_status("idle", None, false).unwrap();

        let content = fs::read_to_string(writer.status_file_path()).unwrap();
        let json: serde_json::Value = serde_json::from_str(&content).unwrap();

        assert!(
            json["last_message"].is_null(),
            "last_message should be null when None"
        );

        cleanup_test_dir(&dir);
    }

    #[test]
    fn test_write_status_atomic() {
        let dir = test_sessions_dir("write_status_atomic");
        let writer =
            StatusWriter::new("test-atomic".to_string(), "claude".to_string(), dir.clone());

        writer.write_status("thinking", None, false).unwrap();

        // The .tmp file should not exist after write completes
        let tmp_path = writer.status_file_path().with_extension("tmp");
        assert!(
            !tmp_path.exists(),
            "tmp file should not remain after atomic write"
        );
        // But the actual status file should exist
        assert!(
            writer.status_file_path().exists(),
            "status file should exist"
        );

        cleanup_test_dir(&dir);
    }

    #[test]
    fn test_cleanup_removes_file() {
        let dir = test_sessions_dir("cleanup_removes");
        let writer =
            StatusWriter::new("test-cleanup".to_string(), "aider".to_string(), dir.clone());

        writer.write_status("thinking", None, false).unwrap();
        assert!(
            writer.status_file_path().exists(),
            "status file should exist before cleanup"
        );

        writer.cleanup().unwrap();
        assert!(
            !writer.status_file_path().exists(),
            "status file should be gone after cleanup"
        );

        cleanup_test_dir(&dir);
    }

    #[test]
    fn test_cleanup_idempotent() {
        let dir = test_sessions_dir("cleanup_idempotent");
        let writer = StatusWriter::new("test-idem".to_string(), "claude".to_string(), dir.clone());

        // cleanup without ever writing — should not error
        writer.cleanup().unwrap();
        // cleanup twice — should not error
        writer.write_status("idle", None, false).unwrap();
        writer.cleanup().unwrap();
        writer.cleanup().unwrap();

        cleanup_test_dir(&dir);
    }

    #[test]
    fn test_json_escapes_special_chars() {
        // Verify serde_json properly handles special characters in working_dir / last_message.
        // We can't easily set the working_dir to a path with quotes (it reads CWD),
        // but we CAN test that last_message with special chars produces valid JSON.
        let dir = test_sessions_dir("json_escapes");
        let writer =
            StatusWriter::new("test-escape".to_string(), "claude".to_string(), dir.clone());

        let tricky_msg = r#"Reading "file with quotes" and \backslashes\ and tabs	here"#;
        writer
            .write_status("thinking", Some(tricky_msg), false)
            .unwrap();

        let content = fs::read_to_string(writer.status_file_path()).unwrap();
        // Must parse as valid JSON — this is the critical check
        let json: serde_json::Value =
            serde_json::from_str(&content).expect("JSON with special characters should be valid");
        assert_eq!(json["last_message"].as_str().unwrap(), tricky_msg);

        cleanup_test_dir(&dir);
    }

    #[test]
    fn test_status_file_path() {
        let dir = PathBuf::from("/tmp/zellai/sessions");
        let writer = StatusWriter::new("my-host-1234".to_string(), "claude".to_string(), dir);
        assert_eq!(
            writer.status_file_path(),
            PathBuf::from("/tmp/zellai/sessions/my-host-1234.json")
        );
    }

    #[test]
    fn test_write_status_creates_sessions_dir() {
        let dir = test_sessions_dir("creates_dir").join("nested").join("deep");
        // The nested directory doesn't exist yet
        assert!(!dir.exists());

        let writer = StatusWriter::new("test-mkdir".to_string(), "gemini".to_string(), dir.clone());
        writer.write_status("thinking", None, false).unwrap();

        assert!(
            dir.exists(),
            "write_status should create the sessions directory"
        );
        assert!(writer.status_file_path().exists());

        // Clean up the parent
        cleanup_test_dir(&dir.parent().unwrap().parent().unwrap().to_path_buf());
    }

    #[test]
    fn test_write_status_overwrites() {
        let dir = test_sessions_dir("write_overwrite");
        let writer = StatusWriter::new(
            "test-overwrite".to_string(),
            "claude".to_string(),
            dir.clone(),
        );

        writer
            .write_status("thinking", Some("first"), false)
            .unwrap();
        writer.write_status("idle", Some("second"), false).unwrap();

        let content = fs::read_to_string(writer.status_file_path()).unwrap();
        let json: serde_json::Value = serde_json::from_str(&content).unwrap();
        assert_eq!(json["status"], "idle");
        assert_eq!(json["last_message"], "second");

        cleanup_test_dir(&dir);
    }
}
