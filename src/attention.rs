use std::collections::HashSet;

use crate::status::AgentStatus;

/// Tracks attention state across agent sessions for keyboard navigation.
///
/// The sidebar renders attention indicators visually, but this module provides
/// the state tracking for cycling through sessions that need attention and
/// dismissing them via keyboard.
pub struct AttentionTracker {
    /// Ordered list of session IDs needing attention (rebuilt on each update).
    attention_ids: Vec<String>,
    /// Index into `attention_ids` for cycling. None = no selection.
    cursor: Option<usize>,
    /// Set of dismissed session IDs (user explicitly dismissed attention).
    dismissed: HashSet<String>,
}

impl AttentionTracker {
    /// Create a new empty tracker.
    pub fn new() -> Self {
        Self {
            attention_ids: Vec::new(),
            cursor: None,
            dismissed: HashSet::new(),
        }
    }

    /// Rebuild the attention list from current agent state.
    ///
    /// Filters to agents where `needs_attention == true` AND not in the
    /// dismissed set. Preserves cursor position if the currently-selected
    /// session is still in the list. Sorts by session_id for stable ordering.
    pub fn update(&mut self, agents: &[&AgentStatus]) {
        // Remember the currently-selected session_id before rebuilding
        let current_id = self.current().map(|s| s.to_string());

        // Rebuild attention_ids: needs_attention && not dismissed, sorted
        let mut new_ids: Vec<String> = agents
            .iter()
            .filter(|a| a.needs_attention && !self.dismissed.contains(&a.session_id))
            .map(|a| a.session_id.clone())
            .collect();
        new_ids.sort();

        // Preserve cursor if the current session is still in the new list
        let new_cursor = if let Some(ref id) = current_id {
            new_ids.iter().position(|s| s == id)
        } else {
            None
        };

        self.attention_ids = new_ids;
        self.cursor = new_cursor;
    }

    /// Advance cursor to the next session needing attention, wrapping around.
    ///
    /// Returns the session_id or None if no sessions need attention.
    pub fn next_attention(&mut self) -> Option<&str> {
        if self.attention_ids.is_empty() {
            self.cursor = None;
            return None;
        }

        let next = match self.cursor {
            Some(idx) => (idx + 1) % self.attention_ids.len(),
            None => 0,
        };
        self.cursor = Some(next);
        Some(&self.attention_ids[next])
    }

    /// Return the currently-selected session_id without advancing.
    pub fn current(&self) -> Option<&str> {
        self.cursor
            .and_then(|idx| self.attention_ids.get(idx))
            .map(|s| s.as_str())
    }

    /// Add session_id to the dismissed set. Remove it from attention_ids.
    /// Adjust cursor if needed.
    pub fn dismiss(&mut self, session_id: &str) {
        self.dismissed.insert(session_id.to_string());

        if let Some(pos) = self.attention_ids.iter().position(|s| s == session_id) {
            self.attention_ids.remove(pos);

            // Adjust cursor
            if self.attention_ids.is_empty() {
                self.cursor = None;
            } else if let Some(cursor) = self.cursor {
                if pos < cursor {
                    // Removed element was before cursor — shift cursor back
                    self.cursor = Some(cursor - 1);
                } else if pos == cursor {
                    // Removed the selected element — cursor stays but may need wrapping
                    if cursor >= self.attention_ids.len() {
                        self.cursor = Some(0);
                    }
                }
                // pos > cursor — cursor is unaffected
            }
        }
    }

    /// Clear all dismissals. Previously-dismissed sessions will reappear
    /// on the next `update()` call if they still need attention.
    pub fn clear_dismissed(&mut self) {
        self.dismissed.clear();
    }

    /// Number of sessions currently needing attention (excluding dismissed).
    pub fn attention_count(&self) -> usize {
        self.attention_ids.len()
    }

    /// Check if a session has been dismissed.
    pub fn is_dismissed(&self, session_id: &str) -> bool {
        self.dismissed.contains(session_id)
    }
}

impl Default for AttentionTracker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::status::{AgentKind, AgentStatus, AgentStatusValue};

    /// Helper to build an `AgentStatus` with the given session_id and needs_attention.
    fn make_agent(session_id: &str, needs_attention: bool) -> AgentStatus {
        AgentStatus {
            version: 1,
            session_id: session_id.to_string(),
            agent: AgentKind::Claude,
            status: if needs_attention {
                AgentStatusValue::Waiting
            } else {
                AgentStatusValue::Idle
            },
            git_branch: None,
            git_dirty: false,
            working_dir: "/tmp".to_string(),
            last_message: None,
            ports: vec![],
            pr_number: None,
            pr_ci_status: None,
            needs_attention,
            updated_at: 1000,
        }
    }

    #[test]
    fn test_empty_tracker() {
        let mut tracker = AttentionTracker::new();
        assert_eq!(tracker.attention_count(), 0);
        assert!(tracker.current().is_none());
        assert!(tracker.next_attention().is_none());
    }

    #[test]
    fn test_update_adds_attention() {
        let mut tracker = AttentionTracker::new();
        let a1 = make_agent("agent-1", true);
        let a2 = make_agent("agent-2", true);
        tracker.update(&[&a1, &a2]);
        assert_eq!(tracker.attention_count(), 2);
    }

    #[test]
    fn test_update_excludes_non_attention() {
        let mut tracker = AttentionTracker::new();
        let a1 = make_agent("agent-1", true);
        let a2 = make_agent("agent-2", false);
        let a3 = make_agent("agent-3", true);
        tracker.update(&[&a1, &a2, &a3]);
        assert_eq!(tracker.attention_count(), 2);
        // agent-2 should not be in the list
        assert!(tracker.next_attention() != Some("agent-2"));
    }

    #[test]
    fn test_next_cycles() {
        let mut tracker = AttentionTracker::new();
        let a1 = make_agent("agent-a", true);
        let a2 = make_agent("agent-b", true);
        let a3 = make_agent("agent-c", true);
        tracker.update(&[&a1, &a2, &a3]);

        // Sorted order: agent-a, agent-b, agent-c
        assert_eq!(tracker.next_attention(), Some("agent-a"));
        assert_eq!(tracker.next_attention(), Some("agent-b"));
        assert_eq!(tracker.next_attention(), Some("agent-c"));
    }

    #[test]
    fn test_next_wraps_around() {
        let mut tracker = AttentionTracker::new();
        let a1 = make_agent("agent-a", true);
        let a2 = make_agent("agent-b", true);
        tracker.update(&[&a1, &a2]);

        // Sorted order: agent-a, agent-b
        assert_eq!(tracker.next_attention(), Some("agent-a"));
        assert_eq!(tracker.next_attention(), Some("agent-b"));
        // Should wrap back to first
        assert_eq!(tracker.next_attention(), Some("agent-a"));
    }

    #[test]
    fn test_dismiss_removes_session() {
        let mut tracker = AttentionTracker::new();
        let a1 = make_agent("agent-a", true);
        let a2 = make_agent("agent-b", true);
        let a3 = make_agent("agent-c", true);
        tracker.update(&[&a1, &a2, &a3]);

        assert_eq!(tracker.attention_count(), 3);
        tracker.dismiss("agent-b");
        assert_eq!(tracker.attention_count(), 2);
        assert!(tracker.is_dismissed("agent-b"));

        // Cycling should skip agent-b
        let mut seen = Vec::new();
        for _ in 0..3 {
            if let Some(id) = tracker.next_attention() {
                seen.push(id.to_string());
            }
        }
        assert!(!seen.contains(&"agent-b".to_string()));
    }

    #[test]
    fn test_dismiss_adjusts_cursor() {
        let mut tracker = AttentionTracker::new();
        let a1 = make_agent("agent-a", true);
        let a2 = make_agent("agent-b", true);
        let a3 = make_agent("agent-c", true);
        tracker.update(&[&a1, &a2, &a3]);

        // Navigate to agent-c (index 2)
        tracker.next_attention(); // agent-a (0)
        tracker.next_attention(); // agent-b (1)
        tracker.next_attention(); // agent-c (2)
        assert_eq!(tracker.current(), Some("agent-c"));

        // Dismiss agent-a (index 0, before cursor at 2)
        tracker.dismiss("agent-a");
        // Cursor should adjust from 2 to 1 (still pointing to agent-c)
        assert_eq!(tracker.current(), Some("agent-c"));
        assert_eq!(tracker.attention_count(), 2);
    }

    #[test]
    fn test_clear_dismissed() {
        let mut tracker = AttentionTracker::new();
        let a1 = make_agent("agent-a", true);
        let a2 = make_agent("agent-b", true);
        tracker.update(&[&a1, &a2]);

        tracker.dismiss("agent-b");
        assert_eq!(tracker.attention_count(), 1);
        assert!(tracker.is_dismissed("agent-b"));

        tracker.clear_dismissed();
        assert!(!tracker.is_dismissed("agent-b"));

        // After clear, next update should bring agent-b back
        tracker.update(&[&a1, &a2]);
        assert_eq!(tracker.attention_count(), 2);
    }

    #[test]
    fn test_update_preserves_cursor() {
        let mut tracker = AttentionTracker::new();
        let a1 = make_agent("agent-a", true);
        let a2 = make_agent("agent-b", true);
        let a3 = make_agent("agent-c", true);
        tracker.update(&[&a1, &a2, &a3]);

        // Navigate to agent-b
        tracker.next_attention(); // agent-a
        tracker.next_attention(); // agent-b
        assert_eq!(tracker.current(), Some("agent-b"));

        // Update again with same agents — cursor should stay on agent-b
        tracker.update(&[&a1, &a2, &a3]);
        assert_eq!(tracker.current(), Some("agent-b"));
    }

    #[test]
    fn test_dismiss_current_at_end_wraps_cursor() {
        let mut tracker = AttentionTracker::new();
        let a1 = make_agent("agent-a", true);
        let a2 = make_agent("agent-b", true);
        tracker.update(&[&a1, &a2]);

        // Navigate to agent-b (index 1, last element)
        tracker.next_attention(); // agent-a
        tracker.next_attention(); // agent-b
        assert_eq!(tracker.current(), Some("agent-b"));

        // Dismiss agent-b — cursor should wrap to 0
        tracker.dismiss("agent-b");
        assert_eq!(tracker.current(), Some("agent-a"));
    }

    #[test]
    fn test_dismiss_all_clears_cursor() {
        let mut tracker = AttentionTracker::new();
        let a1 = make_agent("agent-a", true);
        tracker.update(&[&a1]);

        tracker.next_attention(); // agent-a
        assert_eq!(tracker.current(), Some("agent-a"));

        tracker.dismiss("agent-a");
        assert_eq!(tracker.attention_count(), 0);
        assert!(tracker.current().is_none());
        assert!(tracker.next_attention().is_none());
    }

    #[test]
    fn test_update_removes_no_longer_needing_attention() {
        let mut tracker = AttentionTracker::new();
        let a1 = make_agent("agent-a", true);
        let a2 = make_agent("agent-b", true);
        tracker.update(&[&a1, &a2]);
        assert_eq!(tracker.attention_count(), 2);

        // agent-b no longer needs attention
        let a2_updated = make_agent("agent-b", false);
        tracker.update(&[&a1, &a2_updated]);
        assert_eq!(tracker.attention_count(), 1);
    }

    #[test]
    fn test_dismissed_sessions_stay_excluded_on_update() {
        let mut tracker = AttentionTracker::new();
        let a1 = make_agent("agent-a", true);
        let a2 = make_agent("agent-b", true);
        tracker.update(&[&a1, &a2]);

        tracker.dismiss("agent-b");
        assert_eq!(tracker.attention_count(), 1);

        // Re-update with agent-b still needing attention — should stay dismissed
        tracker.update(&[&a1, &a2]);
        assert_eq!(tracker.attention_count(), 1);
        assert!(tracker.is_dismissed("agent-b"));
    }
}
