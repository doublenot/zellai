use std::collections::BTreeMap;
use zellij_tile::prelude::*;

pub mod config;
pub mod status;
pub mod status_bridge;

// Module declarations for future files (commented out until they exist)
// mod sidebar;
// mod attention;
// mod workspace;
// mod teams;

struct ZellaiPlugin {
    bridge: status_bridge::StatusBridge,
    config: config::ZellaiConfig,
}

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
        }
    }
}

register_plugin!(ZellaiPlugin);

impl ZellijPlugin for ZellaiPlugin {
    fn load(&mut self, configuration: BTreeMap<String, String>) {
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
    }

    fn update(&mut self, event: Event) -> bool {
        match event {
            Event::Timer(_) => {
                // Re-arm the timer
                let interval = self.config.bridge.poll_interval_ms as f64 / 1000.0;
                set_timeout(interval);

                // Trigger an async listing of the sessions directory
                let sessions_dir = self.bridge.sessions_dir().to_string();
                let context = BTreeMap::from([
                    ("zellai_cmd".to_string(), "list_sessions".to_string()),
                    ("sessions_dir".to_string(), sessions_dir.clone()),
                ]);
                run_command(&["ls", "-1", &sessions_dir], context);

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
        // Top border
        println!("╭{}╮", "─".repeat(cols.saturating_sub(2)));

        if rows > 2 {
            let title = " zellai ";
            let padding = cols.saturating_sub(2 + title.len());
            println!("│{}{: <width$}│", title, "", width = padding);
        }

        if self.bridge.has_agents() {
            // Render one line per agent
            let agents = self.bridge.agents_sorted();
            let max_agent_rows = rows.saturating_sub(4); // reserve top border, title, gap, bottom border
            for (i, agent) in agents.iter().enumerate() {
                if i >= max_agent_rows {
                    break;
                }
                let icon = status_icon(&agent.status);
                let line = format!(" {} {}: {} ", icon, agent.session_id, agent.status);
                let padding = cols.saturating_sub(2 + line.len());
                println!("│{}{: <width$}│", line, "", width = padding);
            }
            // Fill remaining rows
            let used = agents.len().min(max_agent_rows);
            for _ in 0..max_agent_rows.saturating_sub(used) {
                println!("│{: <width$}│", "", width = cols.saturating_sub(2));
            }
        } else {
            // No agents — show placeholder
            if rows > 3 {
                let msg = " No agents connected ";
                let padding = cols.saturating_sub(2 + msg.len());
                println!("│{}{: <width$}│", msg, "", width = padding);
            }
            for _ in 0..rows.saturating_sub(4).min(rows) {
                println!("│{: <width$}│", "", width = cols.saturating_sub(2));
            }
        }

        // Bottom border
        if rows > 1 {
            println!("╰{}╯", "─".repeat(cols.saturating_sub(2)));
        }
    }
}

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
                    .unwrap_or_else(|| self.bridge.sessions_dir().to_string());

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
                true // trigger re-render
            }
            _ => false,
        }
    }
}

/// Map agent status to a Unicode icon for display.
fn status_icon(status: &status::AgentStatusValue) -> &'static str {
    match status {
        status::AgentStatusValue::Thinking => "🧠",
        status::AgentStatusValue::Waiting => "⏳",
        status::AgentStatusValue::Idle => "💤",
        status::AgentStatusValue::Error => "❌",
    }
}
