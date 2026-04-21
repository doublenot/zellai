use std::collections::HashMap;

use crate::status::{parse_status, AgentStatus, AgentStatusValue};

/// Pure data-management layer for tracking active agent sessions.
///
/// `StatusBridge` sits between file I/O events (handled by `lib.rs`) and
/// rendering (handled by `sidebar.rs` in a future task). It holds no Zellij
/// API dependencies — all I/O is driven externally.
pub struct StatusBridge {
    agents: HashMap<String, AgentStatus>,
    stale_threshold_s: u64,
    sessions_dir: String,
}

impl StatusBridge {
    /// Create a new `StatusBridge`.
    ///
    /// - `sessions_dir`: path to the directory containing session status JSON files.
    /// - `stale_threshold_s`: seconds after which an agent is considered stale.
    pub fn new(sessions_dir: &str, stale_threshold_s: u64) -> Self {
        Self {
            agents: HashMap::new(),
            stale_threshold_s,
            sessions_dir: sessions_dir.to_string(),
        }
    }

    /// Parse `json` as an `AgentStatus` and store/replace it under `session_id`.
    ///
    /// Returns `Err` with a human-readable description on parse failure.
    /// On failure the map is left unchanged.
    pub fn update_from_json(&mut self, session_id: &str, json: &str) -> Result<(), String> {
        let status =
            parse_status(json).map_err(|e| format!("failed to parse status for {session_id}: {e}"))?;
        self.agents.insert(session_id.to_string(), status);
        Ok(())
    }

    /// Remove a session from the map (e.g. when its status file is deleted).
    ///
    /// No-op if the session doesn't exist.
    pub fn remove_session(&mut self, session_id: &str) {
        self.agents.remove(session_id);
    }

    /// Mark stale agents as `Idle` with `needs_attention = false`.
    ///
    /// `now_epoch` is the current Unix timestamp in seconds. Any agent whose
    /// `updated_at` is more than `stale_threshold_s` seconds in the past is
    /// transitioned to Idle.
    pub fn mark_stale(&mut self, now_epoch: u64) {
        for agent in self.agents.values_mut() {
            if agent.is_stale(now_epoch, self.stale_threshold_s) {
                agent.status = AgentStatusValue::Idle;
                agent.needs_attention = false;
            }
        }
    }

    /// Return agents sorted for sidebar display.
    ///
    /// Ordering: agents with `needs_attention == true` come first, then the
    /// rest. Within each group, agents are sorted alphabetically by `session_id`.
    pub fn agents_sorted(&self) -> Vec<&AgentStatus> {
        let mut agents: Vec<&AgentStatus> = self.agents.values().collect();
        agents.sort_by(|a, b| {
            // needs_attention == true sorts before false (reverse bool order)
            b.needs_attention
                .cmp(&a.needs_attention)
                .then_with(|| a.session_id.cmp(&b.session_id))
        });
        agents
    }

    /// Return all tracked session IDs.
    pub fn session_ids(&self) -> Vec<String> {
        self.agents.keys().cloned().collect()
    }

    /// Returns `true` if at least one agent is tracked.
    pub fn has_agents(&self) -> bool {
        !self.agents.is_empty()
    }

    /// The configured sessions directory path.
    pub fn sessions_dir(&self) -> &str {
        &self.sessions_dir
    }

    /// The configured stale threshold in seconds.
    pub fn stale_threshold_s(&self) -> u64 {
        self.stale_threshold_s
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::status::{AgentKind, AgentStatusValue};

    /// Helper: return valid agent status JSON.
    fn make_json(session_id: &str, status: &str, updated_at: u64) -> String {
        format!(
            r#"{{
                "version": 1,
                "session_id": "{session_id}",
                "agent": "claude",
                "status": "{status}",
                "git_branch": "main",
                "git_dirty": false,
                "working_dir": "/tmp",
                "last_message": null,
                "ports": [],
                "needs_attention": false,
                "updated_at": {updated_at}
            }}"#
        )
    }

    fn new_bridge() -> StatusBridge {
        StatusBridge::new("/tmp/zellai/sessions", 60)
    }

    // -----------------------------------------------------------------------
    // update_from_json
    // -----------------------------------------------------------------------

    #[test]
    fn test_update_from_json_valid() {
        let mut bridge = new_bridge();
        let json = make_json("sess-1", "thinking", 1000);
        let result = bridge.update_from_json("sess-1", &json);
        assert!(result.is_ok());
        assert!(bridge.has_agents());
        let agents = bridge.agents_sorted();
        assert_eq!(agents.len(), 1);
        assert_eq!(agents[0].session_id, "sess-1");
        assert_eq!(agents[0].agent, AgentKind::Claude);
        assert_eq!(agents[0].status, AgentStatusValue::Thinking);
    }

    #[test]
    fn test_update_from_json_invalid() {
        let mut bridge = new_bridge();
        let result = bridge.update_from_json("sess-bad", "not json!!!");
        assert!(result.is_err());
        assert!(!bridge.has_agents());
    }

    #[test]
    fn test_update_from_json_replaces_existing() {
        let mut bridge = new_bridge();
        let json1 = make_json("sess-1", "thinking", 1000);
        bridge.update_from_json("sess-1", &json1).unwrap();
        assert_eq!(bridge.agents_sorted()[0].status, AgentStatusValue::Thinking);

        let json2 = make_json("sess-1", "idle", 2000);
        bridge.update_from_json("sess-1", &json2).unwrap();
        let agents = bridge.agents_sorted();
        assert_eq!(agents.len(), 1);
        assert_eq!(agents[0].status, AgentStatusValue::Idle);
        assert_eq!(agents[0].updated_at, 2000);
    }

    // -----------------------------------------------------------------------
    // remove_session
    // -----------------------------------------------------------------------

    #[test]
    fn test_remove_session() {
        let mut bridge = new_bridge();
        bridge
            .update_from_json("sess-1", &make_json("sess-1", "thinking", 1000))
            .unwrap();
        assert!(bridge.has_agents());

        bridge.remove_session("sess-1");
        assert!(!bridge.has_agents());
    }

    #[test]
    fn test_remove_session_nonexistent_is_noop() {
        let mut bridge = new_bridge();
        bridge.remove_session("does-not-exist"); // should not panic
        assert!(!bridge.has_agents());
    }

    // -----------------------------------------------------------------------
    // mark_stale
    // -----------------------------------------------------------------------

    #[test]
    fn test_mark_stale_transitions_old_agents() {
        let mut bridge = new_bridge(); // stale_threshold_s = 60
        bridge
            .update_from_json("sess-1", &make_json("sess-1", "thinking", 100))
            .unwrap();

        // At now=200, the agent is 100s old (> 60s threshold) → stale
        bridge.mark_stale(200);
        let agents = bridge.agents_sorted();
        assert_eq!(agents[0].status, AgentStatusValue::Idle);
        assert!(!agents[0].needs_attention);
    }

    #[test]
    fn test_mark_stale_leaves_fresh_agents() {
        let mut bridge = new_bridge(); // stale_threshold_s = 60
        bridge
            .update_from_json("sess-1", &make_json("sess-1", "thinking", 100))
            .unwrap();

        // At now=130, the agent is 30s old (< 60s threshold) → still fresh
        bridge.mark_stale(130);
        let agents = bridge.agents_sorted();
        assert_eq!(agents[0].status, AgentStatusValue::Thinking);
    }

    // -----------------------------------------------------------------------
    // agents_sorted
    // -----------------------------------------------------------------------

    #[test]
    fn test_agents_sorted_needs_attention_first() {
        let mut bridge = new_bridge();

        // "alpha" is thinking (needs_attention = false)
        bridge
            .update_from_json("alpha", &make_json("alpha", "thinking", 1000))
            .unwrap();

        // "beta" is waiting (needs_attention = true, forced by validate())
        bridge
            .update_from_json("beta", &make_json("beta", "waiting", 1000))
            .unwrap();

        // "gamma" is idle (needs_attention = false)
        bridge
            .update_from_json("gamma", &make_json("gamma", "idle", 1000))
            .unwrap();

        let sorted = bridge.agents_sorted();
        assert_eq!(sorted.len(), 3);

        // beta (needs_attention=true) should come first
        assert_eq!(sorted[0].session_id, "beta");
        assert!(sorted[0].needs_attention);

        // Then alpha and gamma alphabetically
        assert_eq!(sorted[1].session_id, "alpha");
        assert_eq!(sorted[2].session_id, "gamma");
    }

    // -----------------------------------------------------------------------
    // has_agents
    // -----------------------------------------------------------------------

    #[test]
    fn test_has_agents_empty() {
        let bridge = new_bridge();
        assert!(!bridge.has_agents());
    }

    #[test]
    fn test_has_agents_populated() {
        let mut bridge = new_bridge();
        bridge
            .update_from_json("sess-1", &make_json("sess-1", "idle", 1000))
            .unwrap();
        assert!(bridge.has_agents());
    }

    // -----------------------------------------------------------------------
    // session_ids
    // -----------------------------------------------------------------------

    #[test]
    fn test_session_ids() {
        let mut bridge = new_bridge();
        bridge
            .update_from_json("sess-a", &make_json("sess-a", "idle", 1000))
            .unwrap();
        bridge
            .update_from_json("sess-b", &make_json("sess-b", "thinking", 1000))
            .unwrap();

        let mut ids = bridge.session_ids();
        ids.sort();
        assert_eq!(ids, vec!["sess-a".to_string(), "sess-b".to_string()]);
    }

    // -----------------------------------------------------------------------
    // Constructor / accessors
    // -----------------------------------------------------------------------

    #[test]
    fn test_constructor_defaults() {
        let bridge = StatusBridge::new("~/.local/share/zellai/sessions", 60);
        assert_eq!(bridge.sessions_dir(), "~/.local/share/zellai/sessions");
        assert_eq!(bridge.stale_threshold_s(), 60);
        assert!(!bridge.has_agents());
    }
}
