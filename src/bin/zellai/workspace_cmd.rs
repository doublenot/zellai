//! Workspace CLI subcommands: `new`, `list`, `kill`.

use std::time::SystemTime;
use zellai::workspace::{
    WorkspaceTemplate, delete_workspace, from_template, list_workspaces, load_workspace,
    save_workspace, validate_workspace_name,
};

/// Parse a template name string into a `WorkspaceTemplate`.
fn parse_template(name: &str) -> Result<WorkspaceTemplate, String> {
    match name {
        "single-agent" => Ok(WorkspaceTemplate::SingleAgent),
        "team" => Ok(WorkspaceTemplate::Team),
        "review" => Ok(WorkspaceTemplate::Review),
        "research" => Ok(WorkspaceTemplate::Research),
        _ => Err(format!(
            "unknown template '{}'. Valid templates: single-agent, team, review, research",
            name
        )),
    }
}

/// Format a Unix epoch timestamp as a human-readable relative time string.
fn format_relative_time(epoch_secs: u64) -> String {
    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    if epoch_secs == 0 || now < epoch_secs {
        return "unknown".to_string();
    }

    let delta = now - epoch_secs;

    if delta < 60 {
        return "just now".to_string();
    }
    if delta < 3600 {
        let mins = delta / 60;
        return format!("{}m ago", mins);
    }
    if delta < 86400 {
        let hours = delta / 3600;
        return format!("{}h ago", hours);
    }
    let days = delta / 86400;
    format!("{}d ago", days)
}

/// Format a template name for display.
fn template_display(template: &Option<WorkspaceTemplate>) -> &str {
    match template {
        Some(WorkspaceTemplate::SingleAgent) => "single-agent",
        Some(WorkspaceTemplate::Team) => "team",
        Some(WorkspaceTemplate::Review) => "review",
        Some(WorkspaceTemplate::Research) => "research",
        None => "custom",
    }
}

/// `zellai new <name> [--template <template>] [--dir <dir>] [--force]`
pub fn cmd_new(name: &str, template: &str, dir: Option<&str>, force: bool) -> Result<(), String> {
    validate_workspace_name(name)?;

    let tmpl = parse_template(template)?;

    let working_dir = match dir {
        Some(d) => d.to_string(),
        None => std::env::current_dir()
            .map(|p| p.to_string_lossy().to_string())
            .map_err(|e| format!("failed to get current directory: {e}"))?,
    };

    // Check if workspace already exists (by trying to load it)
    if !force && load_workspace(name).is_ok() {
        return Err(format!(
            "workspace '{}' already exists. Use --force to overwrite.",
            name
        ));
    }

    let mut ws = from_template(name, tmpl, &working_dir);

    // Set saved_at to current time
    ws.saved_at = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    save_workspace(&ws)?;

    println!("Created workspace '{}' (template: {})", name, template);
    Ok(())
}

/// `zellai list`
pub fn cmd_list() -> Result<(), String> {
    let names = list_workspaces()?;

    if names.is_empty() {
        println!("No saved workspaces. Create one with: zellai new <name>");
        return Ok(());
    }

    for name in &names {
        match load_workspace(name) {
            Ok(ws) => {
                let tmpl = template_display(&ws.template);
                let pane_count = ws.panes.len();
                let saved = format_relative_time(ws.saved_at);
                println!(
                    "{:<16}{:<16}{:<40}{} panes    saved {}",
                    ws.name, tmpl, ws.working_dir, pane_count, saved
                );
            }
            Err(_) => {
                // If we can't load it, still list the name
                println!("{:<16}(error loading workspace)", name);
            }
        }
    }

    Ok(())
}

/// `zellai kill <name>`
pub fn cmd_kill(name: &str) -> Result<(), String> {
    delete_workspace(name)?;
    println!("Deleted workspace '{}'", name);
    Ok(())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_relative_time_just_now() {
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        assert_eq!(format_relative_time(now), "just now");
        assert_eq!(format_relative_time(now - 30), "just now");
    }

    #[test]
    fn test_format_relative_time_minutes() {
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        assert_eq!(format_relative_time(now - 60), "1m ago");
        assert_eq!(format_relative_time(now - 120), "2m ago");
        assert_eq!(format_relative_time(now - 3540), "59m ago");
    }

    #[test]
    fn test_format_relative_time_hours() {
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        assert_eq!(format_relative_time(now - 3600), "1h ago");
        assert_eq!(format_relative_time(now - 7200), "2h ago");
        assert_eq!(format_relative_time(now - 82800), "23h ago");
    }

    #[test]
    fn test_format_relative_time_days() {
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        assert_eq!(format_relative_time(now - 86400), "1d ago");
        assert_eq!(format_relative_time(now - 172800), "2d ago");
        assert_eq!(format_relative_time(now - 604800), "7d ago");
    }

    #[test]
    fn test_format_relative_time_zero() {
        assert_eq!(format_relative_time(0), "unknown");
    }

    #[test]
    fn test_format_relative_time_future() {
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        assert_eq!(format_relative_time(now + 1000), "unknown");
    }

    #[test]
    fn test_parse_template_valid() {
        assert_eq!(
            parse_template("single-agent").unwrap(),
            WorkspaceTemplate::SingleAgent
        );
        assert_eq!(parse_template("team").unwrap(), WorkspaceTemplate::Team);
        assert_eq!(parse_template("review").unwrap(), WorkspaceTemplate::Review);
        assert_eq!(
            parse_template("research").unwrap(),
            WorkspaceTemplate::Research
        );
    }

    #[test]
    fn test_parse_template_invalid() {
        let err = parse_template("nonexistent").unwrap_err();
        assert!(err.contains("unknown template"));
        assert!(err.contains("nonexistent"));
    }

    #[test]
    fn test_template_display() {
        assert_eq!(
            template_display(&Some(WorkspaceTemplate::SingleAgent)),
            "single-agent"
        );
        assert_eq!(template_display(&Some(WorkspaceTemplate::Team)), "team");
        assert_eq!(template_display(&Some(WorkspaceTemplate::Review)), "review");
        assert_eq!(
            template_display(&Some(WorkspaceTemplate::Research)),
            "research"
        );
        assert_eq!(template_display(&None), "custom");
    }
}
