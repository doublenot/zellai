use serde::{Deserialize, Serialize};
use std::fmt;

/// The type of AI coding agent.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AgentKind {
    Claude,
    Codex,
    Gemini,
    Aider,
    Opencode,
    #[serde(other)]
    Unknown,
}

impl fmt::Display for AgentKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AgentKind::Claude => write!(f, "Claude"),
            AgentKind::Codex => write!(f, "Codex"),
            AgentKind::Gemini => write!(f, "Gemini"),
            AgentKind::Aider => write!(f, "Aider"),
            AgentKind::Opencode => write!(f, "Opencode"),
            AgentKind::Unknown => write!(f, "Unknown"),
        }
    }
}

/// The current status of an agent session.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AgentStatusValue {
    Thinking,
    Waiting,
    Idle,
    Error,
}

impl fmt::Display for AgentStatusValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AgentStatusValue::Thinking => write!(f, "Thinking"),
            AgentStatusValue::Waiting => write!(f, "Waiting"),
            AgentStatusValue::Idle => write!(f, "Idle"),
            AgentStatusValue::Error => write!(f, "Error"),
        }
    }
}

/// CI status for a linked pull request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CiStatus {
    Passing,
    Failing,
    Pending,
}

/// The main data model for an agent's status, matching the JSON schema in SCHEMA.md.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AgentStatus {
    pub version: u32,
    pub session_id: String,
    pub agent: AgentKind,
    pub status: AgentStatusValue,
    pub git_branch: Option<String>,
    pub git_dirty: bool,
    pub working_dir: String,
    pub last_message: Option<String>,
    pub ports: Vec<u16>,
    #[serde(default)]
    pub pr_number: Option<u32>,
    #[serde(default)]
    pub pr_ci_status: Option<CiStatus>,
    pub needs_attention: bool,
    pub updated_at: u64,
}

impl AgentStatus {
    /// Enforce invariants from SCHEMA.md:
    /// - `needs_attention` must be `true` if and only if `status == Waiting`
    pub fn validate(&mut self) {
        match self.status {
            AgentStatusValue::Waiting => {
                self.needs_attention = true;
            }
            _ => {
                self.needs_attention = false;
            }
        }
    }

    /// Returns true if the status is stale (not updated within the threshold).
    pub fn is_stale(&self, now_epoch: u64, threshold_s: u64) -> bool {
        now_epoch.saturating_sub(self.updated_at) > threshold_s
    }
}

/// Parse a JSON string into an `AgentStatus`, applying validation.
pub fn parse_status(json: &str) -> Result<AgentStatus, serde_json::Error> {
    let mut status: AgentStatus = serde_json::from_str(json)?;
    status.validate();
    Ok(status)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn full_json() -> String {
        r#"{
            "version": 1,
            "session_id": "hostname-pane-42",
            "agent": "claude",
            "status": "thinking",
            "git_branch": "feat/auth",
            "git_dirty": false,
            "working_dir": "/home/user/projects/app",
            "last_message": "Reading src/auth.ts…",
            "ports": [3000, 5173],
            "pr_number": 42,
            "pr_ci_status": "passing",
            "needs_attention": false,
            "updated_at": 1706000101
        }"#
        .to_string()
    }

    #[test]
    fn test_parse_valid_status() {
        let status = parse_status(&full_json()).expect("should parse valid JSON");
        assert_eq!(status.version, 1);
        assert_eq!(status.session_id, "hostname-pane-42");
        assert_eq!(status.agent, AgentKind::Claude);
        assert_eq!(status.status, AgentStatusValue::Thinking);
        assert_eq!(status.git_branch, Some("feat/auth".to_string()));
        assert!(!status.git_dirty);
        assert_eq!(status.working_dir, "/home/user/projects/app");
        assert_eq!(
            status.last_message,
            Some("Reading src/auth.ts…".to_string())
        );
        assert_eq!(status.ports, vec![3000, 5173]);
        assert_eq!(status.pr_number, Some(42));
        assert_eq!(status.pr_ci_status, Some(CiStatus::Passing));
        // needs_attention is forced false because status is Thinking (not Waiting)
        assert!(!status.needs_attention);
        assert_eq!(status.updated_at, 1706000101);
    }

    #[test]
    fn test_parse_minimal_status() {
        let json = r#"{
            "version": 1,
            "session_id": "host-1",
            "agent": "codex",
            "status": "idle",
            "git_branch": null,
            "git_dirty": true,
            "working_dir": "/tmp",
            "last_message": null,
            "ports": [],
            "needs_attention": false,
            "updated_at": 1000
        }"#;
        let status = parse_status(json).expect("should parse minimal JSON");
        assert_eq!(status.agent, AgentKind::Codex);
        assert_eq!(status.status, AgentStatusValue::Idle);
        assert!(status.git_branch.is_none());
        assert!(status.git_dirty);
        assert!(status.last_message.is_none());
        assert!(status.ports.is_empty());
        // Optional fields default to None
        assert_eq!(status.pr_number, None);
        assert_eq!(status.pr_ci_status, None);
    }

    #[test]
    fn test_validate_forces_needs_attention_false() {
        // status=thinking with needs_attention=true in JSON → validate forces false
        let json = r#"{
            "version": 1,
            "session_id": "host-2",
            "agent": "claude",
            "status": "thinking",
            "git_branch": null,
            "git_dirty": false,
            "working_dir": "/tmp",
            "last_message": null,
            "ports": [],
            "needs_attention": true,
            "updated_at": 1000
        }"#;
        let status = parse_status(json).expect("should parse");
        assert!(!status.needs_attention);
    }

    #[test]
    fn test_validate_forces_needs_attention_true() {
        // status=waiting with needs_attention=false in JSON → validate forces true
        let json = r#"{
            "version": 1,
            "session_id": "host-3",
            "agent": "claude",
            "status": "waiting",
            "git_branch": null,
            "git_dirty": false,
            "working_dir": "/tmp",
            "last_message": null,
            "ports": [],
            "needs_attention": false,
            "updated_at": 1000
        }"#;
        let status = parse_status(json).expect("should parse");
        assert!(status.needs_attention);
    }

    #[test]
    fn test_stale_detection() {
        let mut status = parse_status(&full_json()).unwrap();
        status.updated_at = 100;
        assert!(status.is_stale(200, 60));
    }

    #[test]
    fn test_not_stale() {
        let mut status = parse_status(&full_json()).unwrap();
        status.updated_at = 100;
        assert!(!status.is_stale(130, 60));
    }

    #[test]
    fn test_parse_invalid_json() {
        let result = parse_status("this is not json");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_unknown_agent() {
        // An unknown agent string should deserialize to AgentKind::Unknown via #[serde(other)]
        let json = r#"{
            "version": 1,
            "session_id": "host-4",
            "agent": "cursor",
            "status": "idle",
            "git_branch": null,
            "git_dirty": false,
            "working_dir": "/tmp",
            "last_message": null,
            "ports": [],
            "needs_attention": false,
            "updated_at": 1000
        }"#;
        let status = parse_status(json).expect("unknown agent should parse to Unknown");
        assert_eq!(status.agent, AgentKind::Unknown);
    }
}
