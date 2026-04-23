//! `zellai teams` subcommand — launches a multi-agent team layout.

use zellai::config::{TeamsLayout, ZellaiConfig, parse_config};
use zellai::teams::generate_team_layout;

use crate::workspace_cmd::pane_direction_flag;

/// Parse a layout name string into a [`TeamsLayout`].
pub fn parse_teams_layout(s: &str) -> Result<TeamsLayout, String> {
    match s {
        "orchestrator-top" => Ok(TeamsLayout::OrchestratorTop),
        "orchestrator-left" => Ok(TeamsLayout::OrchestratorLeft),
        "equal-grid" => Ok(TeamsLayout::EqualGrid),
        _ => Err(format!(
            "unknown layout '{}'. Valid layouts: orchestrator-top, orchestrator-left, equal-grid",
            s
        )),
    }
}

/// `zellai teams [--layout <layout>] [--dir <dir>]`
///
/// Loads `zellai.toml` from the working directory (or `dir` if specified),
/// generates the team layout, and executes Zellij commands to create panes.
pub fn cmd_teams(layout: Option<&str>, dir: Option<&str>) -> Result<(), String> {
    let working_dir = match dir {
        Some(d) => d.to_string(),
        None => std::env::current_dir()
            .map(|p| p.to_string_lossy().to_string())
            .map_err(|e| format!("failed to get current directory: {e}"))?,
    };

    // Load config from zellai.toml in the working directory, or use defaults.
    let config_path = format!("{}/zellai.toml", working_dir);
    let mut config = match std::fs::read_to_string(&config_path) {
        Ok(contents) => {
            parse_config(&contents).map_err(|e| format!("failed to parse {}: {e}", config_path))?
        }
        Err(_) => ZellaiConfig::default(),
    };

    // Override layout if specified on the command line.
    if let Some(layout_str) = layout {
        config.teams.default_layout = parse_teams_layout(layout_str)?;
    }

    let panes = generate_team_layout(&config.teams, &working_dir);

    if panes.is_empty() {
        println!("Custom layout not yet supported. No panes to create.");
        return Ok(());
    }

    // Count orchestrator vs workers for the summary.
    let orchestrator_count = panes.iter().filter(|p| p.name == "orchestrator").count();
    let worker_count = panes.len() - orchestrator_count;

    // First pane: create a new tab.
    let first_pane = &panes[0];
    let tab_status = std::process::Command::new("zellij")
        .args(["action", "new-tab", "--name", "team", "--cwd", &working_dir])
        .status()
        .map_err(|e| format!("failed to run zellij: {e}"))?;

    if !tab_status.success() {
        return Err(format!(
            "zellij action new-tab failed (exit code: {:?})",
            tab_status.code()
        ));
    }

    // Write command into the first pane if present.
    if !first_pane.command.is_empty() {
        let cmd_str = format!("{}\n", first_pane.command.join(" "));
        let write_status = std::process::Command::new("zellij")
            .args(["action", "write-chars", &cmd_str])
            .status()
            .map_err(|e| format!("failed to run zellij: {e}"))?;

        if !write_status.success() {
            return Err(format!(
                "zellij action write-chars failed (exit code: {:?})",
                write_status.code()
            ));
        }
    }

    // Subsequent panes.
    for pane in &panes[1..] {
        let direction = pane_direction_flag(&pane.direction);
        let pane_status = std::process::Command::new("zellij")
            .args([
                "action",
                "new-pane",
                "--direction",
                direction,
                "--cwd",
                &working_dir,
                "--name",
                &pane.name,
            ])
            .status()
            .map_err(|e| format!("failed to run zellij: {e}"))?;

        if !pane_status.success() {
            return Err(format!(
                "zellij action new-pane failed (exit code: {:?})",
                pane_status.code()
            ));
        }

        // Write command into this pane if present.
        if !pane.command.is_empty() {
            let cmd_str = format!("{}\n", pane.command.join(" "));
            let write_status = std::process::Command::new("zellij")
                .args(["action", "write-chars", &cmd_str])
                .status()
                .map_err(|e| format!("failed to run zellij: {e}"))?;

            if !write_status.success() {
                return Err(format!(
                    "zellij action write-chars failed (exit code: {:?})",
                    write_status.code()
                ));
            }
        }
    }

    println!(
        "Launched team: {} orchestrator + {} workers",
        orchestrator_count, worker_count
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_teams_layout_orchestrator_top() {
        assert_eq!(
            parse_teams_layout("orchestrator-top").unwrap(),
            TeamsLayout::OrchestratorTop
        );
    }

    #[test]
    fn test_parse_teams_layout_orchestrator_left() {
        assert_eq!(
            parse_teams_layout("orchestrator-left").unwrap(),
            TeamsLayout::OrchestratorLeft
        );
    }

    #[test]
    fn test_parse_teams_layout_equal_grid() {
        assert_eq!(
            parse_teams_layout("equal-grid").unwrap(),
            TeamsLayout::EqualGrid
        );
    }

    #[test]
    fn test_parse_teams_layout_invalid() {
        let err = parse_teams_layout("nonexistent").unwrap_err();
        assert!(err.contains("unknown layout"));
        assert!(err.contains("nonexistent"));
        assert!(err.contains("orchestrator-top"));
        assert!(err.contains("orchestrator-left"));
        assert!(err.contains("equal-grid"));
    }

    #[test]
    fn test_parse_teams_layout_custom_not_exposed() {
        // "custom" is a valid TeamsLayout variant but not selectable from CLI
        let err = parse_teams_layout("custom").unwrap_err();
        assert!(err.contains("unknown layout"));
    }

    #[test]
    fn test_config_fallback_to_defaults() {
        // When no zellai.toml exists, config should be default
        let config = ZellaiConfig::default();
        assert_eq!(config.teams.default_layout, TeamsLayout::OrchestratorTop);
        assert_eq!(config.teams.orchestrator_agent, "claude");
        assert_eq!(config.teams.worker_agent, "claude");
        assert_eq!(config.teams.worker_count, 2);
    }

    #[test]
    fn test_config_with_toml_override() {
        let toml_str = r#"
[teams]
default_layout = "equal-grid"
worker_count = 4
"#;
        let config = parse_config(toml_str).unwrap();
        assert_eq!(config.teams.default_layout, TeamsLayout::EqualGrid);
        assert_eq!(config.teams.worker_count, 4);
        // Defaults preserved for unspecified fields
        assert_eq!(config.teams.orchestrator_agent, "claude");
        assert_eq!(config.teams.worker_agent, "claude");
    }

    #[test]
    fn test_layout_override_applied() {
        // Simulate what cmd_teams does: parse config, then override layout
        let mut config = ZellaiConfig::default();
        assert_eq!(config.teams.default_layout, TeamsLayout::OrchestratorTop);

        config.teams.default_layout = parse_teams_layout("orchestrator-left").unwrap();
        assert_eq!(config.teams.default_layout, TeamsLayout::OrchestratorLeft);
    }

    #[test]
    fn test_empty_panes_for_custom_layout() {
        let mut config = ZellaiConfig::default();
        config.teams.default_layout = TeamsLayout::Custom;
        let panes = generate_team_layout(&config.teams, "/tmp");
        assert!(panes.is_empty());
    }
}
