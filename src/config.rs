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

/// Configuration for the teams / multi-agent layout.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct TeamsConfig {
    pub default_layout: TeamsLayout,
    pub orchestrator_agent: String,
    pub worker_agent: String,
    pub worker_count: u32,
}

impl Default for TeamsConfig {
    fn default() -> Self {
        Self {
            default_layout: TeamsLayout::default(),
            orchestrator_agent: "claude".to_string(),
            worker_agent: "claude".to_string(),
            worker_count: 2,
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
}
