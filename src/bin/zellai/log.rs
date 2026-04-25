//! `zellai log <pane>` — view per-pane execution logs.
//!
//! Reads log files written by `StatusWriter` during `zellai run` sessions.
//! Log files are stored at `<sessions_dir>/<workspace>/<pane>.log`.

use std::fs;
use std::path::PathBuf;

use crate::status_writer;

/// Resolve the log directory for a given workspace.
///
/// Returns `<sessions_dir>/<workspace>/`.
#[allow(dead_code)] // Convenience wrapper for runtime use
pub fn resolve_log_dir(workspace: &str) -> PathBuf {
    resolve_log_dir_with_env(workspace, |k| std::env::var(k).ok())
}

/// Testable version: accepts an env-var lookup function.
pub fn resolve_log_dir_with_env(
    workspace: &str,
    env_var: impl Fn(&str) -> Option<String>,
) -> PathBuf {
    let sessions_dir = status_writer::resolve_sessions_dir_with_env(env_var);
    sessions_dir.join(workspace)
}

/// Resolve the workspace name from explicit argument, env var, or default.
pub fn resolve_workspace(explicit: Option<&str>) -> String {
    resolve_workspace_with_env(explicit, |k| std::env::var(k).ok())
}

/// Testable version: accepts an env-var lookup function.
pub fn resolve_workspace_with_env(
    explicit: Option<&str>,
    env_var: impl Fn(&str) -> Option<String>,
) -> String {
    if let Some(ws) = explicit {
        return ws.to_string();
    }
    if let Some(ws) = env_var("ZELLAI_WORKSPACE")
        && !ws.is_empty()
    {
        return ws;
    }
    "default".to_string()
}

/// Resolve the full path to a pane's log file.
pub fn resolve_log_path(pane_name: &str, workspace: &str) -> PathBuf {
    resolve_log_path_with_env(pane_name, workspace, |k| std::env::var(k).ok())
}

/// Testable version: accepts an env-var lookup function.
pub fn resolve_log_path_with_env(
    pane_name: &str,
    workspace: &str,
    env_var: impl Fn(&str) -> Option<String>,
) -> PathBuf {
    let log_dir = resolve_log_dir_with_env(workspace, env_var);
    log_dir.join(format!("{pane_name}.log"))
}

/// Run the `zellai log` subcommand.
///
/// Reads and prints the log file for the given pane. Supports `--lines N` to
/// show only the last N lines (tail behavior) and `--follow` as a placeholder.
pub fn run(
    pane_name: &str,
    workspace: Option<&str>,
    follow: bool,
    lines: Option<usize>,
) -> Result<(), String> {
    let ws = resolve_workspace(workspace);
    let log_path = resolve_log_path(pane_name, &ws);

    if !log_path.exists() {
        return Err(format!(
            "No log found for pane '{}'. Logs are created when agents are run via 'zellai run'.",
            pane_name
        ));
    }

    let content =
        fs::read_to_string(&log_path).map_err(|e| format!("Failed to read log file: {e}"))?;

    if follow {
        eprintln!("Note: --follow mode is not yet supported. Showing current log contents.");
    }

    match lines {
        Some(n) => {
            let all_lines: Vec<&str> = content.lines().collect();
            let start = all_lines.len().saturating_sub(n);
            for line in &all_lines[start..] {
                println!("{line}");
            }
        }
        None => {
            print!("{content}");
        }
    }

    Ok(())
}

/// Search all workspaces for a matching pane log file.
///
/// Used when no workspace is specified and `ZELLAI_WORKSPACE` is not set.
/// Returns the workspace name if a matching `<pane>.log` is found.
#[allow(dead_code)] // Public API for future use
pub fn find_workspace_for_pane(pane_name: &str) -> Option<String> {
    find_workspace_for_pane_with_env(pane_name, |k| std::env::var(k).ok())
}

/// Testable version: accepts an env-var lookup function.
#[allow(dead_code)] // Used by find_workspace_for_pane and tests
pub fn find_workspace_for_pane_with_env(
    pane_name: &str,
    env_var: impl Fn(&str) -> Option<String>,
) -> Option<String> {
    let sessions_dir = status_writer::resolve_sessions_dir_with_env(&env_var);
    let entries = fs::read_dir(&sessions_dir).ok()?;
    let log_filename = format!("{pane_name}.log");

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir()
            && path.join(&log_filename).exists()
            && let Some(name) = path.file_name().and_then(|n| n.to_str())
        {
            return Some(name.to_string());
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    // ---- resolve_workspace tests ----

    #[test]
    fn test_resolve_workspace_explicit() {
        let ws = resolve_workspace_with_env(Some("my-project"), |_| None);
        assert_eq!(ws, "my-project");
    }

    #[test]
    fn test_resolve_workspace_from_env() {
        let ws = resolve_workspace_with_env(None, |key| {
            if key == "ZELLAI_WORKSPACE" {
                Some("env-workspace".to_string())
            } else {
                None
            }
        });
        assert_eq!(ws, "env-workspace");
    }

    #[test]
    fn test_resolve_workspace_empty_env() {
        let ws = resolve_workspace_with_env(None, |key| {
            if key == "ZELLAI_WORKSPACE" {
                Some(String::new())
            } else {
                None
            }
        });
        assert_eq!(ws, "default");
    }

    #[test]
    fn test_resolve_workspace_default() {
        let ws = resolve_workspace_with_env(None, |_| None);
        assert_eq!(ws, "default");
    }

    #[test]
    fn test_resolve_workspace_explicit_overrides_env() {
        let ws = resolve_workspace_with_env(Some("explicit"), |key| {
            if key == "ZELLAI_WORKSPACE" {
                Some("env-workspace".to_string())
            } else {
                None
            }
        });
        assert_eq!(ws, "explicit");
    }

    // ---- resolve_log_dir tests ----

    #[test]
    fn test_resolve_log_dir() {
        let dir = resolve_log_dir_with_env("my-workspace", |key| match key {
            "ZELLAI_SESSIONS_DIR" => Some("/custom/sessions".to_string()),
            _ => None,
        });
        assert_eq!(dir, PathBuf::from("/custom/sessions/my-workspace"));
    }

    #[test]
    fn test_resolve_log_dir_default_sessions() {
        let dir = resolve_log_dir_with_env("default", |key| match key {
            "HOME" => Some("/home/testuser".to_string()),
            _ => None,
        });
        assert_eq!(
            dir,
            PathBuf::from("/home/testuser/.local/share/zellai/sessions/default")
        );
    }

    // ---- resolve_log_path tests ----

    #[test]
    fn test_resolve_log_path() {
        let path = resolve_log_path_with_env("agent-1", "my-workspace", |key| match key {
            "ZELLAI_SESSIONS_DIR" => Some("/data/sessions".to_string()),
            _ => None,
        });
        assert_eq!(
            path,
            PathBuf::from("/data/sessions/my-workspace/agent-1.log")
        );
    }

    #[test]
    fn test_resolve_log_path_default_workspace() {
        let path = resolve_log_path_with_env("main-pane", "default", |key| match key {
            "HOME" => Some("/home/user".to_string()),
            _ => None,
        });
        assert_eq!(
            path,
            PathBuf::from("/home/user/.local/share/zellai/sessions/default/main-pane.log")
        );
    }

    // ---- run() error handling tests ----

    #[test]
    fn test_run_missing_log_file() {
        // Use an explicit workspace with a non-existent directory.
        // Safety: set_var in test context for test isolation.
        unsafe {
            std::env::set_var(
                "ZELLAI_SESSIONS_DIR",
                "/tmp/zellai-test-nonexistent-dir-12345",
            );
        }
        let result = run("nonexistent-pane", Some("no-workspace"), false, None);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            err.contains("No log found for pane 'nonexistent-pane'"),
            "expected helpful error, got: {err}"
        );
        assert!(
            err.contains("zellai run"),
            "error should mention 'zellai run', got: {err}"
        );
    }

    // ---- run() reading tests (with temp files) ----

    #[test]
    fn test_run_reads_full_log() {
        let dir = std::env::temp_dir()
            .join("zellai-test-log")
            .join(format!("full-log-{}", std::process::id()));
        let ws_dir = dir.join("test-ws");
        let _ = std::fs::create_dir_all(&ws_dir);

        let log_content = "2025-01-01T00:00:00Z Status changed to: thinking\n\
                           2025-01-01T00:00:05Z Status changed to: waiting\n\
                           2025-01-01T00:00:10Z Status changed to: idle\n";
        std::fs::write(ws_dir.join("my-pane.log"), log_content).unwrap();

        // Safety: set_var in test context for test isolation.
        unsafe {
            std::env::set_var("ZELLAI_SESSIONS_DIR", &dir);
        }
        let result = run("my-pane", Some("test-ws"), false, None);
        assert!(result.is_ok());

        // Clean up
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_run_tail_lines() {
        let dir = std::env::temp_dir()
            .join("zellai-test-log")
            .join(format!("tail-{}", std::process::id()));
        let ws_dir = dir.join("tail-ws");
        let _ = std::fs::create_dir_all(&ws_dir);

        let log_content = "line1\nline2\nline3\nline4\nline5\n";
        std::fs::write(ws_dir.join("tail-pane.log"), log_content).unwrap();

        // Safety: set_var in test context for test isolation.
        unsafe {
            std::env::set_var("ZELLAI_SESSIONS_DIR", &dir);
        }
        // We can't easily capture stdout in a unit test, but we can verify it doesn't error
        let result = run("tail-pane", Some("tail-ws"), false, Some(2));
        assert!(result.is_ok());

        // Clean up
        let _ = std::fs::remove_dir_all(&dir);
    }

    // ---- find_workspace_for_pane tests ----

    #[test]
    fn test_find_workspace_for_pane_found() {
        let dir = std::env::temp_dir()
            .join("zellai-test-log")
            .join(format!("find-ws-{}", std::process::id()));
        let ws_dir = dir.join("my-project");
        let _ = std::fs::create_dir_all(&ws_dir);
        std::fs::write(ws_dir.join("agent-1.log"), "test").unwrap();

        let result = find_workspace_for_pane_with_env("agent-1", |key| match key {
            "ZELLAI_SESSIONS_DIR" => Some(dir.to_string_lossy().to_string()),
            _ => None,
        });
        assert_eq!(result, Some("my-project".to_string()));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_find_workspace_for_pane_not_found() {
        let dir = std::env::temp_dir()
            .join("zellai-test-log")
            .join(format!("find-ws-none-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);

        let result = find_workspace_for_pane_with_env("nonexistent", |key| match key {
            "ZELLAI_SESSIONS_DIR" => Some(dir.to_string_lossy().to_string()),
            _ => None,
        });
        assert_eq!(result, None);

        let _ = std::fs::remove_dir_all(&dir);
    }
}
