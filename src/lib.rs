#[cfg(target_arch = "wasm32")]
use std::collections::BTreeMap;
#[cfg(target_arch = "wasm32")]
use zellij_tile::prelude::*;

pub mod attention;
pub mod config;
pub mod sidebar;
pub mod status;
pub mod status_bar;
pub mod status_bridge;
pub mod workspace;

pub mod teams;

/// Plugin rendering mode — determined by the `mode` key in the Zellij plugin
/// configuration BTreeMap.
#[cfg(target_arch = "wasm32")]
#[derive(Debug, Clone, PartialEq, Eq, Default)]
enum PluginMode {
    /// Full sidebar rendering (default).
    #[default]
    Sidebar,
    /// Minimal single-line status bar segment.
    StatusBar,
}

#[cfg(target_arch = "wasm32")]
struct ZellaiPlugin {
    bridge: status_bridge::StatusBridge,
    config: config::ZellaiConfig,
    attention: attention::AttentionTracker,
    /// Resolved home directory (from `echo ~`); used to expand `~` in paths.
    home_dir: Option<String>,
    /// Rendering mode: sidebar (default) or status-bar.
    mode: PluginMode,
    /// Workspace name shown in the status bar segment.
    workspace_name: String,
}

#[cfg(target_arch = "wasm32")]
impl Default for ZellaiPlugin {
    fn default() -> Self {
        let cfg = config::ZellaiConfig::default();
        let bridge = status_bridge::StatusBridge::new(
            &cfg.bridge.sessions_dir,
            cfg.bridge.stale_threshold_s,
        );
        Self {
            bridge,
            config: cfg,
            attention: attention::AttentionTracker::new(),
            home_dir: None,
            mode: PluginMode::default(),
            workspace_name: "zellai".to_string(),
        }
    }
}

#[cfg(target_arch = "wasm32")]
register_plugin!(ZellaiPlugin);

#[cfg(target_arch = "wasm32")]
impl ZellijPlugin for ZellaiPlugin {
    fn load(&mut self, configuration: BTreeMap<String, String>) {
        // Parse plugin mode from configuration BTreeMap.
        // Default is "sidebar"; "status-bar" switches to status bar mode.
        if let Some(mode_str) = configuration.get("mode") {
            match mode_str.as_str() {
                "status-bar" => self.mode = PluginMode::StatusBar,
                _ => self.mode = PluginMode::Sidebar,
            }
        }

        // Parse optional workspace name for the status bar segment.
        if let Some(name) = configuration.get("workspace")
            && !name.is_empty()
        {
            self.workspace_name = name.clone();
        }

        // Parse configuration from the plugin's configuration BTreeMap.
        // Look for a "config" key containing TOML; fall back to defaults.
        if let Some(toml_str) = configuration.get("config")
            && let Ok(cfg) = config::parse_config(toml_str)
        {
            self.config = cfg;
            self.bridge = status_bridge::StatusBridge::new(
                &self.config.bridge.sessions_dir,
                self.config.bridge.stale_threshold_s,
            );
        }

        // Request permissions needed for file watching and running commands
        request_permission(&[
            PermissionType::ReadApplicationState,
            PermissionType::ChangeApplicationState,
            PermissionType::RunCommands,
        ]);
        // Subscribe to events we'll need
        subscribe(&[
            EventType::Timer,
            EventType::FileSystemUpdate,
            EventType::FileSystemCreate,
            EventType::FileSystemDelete,
            EventType::RunCommandResult,
            EventType::PermissionRequestResult,
        ]);
        // Set a periodic timer for polling status files
        let interval = self.config.bridge.poll_interval_ms as f64 / 1000.0;
        set_timeout(interval);

        // Resolve the user's home directory so we can expand `~` in paths.
        // WASM plugins cannot call std::env, so we shell out.
        run_command(
            &["sh", "-c", "echo ~"],
            BTreeMap::from([("zellai_cmd".to_string(), "resolve_home".to_string())]),
        );
    }

    fn update(&mut self, event: Event) -> bool {
        match event {
            Event::Timer(_) => {
                // Re-arm the timer
                let interval = self.config.bridge.poll_interval_ms as f64 / 1000.0;
                set_timeout(interval);

                // Trigger an async listing of the sessions directory
                let sessions_dir = self.resolved_sessions_dir();
                let context = BTreeMap::from([
                    ("zellai_cmd".to_string(), "list_sessions".to_string()),
                    ("sessions_dir".to_string(), sessions_dir.clone()),
                ]);
                run_command(&["ls", "-1", &sessions_dir], context);

                // Get current epoch time for stale detection
                run_command(
                    &["date", "+%s"],
                    BTreeMap::from([("zellai_cmd".to_string(), "get_time".to_string())]),
                );

                false // don't re-render yet; wait for RunCommandResult
            }
            Event::RunCommandResult(exit_code, stdout, stderr, context) => {
                self.handle_run_command_result(exit_code, stdout, stderr, context)
            }
            Event::PermissionRequestResult(PermissionStatus::Granted) => {
                // Permissions granted — start watching filesystem
                watch_filesystem();
                true
            }
            Event::FileSystemCreate(_)
            | Event::FileSystemUpdate(_)
            | Event::FileSystemDelete(_) => {
                // Filesystem changed — trigger re-render so the next timer
                // cycle picks up the changes.
                true
            }
            _ => false,
        }
    }

    fn render(&mut self, rows: usize, cols: usize) {
        let agents = self.bridge.agents_sorted();
        let agent_refs: Vec<&status::AgentStatus> = agents.into_iter().collect();

        match self.mode {
            PluginMode::Sidebar => {
                let lines =
                    sidebar::render_sidebar(&agent_refs, &self.config.sidebar, rows, cols);
                for line in lines {
                    println!("{}", line);
                }
            }
            PluginMode::StatusBar => {
                let line =
                    status_bar::render_status_bar(&agent_refs, &self.workspace_name, cols);
                print!("{}", line);
            }
        }
    }
}

#[cfg(target_arch = "wasm32")]
impl ZellaiPlugin {
    fn handle_run_command_result(
        &mut self,
        exit_code: Option<i32>,
        stdout: Vec<u8>,
        _stderr: Vec<u8>,
        context: BTreeMap<String, String>,
    ) -> bool {
        let Some(cmd) = context.get("zellai_cmd") else {
            return false;
        };

        match cmd.as_str() {
            "list_sessions" => {
                if exit_code != Some(0) {
                    // ls failed (e.g. directory doesn't exist yet) — not an error,
                    // just means no sessions.
                    return false;
                }
                let stdout_str = String::from_utf8_lossy(&stdout);
                let sessions_dir = context
                    .get("sessions_dir")
                    .cloned()
                    .unwrap_or_else(|| self.resolved_sessions_dir());

                // Collect session IDs from .json filenames for cleanup
                let session_ids: Vec<&str> = stdout_str
                    .lines()
                    .map(|l| l.trim())
                    .filter(|f| !f.is_empty() && f.ends_with(".json"))
                    .filter_map(|f| f.strip_suffix(".json"))
                    .collect();

                // Remove sessions whose files no longer exist on disk
                self.bridge.retain_sessions(&session_ids);

                for line in stdout_str.lines() {
                    let filename = line.trim();
                    if filename.is_empty() || !filename.ends_with(".json") {
                        continue;
                    }
                    let filepath = format!("{}/{}", sessions_dir, filename);
                    run_command(
                        &["cat", &filepath],
                        BTreeMap::from([
                            ("zellai_cmd".to_string(), "read_status".to_string()),
                            ("session_file".to_string(), filename.to_string()),
                        ]),
                    );
                }
                false // wait for read_status results before re-rendering
            }
            "read_status" => {
                let Some(filename) = context.get("session_file") else {
                    return false;
                };
                let session_id = filename.strip_suffix(".json").unwrap_or(filename);

                if exit_code == Some(0) {
                    let stdout_str = String::from_utf8_lossy(&stdout);
                    // Ignore parse errors — the file might be partially written
                    let _ = self.bridge.update_from_json(session_id, &stdout_str);
                } else {
                    // File disappeared or is unreadable — remove from bridge
                    self.bridge.remove_session(session_id);
                }

                // Update attention tracker with current agent state
                let agents = self.bridge.agents_sorted();
                let agent_refs: Vec<&status::AgentStatus> = agents.into_iter().collect();
                self.attention.update(&agent_refs);

                true // trigger re-render
            }
            "get_time" => {
                if exit_code == Some(0) {
                    let stdout_str = String::from_utf8_lossy(&stdout);
                    if let Ok(epoch) = stdout_str.trim().parse::<u64>() {
                        self.bridge.mark_stale(epoch);
                        return true; // re-render to reflect staleness changes
                    }
                }
                false
            }
            "resolve_home" => {
                if exit_code == Some(0) {
                    let home = String::from_utf8_lossy(&stdout).trim().to_string();
                    if !home.is_empty() {
                        self.home_dir = Some(home);
                    }
                }
                false
            }
            _ => false,
        }
    }

    /// Return the sessions directory with `~` expanded to the resolved home path.
    ///
    /// If the home directory hasn't been resolved yet (the `resolve_home` command
    /// hasn't returned), returns the raw configured path unchanged.
    fn resolved_sessions_dir(&self) -> String {
        let raw = self.bridge.sessions_dir().to_string();
        if let Some(ref home) = self.home_dir
            && let Some(rest) = raw.strip_prefix('~')
        {
            return format!("{}{}", home, rest);
        }
        raw
    }
}
