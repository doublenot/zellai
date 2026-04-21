use std::collections::BTreeMap;
use zellij_tile::prelude::*;

pub mod status;

// Module declarations for future files (commented out until they exist)
// mod sidebar;
// mod status_bridge;
// mod config;
// mod attention;
// mod workspace;
// mod teams;

#[derive(Default)]
struct ZellaiPlugin {
    loading: bool,
}

register_plugin!(ZellaiPlugin);

impl ZellijPlugin for ZellaiPlugin {
    fn load(&mut self, _configuration: BTreeMap<String, String>) {
        self.loading = true;
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
        // Set a periodic timer (500ms) for polling status files
        set_timeout(0.5);
    }

    fn update(&mut self, event: Event) -> bool {
        match event {
            Event::Timer(_) => {
                // Re-arm the timer
                set_timeout(0.5);
                // Will trigger re-render when we have status data to show
                true
            }
            Event::PermissionRequestResult(PermissionStatus::Granted) => {
                // Permissions granted — start watching filesystem
                watch_filesystem();
                true
            }
            _ => false,
        }
    }

    fn render(&mut self, rows: usize, cols: usize) {
        // Placeholder rendering — proves the plugin loads and renders
        println!("╭{}╮", "─".repeat(cols.saturating_sub(2)));
        if rows > 2 {
            let title = " zellai ";
            let padding = cols.saturating_sub(2 + title.len());
            println!("│{}{: <width$}│", title, "", width = padding);
        }
        if rows > 3 {
            let msg = " No agents connected ";
            let padding = cols.saturating_sub(2 + msg.len());
            println!("│{}{: <width$}│", msg, "", width = padding);
        }
        for _ in 0..rows.saturating_sub(4).min(rows) {
            println!("│{: <width$}│", "", width = cols.saturating_sub(2));
        }
        if rows > 1 {
            println!("╰{}╯", "─".repeat(cols.saturating_sub(2)));
        }
    }
}
