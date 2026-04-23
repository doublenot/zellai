//! Teams layout generation — transforms TeamsConfig into pane specifications.
//!
//! This module contains the pure-logic layer for generating team pane layouts
//! from configuration. It does NOT contain CLI or Zellij API calls — just the
//! data transformation from `TeamsConfig` + working directory into a list of
//! pane specifications suitable for workspace creation.

use crate::config::{TeamsConfig, TeamsLayout};
use crate::workspace::{PaneConfig, PaneDirection};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Build the command array for a pane running a given agent.
///
/// The convention is: `["zellai", "run", "--agent", "<agent>", "--", "<agent>"]`
fn agent_command(agent: &str) -> Vec<String> {
    vec![
        "zellai".to_string(),
        "run".to_string(),
        "--agent".to_string(),
        agent.to_string(),
        "--".to_string(),
        agent.to_string(),
    ]
}

/// Create a worker pane config.
fn worker_pane(index: u32, agent: &str, direction: PaneDirection) -> PaneConfig {
    PaneConfig {
        name: format!("worker-{}", index),
        agent: agent.to_string(),
        command: agent_command(agent),
        prompt: None,
        direction,
    }
}

/// Create the orchestrator pane config.
fn orchestrator_pane(agent: &str, direction: PaneDirection) -> PaneConfig {
    PaneConfig {
        name: "orchestrator".to_string(),
        agent: agent.to_string(),
        command: agent_command(agent),
        prompt: None,
        direction,
    }
}

// ---------------------------------------------------------------------------
// Layout generation
// ---------------------------------------------------------------------------

/// Generate a list of [`PaneConfig`]s from a [`TeamsConfig`] and working directory.
///
/// The returned `Vec` represents the panes to create, in order.
/// The first pane is always the orchestrator (or first worker in equal-grid).
///
/// The `_working_dir` parameter is reserved for future use (e.g. setting cwd
/// per pane). Currently the working directory is managed externally by the
/// workspace or CLI layer.
///
/// # Layout rules
///
/// - **`OrchestratorTop`**: One orchestrator pane (horizontal split, full width
///   at top). Then `worker_count` worker panes split vertically below.
/// - **`OrchestratorLeft`**: One orchestrator pane (vertical split, full height
///   on left). Then `worker_count` worker panes split horizontally to the right.
/// - **`EqualGrid`**: No special orchestrator. `worker_count + 1` panes in
///   alternating horizontal/vertical splits to approximate a grid.
/// - **`Custom`**: Returns an empty `Vec` (custom layouts come from
///   `[[teams.layout]]` in TOML, which is a future addition).
pub fn generate_team_layout(config: &TeamsConfig, _working_dir: &str) -> Vec<PaneConfig> {
    match config.default_layout {
        TeamsLayout::OrchestratorTop => layout_orchestrator_top(config),
        TeamsLayout::OrchestratorLeft => layout_orchestrator_left(config),
        TeamsLayout::EqualGrid => layout_equal_grid(config),
        TeamsLayout::Custom => Vec::new(),
    }
}

/// Orchestrator on top (horizontal), workers split vertically below.
fn layout_orchestrator_top(config: &TeamsConfig) -> Vec<PaneConfig> {
    let mut panes = Vec::with_capacity(1 + config.worker_count as usize);

    // The orchestrator is the first pane — horizontal direction means it
    // occupies the full width at the top.
    panes.push(orchestrator_pane(
        &config.orchestrator_agent,
        PaneDirection::Horizontal,
    ));

    // Workers split vertically below the orchestrator.
    for i in 1..=config.worker_count {
        panes.push(worker_pane(
            i,
            &config.worker_agent,
            PaneDirection::Vertical,
        ));
    }

    panes
}

/// Orchestrator on the left (vertical), workers split horizontally to the right.
fn layout_orchestrator_left(config: &TeamsConfig) -> Vec<PaneConfig> {
    let mut panes = Vec::with_capacity(1 + config.worker_count as usize);

    // Orchestrator occupies the full height on the left.
    panes.push(orchestrator_pane(
        &config.orchestrator_agent,
        PaneDirection::Vertical,
    ));

    // Workers split horizontally to the right.
    for i in 1..=config.worker_count {
        panes.push(worker_pane(
            i,
            &config.worker_agent,
            PaneDirection::Horizontal,
        ));
    }

    panes
}

/// Equal grid — no special orchestrator, alternating splits.
fn layout_equal_grid(config: &TeamsConfig) -> Vec<PaneConfig> {
    let total = config.worker_count + 1; // worker_count + 1 panes
    let mut panes = Vec::with_capacity(total as usize);

    for i in 0..total {
        let direction = if i % 2 == 0 {
            PaneDirection::Horizontal
        } else {
            PaneDirection::Vertical
        };
        // In equal-grid, all panes use the worker agent and worker naming.
        // The first pane is "worker-1", etc.
        panes.push(worker_pane(i + 1, &config.worker_agent, direction));
    }

    panes
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_orchestrator_top_layout() {
        let config = TeamsConfig {
            default_layout: TeamsLayout::OrchestratorTop,
            orchestrator_agent: "claude".to_string(),
            worker_agent: "codex".to_string(),
            worker_count: 3,
        };
        let panes = generate_team_layout(&config, "/home/dev/project");

        // Total panes = 1 orchestrator + 3 workers
        assert_eq!(panes.len(), 4);

        // First pane is the orchestrator
        assert_eq!(panes[0].name, "orchestrator");
        assert_eq!(panes[0].agent, "claude");
        assert_eq!(panes[0].direction, PaneDirection::Horizontal);

        // Workers are vertical splits
        for (i, pane) in panes[1..].iter().enumerate() {
            assert_eq!(pane.name, format!("worker-{}", i + 1));
            assert_eq!(pane.agent, "codex");
            assert_eq!(pane.direction, PaneDirection::Vertical);
        }
    }

    #[test]
    fn test_orchestrator_left_layout() {
        let config = TeamsConfig {
            default_layout: TeamsLayout::OrchestratorLeft,
            orchestrator_agent: "gemini".to_string(),
            worker_agent: "aider".to_string(),
            worker_count: 2,
        };
        let panes = generate_team_layout(&config, "/tmp");

        // Total = 1 orchestrator + 2 workers
        assert_eq!(panes.len(), 3);

        // Orchestrator is vertical (full height on left)
        assert_eq!(panes[0].name, "orchestrator");
        assert_eq!(panes[0].agent, "gemini");
        assert_eq!(panes[0].direction, PaneDirection::Vertical);

        // Workers are horizontal splits
        for (i, pane) in panes[1..].iter().enumerate() {
            assert_eq!(pane.name, format!("worker-{}", i + 1));
            assert_eq!(pane.agent, "aider");
            assert_eq!(pane.direction, PaneDirection::Horizontal);
        }
    }

    #[test]
    fn test_equal_grid_layout() {
        let config = TeamsConfig {
            default_layout: TeamsLayout::EqualGrid,
            orchestrator_agent: "claude".to_string(),
            worker_agent: "claude".to_string(),
            worker_count: 3,
        };
        let panes = generate_team_layout(&config, "/home/dev");

        // Total = worker_count + 1 = 4
        assert_eq!(panes.len(), 4);

        // All panes are workers (no special orchestrator)
        for (i, pane) in panes.iter().enumerate() {
            assert_eq!(pane.name, format!("worker-{}", i + 1));
            assert_eq!(pane.agent, "claude");
        }

        // Alternating directions: H, V, H, V
        assert_eq!(panes[0].direction, PaneDirection::Horizontal);
        assert_eq!(panes[1].direction, PaneDirection::Vertical);
        assert_eq!(panes[2].direction, PaneDirection::Horizontal);
        assert_eq!(panes[3].direction, PaneDirection::Vertical);
    }

    #[test]
    fn test_custom_returns_empty() {
        let config = TeamsConfig {
            default_layout: TeamsLayout::Custom,
            orchestrator_agent: "claude".to_string(),
            worker_agent: "claude".to_string(),
            worker_count: 5,
        };
        let panes = generate_team_layout(&config, "/home/dev");
        assert!(panes.is_empty());
    }

    #[test]
    fn test_default_config_layout() {
        let config = TeamsConfig::default();
        let panes = generate_team_layout(&config, "/home/dev");

        // Default: OrchestratorTop, orchestrator=claude, worker=claude, worker_count=2
        assert_eq!(panes.len(), 3); // 1 orchestrator + 2 workers

        assert_eq!(panes[0].name, "orchestrator");
        assert_eq!(panes[0].agent, "claude");
        assert_eq!(panes[0].direction, PaneDirection::Horizontal);

        assert_eq!(panes[1].name, "worker-1");
        assert_eq!(panes[1].agent, "claude");
        assert_eq!(panes[1].direction, PaneDirection::Vertical);

        assert_eq!(panes[2].name, "worker-2");
        assert_eq!(panes[2].agent, "claude");
        assert_eq!(panes[2].direction, PaneDirection::Vertical);
    }

    #[test]
    fn test_zero_workers_orchestrator_top() {
        let config = TeamsConfig {
            default_layout: TeamsLayout::OrchestratorTop,
            orchestrator_agent: "claude".to_string(),
            worker_agent: "claude".to_string(),
            worker_count: 0,
        };
        let panes = generate_team_layout(&config, "/home/dev");

        // Only the orchestrator pane
        assert_eq!(panes.len(), 1);
        assert_eq!(panes[0].name, "orchestrator");
    }

    #[test]
    fn test_zero_workers_orchestrator_left() {
        let config = TeamsConfig {
            default_layout: TeamsLayout::OrchestratorLeft,
            orchestrator_agent: "claude".to_string(),
            worker_agent: "claude".to_string(),
            worker_count: 0,
        };
        let panes = generate_team_layout(&config, "/home/dev");

        assert_eq!(panes.len(), 1);
        assert_eq!(panes[0].name, "orchestrator");
    }

    #[test]
    fn test_zero_workers_equal_grid() {
        let config = TeamsConfig {
            default_layout: TeamsLayout::EqualGrid,
            orchestrator_agent: "claude".to_string(),
            worker_agent: "claude".to_string(),
            worker_count: 0,
        };
        let panes = generate_team_layout(&config, "/home/dev");

        // worker_count + 1 = 1 pane
        assert_eq!(panes.len(), 1);
        assert_eq!(panes[0].name, "worker-1");
    }

    #[test]
    fn test_pane_commands() {
        let config = TeamsConfig {
            default_layout: TeamsLayout::OrchestratorTop,
            orchestrator_agent: "claude".to_string(),
            worker_agent: "codex".to_string(),
            worker_count: 1,
        };
        let panes = generate_team_layout(&config, "/home/dev");

        // Orchestrator command
        assert_eq!(
            panes[0].command,
            vec!["zellai", "run", "--agent", "claude", "--", "claude"]
        );

        // Worker command
        assert_eq!(
            panes[1].command,
            vec!["zellai", "run", "--agent", "codex", "--", "codex"]
        );
    }

    #[test]
    fn test_pane_prompts_are_none() {
        // All generated panes should have prompt = None (future: may be set from TOML)
        let config = TeamsConfig::default();
        let panes = generate_team_layout(&config, "/home/dev");
        for pane in &panes {
            assert!(pane.prompt.is_none());
        }
    }

    #[test]
    fn test_large_worker_count() {
        let config = TeamsConfig {
            default_layout: TeamsLayout::OrchestratorTop,
            orchestrator_agent: "claude".to_string(),
            worker_agent: "gemini".to_string(),
            worker_count: 10,
        };
        let panes = generate_team_layout(&config, "/home/dev");

        assert_eq!(panes.len(), 11); // 1 orchestrator + 10 workers

        // Verify worker naming is sequential
        for i in 1..=10 {
            assert_eq!(panes[i].name, format!("worker-{}", i));
        }
    }
}
