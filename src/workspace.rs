//! Workspace data model and file persistence.
//!
//! Owns named workspace save/restore. Persists workspace layouts to
//! `<user-data-dir>/zellai/workspaces/<name>.json`.
//!
//! The data model types are available on all targets (including WASM).
//! File persistence functions use `std::fs` and are gated behind
//! `#[cfg(not(target_arch = "wasm32"))]`.

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Data model (available on all targets)
// ---------------------------------------------------------------------------

/// A saved workspace definition.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Workspace {
    /// Workspace name (used as filename: `<name>.json`)
    pub name: String,
    /// Working directory for the workspace
    pub working_dir: String,
    /// Template this workspace was created from (if any)
    pub template: Option<WorkspaceTemplate>,
    /// Pane definitions
    pub panes: Vec<PaneConfig>,
    /// When this workspace was last saved (Unix epoch seconds)
    pub saved_at: u64,
}

/// Pre-defined workspace templates from the vision doc.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum WorkspaceTemplate {
    SingleAgent,
    Team,
    Review,
    Research,
}

/// Configuration for a single pane in a workspace.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PaneConfig {
    /// Pane name/label shown in the sidebar
    pub name: String,
    /// Agent to run in this pane
    pub agent: String,
    /// Command to run (e.g., `["claude"]`, `["zellai", "run", "--agent", "codex", "--", "codex"]`)
    pub command: Vec<String>,
    /// Optional initial prompt/instruction
    pub prompt: Option<String>,
    /// Pane direction: horizontal or vertical split (relative to previous pane)
    pub direction: PaneDirection,
}

/// Direction of a pane split relative to the previous pane.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "lowercase")]
pub enum PaneDirection {
    #[default]
    Horizontal,
    Vertical,
}

// ---------------------------------------------------------------------------
// Workspace name validation
// ---------------------------------------------------------------------------

/// Validate a workspace name. Rejects empty names, names containing path
/// separators (`/`, `\`), names starting with `.`, and names that are too long.
pub fn validate_workspace_name(name: &str) -> Result<(), String> {
    if name.is_empty() {
        return Err("workspace name cannot be empty".to_string());
    }
    if name.len() > 255 {
        return Err("workspace name is too long (max 255 characters)".to_string());
    }
    if name.contains('/') || name.contains('\\') {
        return Err("workspace name cannot contain path separators".to_string());
    }
    if name.starts_with('.') {
        return Err("workspace name cannot start with '.'".to_string());
    }
    if name == "." || name == ".." {
        return Err("workspace name cannot be '.' or '..'".to_string());
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Template factory
// ---------------------------------------------------------------------------

/// Create a workspace from a template with sensible defaults.
pub fn from_template(name: &str, template: WorkspaceTemplate, working_dir: &str) -> Workspace {
    let panes = match &template {
        WorkspaceTemplate::SingleAgent => vec![PaneConfig {
            name: "agent".to_string(),
            agent: "claude".to_string(),
            command: vec!["claude".to_string()],
            prompt: None,
            direction: PaneDirection::Horizontal,
        }],
        WorkspaceTemplate::Team => vec![
            PaneConfig {
                name: "orchestrator".to_string(),
                agent: "claude".to_string(),
                command: vec!["claude".to_string()],
                prompt: Some("You are the orchestrator. Coordinate the team.".to_string()),
                direction: PaneDirection::Horizontal,
            },
            PaneConfig {
                name: "worker-1".to_string(),
                agent: "claude".to_string(),
                command: vec!["claude".to_string()],
                prompt: None,
                direction: PaneDirection::Vertical,
            },
            PaneConfig {
                name: "worker-2".to_string(),
                agent: "claude".to_string(),
                command: vec!["claude".to_string()],
                prompt: None,
                direction: PaneDirection::Horizontal,
            },
        ],
        WorkspaceTemplate::Review => vec![
            PaneConfig {
                name: "reviewer".to_string(),
                agent: "claude".to_string(),
                command: vec!["claude".to_string()],
                prompt: Some("Review the code changes for correctness and style.".to_string()),
                direction: PaneDirection::Horizontal,
            },
            PaneConfig {
                name: "main-agent".to_string(),
                agent: "claude".to_string(),
                command: vec!["claude".to_string()],
                prompt: None,
                direction: PaneDirection::Vertical,
            },
        ],
        WorkspaceTemplate::Research => vec![PaneConfig {
            name: "researcher".to_string(),
            agent: "gemini".to_string(),
            command: vec![
                "zellai".to_string(),
                "run".to_string(),
                "--agent".to_string(),
                "gemini".to_string(),
                "--".to_string(),
                "gemini".to_string(),
            ],
            prompt: Some("Research the topic thoroughly.".to_string()),
            direction: PaneDirection::Horizontal,
        }],
    };

    Workspace {
        name: name.to_string(),
        working_dir: working_dir.to_string(),
        template: Some(template),
        panes,
        saved_at: 0,
    }
}

// ---------------------------------------------------------------------------
// Default workspaces directory
// ---------------------------------------------------------------------------

/// Default workspaces directory path: `~/.local/share/zellai/workspaces`
pub fn default_workspaces_dir() -> String {
    "~/.local/share/zellai/workspaces".to_string()
}

// ---------------------------------------------------------------------------
// File persistence (native only — not available in WASM)
// ---------------------------------------------------------------------------

#[cfg(not(target_arch = "wasm32"))]
mod persistence {
    use super::*;
    use std::env;
    use std::fs;
    use std::path::{Path, PathBuf};

    /// Resolve workspaces dir, respecting `$ZELLAI_DATA_DIR` env var.
    ///
    /// - If `$ZELLAI_DATA_DIR` is set, returns `$ZELLAI_DATA_DIR/workspaces`
    /// - Otherwise returns `~/.local/share/zellai/workspaces` with `~` expanded
    pub fn resolve_workspaces_dir() -> PathBuf {
        if let Ok(data_dir) = env::var("ZELLAI_DATA_DIR") {
            return PathBuf::from(data_dir).join("workspaces");
        }
        let home = env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
        PathBuf::from(home).join(".local/share/zellai/workspaces")
    }

    /// Internal helper: resolve workspaces dir with explicit env values for testing.
    #[cfg(test)]
    fn resolve_workspaces_dir_with(zellai_data_dir: Option<&str>, home: Option<&str>) -> PathBuf {
        if let Some(data_dir) = zellai_data_dir {
            return PathBuf::from(data_dir).join("workspaces");
        }
        let home = home.unwrap_or("/tmp");
        PathBuf::from(home).join(".local/share/zellai/workspaces")
    }

    /// Save a workspace to `<dir>/<name>.json`.
    ///
    /// Creates the directory if it doesn't exist. Writes atomically
    /// (write to `.tmp`, then rename).
    pub fn save_workspace_to(workspace: &Workspace, dir: &Path) -> Result<(), String> {
        validate_workspace_name(&workspace.name)?;

        fs::create_dir_all(dir).map_err(|e| format!("failed to create workspaces dir: {e}"))?;

        let json =
            serde_json::to_string_pretty(workspace).map_err(|e| format!("serialize error: {e}"))?;

        let tmp_path = dir.join(format!("{}.json.tmp", workspace.name));
        let final_path = dir.join(format!("{}.json", workspace.name));

        fs::write(&tmp_path, &json).map_err(|e| format!("failed to write temp file: {e}"))?;
        fs::rename(&tmp_path, &final_path)
            .map_err(|e| format!("failed to rename temp file: {e}"))?;

        Ok(())
    }

    /// Save a workspace to the default workspaces directory.
    pub fn save_workspace(workspace: &Workspace) -> Result<(), String> {
        save_workspace_to(workspace, &resolve_workspaces_dir())
    }

    /// Load a workspace by name from `<dir>/<name>.json`.
    pub fn load_workspace_from(name: &str, dir: &Path) -> Result<Workspace, String> {
        validate_workspace_name(name)?;

        let path = dir.join(format!("{name}.json"));

        let content =
            fs::read_to_string(&path).map_err(|e| format!("failed to read workspace file: {e}"))?;
        let workspace: Workspace = serde_json::from_str(&content)
            .map_err(|e| format!("failed to parse workspace: {e}"))?;

        Ok(workspace)
    }

    /// Load a workspace by name from the default workspaces directory.
    pub fn load_workspace(name: &str) -> Result<Workspace, String> {
        load_workspace_from(name, &resolve_workspaces_dir())
    }

    /// List all saved workspace names in `dir` (scan for `.json` files).
    pub fn list_workspaces_in(dir: &Path) -> Result<Vec<String>, String> {
        if !dir.exists() {
            return Ok(Vec::new());
        }

        let entries =
            fs::read_dir(dir).map_err(|e| format!("failed to read workspaces dir: {e}"))?;

        let mut names: Vec<String> = entries
            .filter_map(|entry| {
                let entry = entry.ok()?;
                let filename = entry.file_name().to_string_lossy().to_string();
                filename.strip_suffix(".json").map(|n| n.to_string())
            })
            .collect();

        names.sort();
        Ok(names)
    }

    /// List all saved workspace names in the default workspaces directory.
    pub fn list_workspaces() -> Result<Vec<String>, String> {
        list_workspaces_in(&resolve_workspaces_dir())
    }

    /// Delete a saved workspace file from `dir`.
    pub fn delete_workspace_from(name: &str, dir: &Path) -> Result<(), String> {
        validate_workspace_name(name)?;

        let path = dir.join(format!("{name}.json"));

        if !path.exists() {
            return Err(format!("workspace '{name}' not found"));
        }

        fs::remove_file(&path).map_err(|e| format!("failed to delete workspace: {e}"))?;

        Ok(())
    }

    /// Delete a saved workspace file from the default workspaces directory.
    pub fn delete_workspace(name: &str) -> Result<(), String> {
        delete_workspace_from(name, &resolve_workspaces_dir())
    }

    // Re-export resolve helper for tests
    #[cfg(test)]
    pub(crate) fn resolve_workspaces_dir_with_env(
        zellai_data_dir: Option<&str>,
        home: Option<&str>,
    ) -> PathBuf {
        resolve_workspaces_dir_with(zellai_data_dir, home)
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub use persistence::*;

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // -- Validation tests --

    #[test]
    fn test_validate_workspace_name_valid() {
        assert!(validate_workspace_name("my-workspace").is_ok());
        assert!(validate_workspace_name("test_123").is_ok());
        assert!(validate_workspace_name("a").is_ok());
    }

    #[test]
    fn test_validate_workspace_name_empty() {
        let err = validate_workspace_name("").unwrap_err();
        assert!(err.contains("empty"));
    }

    #[test]
    fn test_validate_workspace_name_slash() {
        let err = validate_workspace_name("foo/bar").unwrap_err();
        assert!(err.contains("path separator"));
    }

    #[test]
    fn test_validate_workspace_name_backslash() {
        let err = validate_workspace_name("foo\\bar").unwrap_err();
        assert!(err.contains("path separator"));
    }

    #[test]
    fn test_validate_workspace_name_dotfile() {
        let err = validate_workspace_name(".hidden").unwrap_err();
        assert!(err.contains("start with '.'"));
    }

    #[test]
    fn test_validate_workspace_name_too_long() {
        let long_name = "a".repeat(256);
        let err = validate_workspace_name(&long_name).unwrap_err();
        assert!(err.contains("too long"));
    }

    // -- Serialization round-trip tests --

    #[test]
    fn test_workspace_serialize_roundtrip() {
        let ws = Workspace {
            name: "test-ws".to_string(),
            working_dir: "/home/user/project".to_string(),
            template: Some(WorkspaceTemplate::Team),
            panes: vec![
                PaneConfig {
                    name: "orchestrator".to_string(),
                    agent: "claude".to_string(),
                    command: vec!["claude".to_string()],
                    prompt: Some("Coordinate the team.".to_string()),
                    direction: PaneDirection::Horizontal,
                },
                PaneConfig {
                    name: "worker".to_string(),
                    agent: "codex".to_string(),
                    command: vec![
                        "zellai".to_string(),
                        "run".to_string(),
                        "--agent".to_string(),
                        "codex".to_string(),
                    ],
                    prompt: None,
                    direction: PaneDirection::Vertical,
                },
            ],
            saved_at: 1706000101,
        };

        let json = serde_json::to_string_pretty(&ws).expect("should serialize");
        let deserialized: Workspace = serde_json::from_str(&json).expect("should deserialize");
        assert_eq!(ws, deserialized);
    }

    #[test]
    fn test_pane_direction_serde() {
        let json = serde_json::to_string(&PaneDirection::Horizontal).unwrap();
        assert_eq!(json, r#""horizontal""#);

        let json = serde_json::to_string(&PaneDirection::Vertical).unwrap();
        assert_eq!(json, r#""vertical""#);
    }

    #[test]
    fn test_workspace_template_serde() {
        let json = serde_json::to_string(&WorkspaceTemplate::SingleAgent).unwrap();
        assert_eq!(json, r#""single-agent""#);

        let json = serde_json::to_string(&WorkspaceTemplate::Team).unwrap();
        assert_eq!(json, r#""team""#);

        let json = serde_json::to_string(&WorkspaceTemplate::Review).unwrap();
        assert_eq!(json, r#""review""#);

        let json = serde_json::to_string(&WorkspaceTemplate::Research).unwrap();
        assert_eq!(json, r#""research""#);
    }

    // -- Template factory tests --

    #[test]
    fn test_from_template_single_agent() {
        let ws = from_template("solo", WorkspaceTemplate::SingleAgent, "/tmp/project");
        assert_eq!(ws.name, "solo");
        assert_eq!(ws.working_dir, "/tmp/project");
        assert_eq!(ws.template, Some(WorkspaceTemplate::SingleAgent));
        assert_eq!(ws.panes.len(), 1);
        assert_eq!(ws.panes[0].agent, "claude");
    }

    #[test]
    fn test_from_template_team() {
        let ws = from_template("my-team", WorkspaceTemplate::Team, "/tmp/project");
        assert_eq!(ws.panes.len(), 3);
        assert_eq!(ws.panes[0].name, "orchestrator");
        assert!(ws.panes[0].prompt.is_some());
        assert_eq!(ws.panes[1].name, "worker-1");
        assert_eq!(ws.panes[2].name, "worker-2");
    }

    #[test]
    fn test_from_template_review() {
        let ws = from_template("code-review", WorkspaceTemplate::Review, "/tmp/project");
        assert_eq!(ws.panes.len(), 2);
        assert_eq!(ws.panes[0].name, "reviewer");
        assert!(ws.panes[0].prompt.as_ref().unwrap().contains("Review"));
        assert_eq!(ws.panes[1].name, "main-agent");
    }

    #[test]
    fn test_from_template_research() {
        let ws = from_template("research", WorkspaceTemplate::Research, "/tmp/project");
        assert_eq!(ws.panes.len(), 1);
        assert_eq!(ws.panes[0].agent, "gemini");
        assert!(ws.panes[0].prompt.is_some());
    }

    // -- Default workspaces dir --

    #[test]
    fn test_default_workspaces_dir() {
        let dir = default_workspaces_dir();
        assert_eq!(dir, "~/.local/share/zellai/workspaces");
    }

    // -- Persistence tests (native only) --

    #[cfg(not(target_arch = "wasm32"))]
    mod persistence_tests {
        use super::super::*;
        use std::fs;
        use std::path::PathBuf;

        /// Create a unique temporary directory for a test.
        fn make_test_dir() -> PathBuf {
            let id = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            // Include thread id for uniqueness across parallel tests
            let tid = format!("{:?}", std::thread::current().id());
            let dir = std::env::temp_dir().join(format!("zellai-ws-test-{id}-{tid}"));
            fs::create_dir_all(&dir).unwrap();
            dir
        }

        fn cleanup(dir: &PathBuf) {
            let _ = fs::remove_dir_all(dir);
        }

        #[test]
        fn test_resolve_workspaces_dir_with_env() {
            let dir = resolve_workspaces_dir_with_env(Some("/custom/data"), None);
            assert_eq!(dir, PathBuf::from("/custom/data/workspaces"));
        }

        #[test]
        fn test_resolve_workspaces_dir_with_home() {
            let dir = resolve_workspaces_dir_with_env(None, Some("/home/testuser"));
            assert_eq!(
                dir,
                PathBuf::from("/home/testuser/.local/share/zellai/workspaces")
            );
        }

        #[test]
        fn test_resolve_workspaces_dir_fallback() {
            let dir = resolve_workspaces_dir_with_env(None, None);
            assert_eq!(dir, PathBuf::from("/tmp/.local/share/zellai/workspaces"));
        }

        #[test]
        fn test_save_load_lifecycle() {
            let test_dir = make_test_dir();

            let ws = from_template(
                "lifecycle-test",
                WorkspaceTemplate::SingleAgent,
                "/tmp/proj",
            );

            // Save
            save_workspace_to(&ws, &test_dir).expect("save should succeed");

            // Load
            let loaded =
                load_workspace_from("lifecycle-test", &test_dir).expect("load should succeed");
            assert_eq!(loaded.name, "lifecycle-test");
            assert_eq!(loaded.panes.len(), 1);
            assert_eq!(loaded, ws);

            cleanup(&test_dir);
        }

        #[test]
        fn test_list_workspaces_empty() {
            let test_dir = make_test_dir();

            let names = list_workspaces_in(&test_dir).expect("list should succeed");
            assert!(names.is_empty());

            cleanup(&test_dir);
        }

        #[test]
        fn test_list_workspaces_multiple() {
            let test_dir = make_test_dir();

            let ws1 = from_template("alpha", WorkspaceTemplate::SingleAgent, "/tmp/a");
            let ws2 = from_template("beta", WorkspaceTemplate::Team, "/tmp/b");
            let ws3 = from_template("gamma", WorkspaceTemplate::Review, "/tmp/c");

            save_workspace_to(&ws1, &test_dir).unwrap();
            save_workspace_to(&ws2, &test_dir).unwrap();
            save_workspace_to(&ws3, &test_dir).unwrap();

            let names = list_workspaces_in(&test_dir).expect("list should succeed");
            assert_eq!(names, vec!["alpha", "beta", "gamma"]);

            cleanup(&test_dir);
        }

        #[test]
        fn test_delete_workspace() {
            let test_dir = make_test_dir();

            let ws = from_template("to-delete", WorkspaceTemplate::SingleAgent, "/tmp/d");
            save_workspace_to(&ws, &test_dir).unwrap();

            // Verify it exists
            assert!(load_workspace_from("to-delete", &test_dir).is_ok());

            // Delete
            delete_workspace_from("to-delete", &test_dir).expect("delete should succeed");

            // Verify it's gone
            assert!(load_workspace_from("to-delete", &test_dir).is_err());
            assert!(
                !list_workspaces_in(&test_dir)
                    .unwrap()
                    .contains(&"to-delete".to_string())
            );

            cleanup(&test_dir);
        }

        #[test]
        fn test_delete_nonexistent_workspace() {
            let test_dir = make_test_dir();

            let err = delete_workspace_from("nonexistent", &test_dir).unwrap_err();
            assert!(err.contains("not found"));

            cleanup(&test_dir);
        }

        #[test]
        fn test_save_invalid_name_rejected() {
            let test_dir = make_test_dir();

            let mut ws = from_template("ok", WorkspaceTemplate::SingleAgent, "/tmp");
            ws.name = "".to_string();
            assert!(save_workspace_to(&ws, &test_dir).is_err());

            ws.name = "foo/bar".to_string();
            assert!(save_workspace_to(&ws, &test_dir).is_err());

            ws.name = ".hidden".to_string();
            assert!(save_workspace_to(&ws, &test_dir).is_err());

            cleanup(&test_dir);
        }

        #[test]
        fn test_load_invalid_name_rejected() {
            let test_dir = make_test_dir();

            assert!(load_workspace_from("", &test_dir).is_err());
            assert!(load_workspace_from("foo/bar", &test_dir).is_err());
            assert!(load_workspace_from(".hidden", &test_dir).is_err());

            cleanup(&test_dir);
        }

        #[test]
        fn test_save_overwrites_existing() {
            let test_dir = make_test_dir();

            let ws1 = from_template("overwrite-test", WorkspaceTemplate::SingleAgent, "/tmp/v1");
            save_workspace_to(&ws1, &test_dir).unwrap();

            let ws2 = from_template("overwrite-test", WorkspaceTemplate::Team, "/tmp/v2");
            save_workspace_to(&ws2, &test_dir).unwrap();

            let loaded = load_workspace_from("overwrite-test", &test_dir).unwrap();
            assert_eq!(loaded.working_dir, "/tmp/v2");
            assert_eq!(loaded.panes.len(), 3); // Team has 3 panes

            cleanup(&test_dir);
        }

        #[test]
        fn test_full_lifecycle() {
            let test_dir = make_test_dir();

            // Start empty
            assert_eq!(list_workspaces_in(&test_dir).unwrap().len(), 0);

            // Create and save
            let ws = from_template("full-cycle", WorkspaceTemplate::Review, "/home/user/proj");
            save_workspace_to(&ws, &test_dir).unwrap();

            // List shows it
            let names = list_workspaces_in(&test_dir).unwrap();
            assert_eq!(names, vec!["full-cycle"]);

            // Load and verify
            let loaded = load_workspace_from("full-cycle", &test_dir).unwrap();
            assert_eq!(loaded.panes.len(), 2);

            // Delete
            delete_workspace_from("full-cycle", &test_dir).unwrap();
            assert_eq!(list_workspaces_in(&test_dir).unwrap().len(), 0);

            cleanup(&test_dir);
        }

        #[test]
        fn test_list_nonexistent_dir() {
            let dir = PathBuf::from("/tmp/zellai-nonexistent-dir-that-does-not-exist");
            let names = list_workspaces_in(&dir).expect("should return empty vec");
            assert!(names.is_empty());
        }
    }
}
