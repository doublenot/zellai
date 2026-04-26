//! Standalone screenshot binary — generates terminal text output of the zellai
//! sidebar and status bar using sample agent data.
//!
//! This binary compiles for the **host** target (not WASM) and exercises only
//! the pure-logic rendering modules from the `zellai` library crate.
//!
//! Usage:
//!   cargo run --bin screenshot

use zellai::config::SidebarConfig;
use zellai::sidebar::render_sidebar;
use zellai::status::{AgentKind, AgentStatus, AgentStatusValue};
use zellai::status_bar::render_status_bar;

fn sample_agents() -> Vec<AgentStatus> {
    vec![
        AgentStatus {
            version: 1,
            session_id: "claude-backend-001".to_string(),
            agent: AgentKind::Claude,
            status: AgentStatusValue::Thinking,
            git_branch: Some("feat/api-v2".to_string()),
            git_dirty: true,
            working_dir: "/home/dev/projects/backend".to_string(),
            last_message: Some("Using Edit to update auth handler…".to_string()),
            ports: vec![8080],
            pr_number: Some(142),
            pr_ci_status: None,
            needs_attention: false,
            updated_at: 1745467200,
            pane_id: None,
        },
        AgentStatus {
            version: 1,
            session_id: "codex-frontend-002".to_string(),
            agent: AgentKind::Codex,
            status: AgentStatusValue::Waiting,
            git_branch: Some("fix/login-ui".to_string()),
            git_dirty: false,
            working_dir: "/home/dev/projects/frontend".to_string(),
            last_message: Some("Needs input on auth flow".to_string()),
            ports: vec![3000, 5173],
            pr_number: Some(87),
            pr_ci_status: None,
            needs_attention: true,
            updated_at: 1745467200,
            pane_id: None,
        },
        AgentStatus {
            version: 1,
            session_id: "aider-tests-003".to_string(),
            agent: AgentKind::Aider,
            status: AgentStatusValue::Thinking,
            git_branch: Some("main".to_string()),
            git_dirty: false,
            working_dir: "/home/dev/projects/tests".to_string(),
            last_message: Some("Running test suite…".to_string()),
            ports: vec![],
            pr_number: None,
            pr_ci_status: None,
            needs_attention: false,
            updated_at: 1745467200,
            pane_id: None,
        },
        AgentStatus {
            version: 1,
            session_id: "claude-docs-004".to_string(),
            agent: AgentKind::Claude,
            status: AgentStatusValue::Idle,
            git_branch: Some("docs/readme".to_string()),
            git_dirty: false,
            working_dir: "/home/dev/projects/docs".to_string(),
            last_message: None,
            ports: vec![],
            pr_number: None,
            pr_ci_status: None,
            needs_attention: false,
            updated_at: 1745467200,
            pane_id: None,
        },
        AgentStatus {
            version: 1,
            session_id: "gemini-refactor-005".to_string(),
            agent: AgentKind::Gemini,
            status: AgentStatusValue::Error,
            git_branch: Some("refactor/db".to_string()),
            git_dirty: true,
            working_dir: "/home/dev/projects/core".to_string(),
            last_message: Some("Build failed: 3 errors".to_string()),
            ports: vec![],
            pr_number: None,
            pr_ci_status: None,
            // Note: validate() would force this to false for Error status,
            // but for the screenshot we want to show the attention indicator.
            needs_attention: true,
            updated_at: 1745467200,
            pane_id: None,
        },
    ]
}

fn main() {
    let agents = sample_agents();
    let agent_refs: Vec<&AgentStatus> = agents.iter().collect();

    let config = SidebarConfig::default();
    let cols: usize = 38;
    let rows: usize = 17;

    // Render sidebar
    let sidebar_lines = render_sidebar(&agent_refs, &config, rows, cols, None);

    println!("┌─────────────────────────────────────────┐");
    println!("│  zellai — AI agent workspace for Zellij  │");
    println!("└─────────────────────────────────────────┘");
    println!();

    for line in &sidebar_lines {
        println!("{}", line);
    }

    println!();

    // Render status bar
    let status_bar = render_status_bar(&agent_refs, "my-project", 60);
    println!("Status bar: {}", status_bar);
}
