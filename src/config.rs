use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Enums
// ---------------------------------------------------------------------------

/// Position of the sidebar pane.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SidebarPosition {
    #[default]
    Left, // YOYO.md mandates left as default
    Right,
    Bottom,
}

/// Card density for agent cards in the sidebar.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CardDensity {
    Compact,
    Detailed,
    #[default]
    Adaptive, // YOYO.md mandates adaptive as default
}

/// Layout style for the teams view.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TeamsLayout {
    #[default]
    OrchestratorTop, // YOYO.md mandates orchestrator-top as default
    OrchestratorLeft,
    EqualGrid,
    Custom,
}

// ---------------------------------------------------------------------------
// Section structs
// ---------------------------------------------------------------------------

/// Configuration for the sidebar pane.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct SidebarConfig {
    pub position: SidebarPosition,
    pub card_density: CardDensity,
    pub attention_animation: bool,
}

impl Default for SidebarConfig {
    fn default() -> Self {
        Self {
            position: SidebarPosition::default(),
            card_density: CardDensity::default(),
            attention_animation: true, // YOYO.md mandates true
        }
    }
}

/// Configuration for the orchestrator pane's task board.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct OrchestratorConfig {
    /// Whether the task board is enabled
    pub task_board: bool,
    /// Kanban column names
    pub task_board_columns: Vec<String>,
    /// Whether to show cost/token tracking
    pub show_cost_tracking: bool,
    /// Whether to show the DAG dependency view
    pub dag_view: bool,
}

impl Default for OrchestratorConfig {
    fn default() -> Self {
        Self {
            task_board: false, // off by default — opt-in feature
            task_board_columns: vec![
                "todo".to_string(),
                "in-progress".to_string(),
                "review".to_string(),
                "done".to_string(),
                "blocked".to_string(),
            ],
            show_cost_tracking: false,
            dag_view: true,
        }
    }
}

/// Configuration for the teams / multi-agent layout.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct TeamsConfig {
    pub default_layout: TeamsLayout,
    pub orchestrator_agent: String,
    pub worker_agent: String,
    pub worker_count: u32,
    pub orchestrator: OrchestratorConfig,
}

impl Default for TeamsConfig {
    fn default() -> Self {
        Self {
            default_layout: TeamsLayout::default(),
            orchestrator_agent: "claude".to_string(),
            worker_agent: "claude".to_string(),
            worker_count: 2,
            orchestrator: OrchestratorConfig::default(),
        }
    }
}

/// Configuration for the status-file bridge (IPC layer).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct BridgeConfig {
    pub sessions_dir: String,
    pub poll_interval_ms: u64,
    pub stale_threshold_s: u64,
}

impl Default for BridgeConfig {
    fn default() -> Self {
        Self {
            sessions_dir: "~/.local/share/zellai/sessions".to_string(),
            poll_interval_ms: 500,
            stale_threshold_s: 60,
        }
    }
}

// ---------------------------------------------------------------------------
// Keybindings
// ---------------------------------------------------------------------------

/// Configuration for keyboard shortcuts.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct KeybindingsConfig {
    pub next_attention: String,
    pub dismiss: String,
    pub jump_to: String,
}

impl Default for KeybindingsConfig {
    fn default() -> Self {
        Self {
            next_attention: "Ctrl a".to_string(),
            dismiss: "Ctrl d".to_string(),
            jump_to: "Ctrl g".to_string(),
        }
    }
}

/// Parse a keybinding string into `(has_ctrl, char)`.
///
/// Accepts formats:
/// - `"Ctrl x"` → `Some((true, 'x'))`
/// - `"x"` → `Some((false, 'x'))`
///
/// Returns `None` for empty strings, incomplete modifiers, or multi-char keys.
pub fn parse_key(s: &str) -> Option<(bool, char)> {
    let s = s.trim();
    if s.is_empty() {
        return None;
    }

    if let Some(rest) = s.strip_prefix("Ctrl ") {
        let rest = rest.trim();
        let mut chars = rest.chars();
        let ch = chars.next()?;
        if chars.next().is_some() {
            // More than one character after "Ctrl "
            return None;
        }
        Some((true, ch))
    } else if s == "Ctrl" {
        // Incomplete modifier
        None
    } else {
        let mut chars = s.chars();
        let ch = chars.next()?;
        if chars.next().is_some() {
            // Multi-char key without recognized modifier
            return None;
        }
        Some((false, ch))
    }
}

// ---------------------------------------------------------------------------
// Top-level config
// ---------------------------------------------------------------------------

/// Top-level zellai configuration, deserialized from `zellai.toml`.
///
/// Every field has a sensible default so the plugin functions correctly
/// even when no config file is present (`ZellaiConfig::default()`).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ZellaiConfig {
    pub sidebar: SidebarConfig,
    pub teams: TeamsConfig,
    pub bridge: BridgeConfig,
    pub keybindings: KeybindingsConfig,
}

// ---------------------------------------------------------------------------
// Parsing
// ---------------------------------------------------------------------------

/// Parse a TOML string into a [`ZellaiConfig`].
///
/// Missing keys are filled with defaults thanks to `#[serde(default)]`.
/// This function does **not** read from disk — the caller is responsible for
/// obtaining the TOML content (e.g. via Zellij's `run_command` API).
pub fn parse_config(toml_str: &str) -> Result<ZellaiConfig, toml::de::Error> {
    toml::from_str(toml_str)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let cfg = ZellaiConfig::default();

        // Sidebar defaults (mandated by YOYO.md)
        assert_eq!(cfg.sidebar.position, SidebarPosition::Left);
        assert_eq!(cfg.sidebar.card_density, CardDensity::Adaptive);
        assert!(cfg.sidebar.attention_animation);

        // Teams defaults
        assert_eq!(cfg.teams.default_layout, TeamsLayout::OrchestratorTop);
        assert_eq!(cfg.teams.orchestrator_agent, "claude");
        assert_eq!(cfg.teams.worker_agent, "claude");
        assert_eq!(cfg.teams.worker_count, 2);

        // Orchestrator defaults
        assert!(!cfg.teams.orchestrator.task_board); // off by default
        assert_eq!(cfg.teams.orchestrator.task_board_columns.len(), 5);
        assert_eq!(cfg.teams.orchestrator.task_board_columns[0], "todo");
        assert_eq!(cfg.teams.orchestrator.task_board_columns[1], "in-progress");
        assert_eq!(cfg.teams.orchestrator.task_board_columns[2], "review");
        assert_eq!(cfg.teams.orchestrator.task_board_columns[3], "done");
        assert_eq!(cfg.teams.orchestrator.task_board_columns[4], "blocked");
        assert!(!cfg.teams.orchestrator.show_cost_tracking);
        assert!(cfg.teams.orchestrator.dag_view);

        // Bridge defaults
        assert_eq!(cfg.bridge.sessions_dir, "~/.local/share/zellai/sessions");
        assert_eq!(cfg.bridge.poll_interval_ms, 500);
        assert_eq!(cfg.bridge.stale_threshold_s, 60);
    }

    #[test]
    fn test_parse_empty_toml() {
        let cfg = parse_config("").expect("empty TOML should parse");
        assert_eq!(cfg, ZellaiConfig::default());
    }

    #[test]
    fn test_parse_partial_config() {
        let toml_str = r#"
[sidebar]
position = "right"
"#;
        let cfg = parse_config(toml_str).expect("partial TOML should parse");

        // Overridden value
        assert_eq!(cfg.sidebar.position, SidebarPosition::Right);

        // Everything else stays default
        assert_eq!(cfg.sidebar.card_density, CardDensity::Adaptive);
        assert!(cfg.sidebar.attention_animation);
        assert_eq!(cfg.teams, TeamsConfig::default());
        assert_eq!(cfg.bridge, BridgeConfig::default());
    }

    #[test]
    fn test_parse_full_config() {
        let toml_str = r#"
[sidebar]
position = "bottom"
card_density = "compact"
attention_animation = false

[teams]
default_layout = "equal-grid"
orchestrator_agent = "codex"
worker_agent = "gemini"
worker_count = 4

[bridge]
sessions_dir = "/tmp/zellai/sessions"
poll_interval_ms = 1000
stale_threshold_s = 120
"#;
        let cfg = parse_config(toml_str).expect("full TOML should parse");

        assert_eq!(cfg.sidebar.position, SidebarPosition::Bottom);
        assert_eq!(cfg.sidebar.card_density, CardDensity::Compact);
        assert!(!cfg.sidebar.attention_animation);

        assert_eq!(cfg.teams.default_layout, TeamsLayout::EqualGrid);
        assert_eq!(cfg.teams.orchestrator_agent, "codex");
        assert_eq!(cfg.teams.worker_agent, "gemini");
        assert_eq!(cfg.teams.worker_count, 4);

        assert_eq!(cfg.bridge.sessions_dir, "/tmp/zellai/sessions");
        assert_eq!(cfg.bridge.poll_interval_ms, 1000);
        assert_eq!(cfg.bridge.stale_threshold_s, 120);
    }

    #[test]
    fn test_parse_invalid_toml() {
        let result = parse_config("this is not valid TOML {{{{");
        assert!(result.is_err());
    }

    #[test]
    fn test_roundtrip_serialization() {
        let cfg = ZellaiConfig::default();
        let serialized = toml::to_string(&cfg).expect("should serialize");
        let deserialized = parse_config(&serialized).expect("should deserialize");
        assert_eq!(cfg, deserialized);
    }

    #[test]
    fn test_enum_serde_values() {
        // Verify that enum serialization uses the expected string forms
        let json = serde_json::to_string(&SidebarPosition::Left).unwrap();
        assert_eq!(json, r#""left""#);

        let json = serde_json::to_string(&CardDensity::Detailed).unwrap();
        assert_eq!(json, r#""detailed""#);

        let json = serde_json::to_string(&TeamsLayout::OrchestratorTop).unwrap();
        assert_eq!(json, r#""orchestrator-top""#);

        let json = serde_json::to_string(&TeamsLayout::OrchestratorLeft).unwrap();
        assert_eq!(json, r#""orchestrator-left""#);

        let json = serde_json::to_string(&TeamsLayout::EqualGrid).unwrap();
        assert_eq!(json, r#""equal-grid""#);
    }

    #[test]
    fn test_partial_section_defaults() {
        // Provide only one field in a section — rest should be default
        let toml_str = r#"
[teams]
worker_count = 5
"#;
        let cfg = parse_config(toml_str).expect("should parse");
        assert_eq!(cfg.teams.worker_count, 5);
        assert_eq!(cfg.teams.default_layout, TeamsLayout::OrchestratorTop);
        assert_eq!(cfg.teams.orchestrator_agent, "claude");
        assert_eq!(cfg.teams.worker_agent, "claude");
    }

    // -----------------------------------------------------------------------
    // Keybindings tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_default_keybindings() {
        let cfg = ZellaiConfig::default();
        assert_eq!(cfg.keybindings.next_attention, "Ctrl a");
        assert_eq!(cfg.keybindings.dismiss, "Ctrl d");
        assert_eq!(cfg.keybindings.jump_to, "Ctrl g");
    }

    #[test]
    fn test_parse_key_ctrl_a() {
        assert_eq!(parse_key("Ctrl a"), Some((true, 'a')));
    }

    #[test]
    fn test_parse_key_ctrl_d() {
        assert_eq!(parse_key("Ctrl d"), Some((true, 'd')));
    }

    #[test]
    fn test_parse_key_plain_char() {
        assert_eq!(parse_key("x"), Some((false, 'x')));
    }

    #[test]
    fn test_parse_key_empty() {
        assert_eq!(parse_key(""), None);
    }

    #[test]
    fn test_parse_key_incomplete_ctrl() {
        assert_eq!(parse_key("Ctrl"), None);
    }

    #[test]
    fn test_keybindings_roundtrip() {
        let cfg = ZellaiConfig::default();
        let serialized = toml::to_string(&cfg).expect("should serialize");
        let deserialized = parse_config(&serialized).expect("should deserialize");
        assert_eq!(cfg.keybindings, deserialized.keybindings);
    }

    #[test]
    fn test_partial_keybindings_override() {
        let toml_str = r#"
[keybindings]
dismiss = "Ctrl x"
"#;
        let cfg = parse_config(toml_str).expect("should parse");
        // Overridden
        assert_eq!(cfg.keybindings.dismiss, "Ctrl x");
        // Defaults preserved
        assert_eq!(cfg.keybindings.next_attention, "Ctrl a");
        assert_eq!(cfg.keybindings.jump_to, "Ctrl g");
    }

    #[test]
    fn test_full_config_with_keybindings() {
        let toml_str = r#"
[sidebar]
position = "right"
card_density = "compact"
attention_animation = false

[teams]
default_layout = "equal-grid"
orchestrator_agent = "codex"
worker_agent = "gemini"
worker_count = 3

[bridge]
sessions_dir = "/tmp/zellai/sessions"
poll_interval_ms = 1000
stale_threshold_s = 120

[keybindings]
next_attention = "Ctrl n"
dismiss = "Ctrl x"
jump_to = "Ctrl j"
"#;
        let cfg = parse_config(toml_str).expect("should parse");
        assert_eq!(cfg.keybindings.next_attention, "Ctrl n");
        assert_eq!(cfg.keybindings.dismiss, "Ctrl x");
        assert_eq!(cfg.keybindings.jump_to, "Ctrl j");
        // Verify other sections also parsed
        assert_eq!(cfg.sidebar.position, SidebarPosition::Right);
        assert_eq!(cfg.teams.worker_count, 3);
        assert_eq!(cfg.bridge.poll_interval_ms, 1000);
        // Orchestrator defaults preserved when not specified
        assert!(!cfg.teams.orchestrator.task_board);
        assert!(cfg.teams.orchestrator.dag_view);
    }

    // -----------------------------------------------------------------------
    // Orchestrator config tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_orchestrator_defaults_when_omitted() {
        let toml_str = r#"
[teams]
worker_count = 3
"#;
        let cfg = parse_config(toml_str).expect("should parse");
        assert_eq!(cfg.teams.worker_count, 3);
        assert_eq!(cfg.teams.orchestrator, OrchestratorConfig::default());
    }

    #[test]
    fn test_orchestrator_partial_config() {
        let toml_str = r#"
[teams.orchestrator]
task_board = true
show_cost_tracking = true
"#;
        let cfg = parse_config(toml_str).expect("should parse");
        assert!(cfg.teams.orchestrator.task_board);
        assert!(cfg.teams.orchestrator.show_cost_tracking);
        // Defaults preserved for unset fields
        assert!(cfg.teams.orchestrator.dag_view);
        assert_eq!(cfg.teams.orchestrator.task_board_columns.len(), 5);
    }

    #[test]
    fn test_orchestrator_full_config() {
        let toml_str = r#"
[teams]
default_layout = "orchestrator-left"
orchestrator_agent = "codex"
worker_agent = "gemini"
worker_count = 4

[teams.orchestrator]
task_board = true
task_board_columns = ["backlog", "doing", "done"]
show_cost_tracking = true
dag_view = false
"#;
        let cfg = parse_config(toml_str).expect("should parse");
        assert_eq!(cfg.teams.default_layout, TeamsLayout::OrchestratorLeft);
        assert_eq!(cfg.teams.orchestrator_agent, "codex");
        assert!(cfg.teams.orchestrator.task_board);
        assert_eq!(
            cfg.teams.orchestrator.task_board_columns,
            vec!["backlog", "doing", "done"]
        );
        assert!(cfg.teams.orchestrator.show_cost_tracking);
        assert!(!cfg.teams.orchestrator.dag_view);
    }

    #[test]
    fn test_orchestrator_custom_columns() {
        let toml_str = r#"
[teams.orchestrator]
task_board_columns = ["todo", "wip", "review", "shipped"]
"#;
        let cfg = parse_config(toml_str).expect("should parse");
        assert_eq!(cfg.teams.orchestrator.task_board_columns.len(), 4);
        assert_eq!(cfg.teams.orchestrator.task_board_columns[3], "shipped");
    }

    #[test]
    fn test_orchestrator_roundtrip() {
        let cfg = ZellaiConfig::default();
        let serialized = toml::to_string(&cfg).expect("should serialize");
        let deserialized = parse_config(&serialized).expect("should deserialize");
        assert_eq!(cfg.teams.orchestrator, deserialized.teams.orchestrator);
    }
}
