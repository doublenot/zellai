//! Pure-logic status bar rendering module.
//!
//! Produces a single `String` for the Zellij status bar plugin's `render()`.
//! No `zellij_tile` imports — fully unit-testable against the host target.

use crate::status::AgentStatus;

/// ANSI reset — clears all attributes.
const RESET: &str = "\x1b[0m";
/// ANSI bold.
const BOLD: &str = "\x1b[1m";
/// ANSI yellow (for attention).
const YELLOW: &str = "\x1b[33m";

/// Render the status bar segment as a single line.
///
/// Format: `⬡ workspace | N agents | M⚠`
/// - If no agents: `⬡ workspace`
/// - The `M⚠` section only appears when at least one agent needs attention.
/// - The `⬡` icon is bold, the `M⚠` section is yellow.
/// - Output is truncated to `cols` visible characters.
pub fn render_status_bar(agents: &[&AgentStatus], workspace_name: &str, cols: usize) -> String {
    if cols == 0 {
        return String::new();
    }

    let attention_count = agents.iter().filter(|a| a.needs_attention).count();
    let agent_count = agents.len();

    let agent_word = if agent_count == 1 { "agent" } else { "agents" };

    let full = if agent_count == 0 {
        format!(" {}⬡{} {} ", BOLD, RESET, workspace_name)
    } else if attention_count > 0 {
        format!(
            " {}⬡{} {} | {} {} | {}{}⚠{} ",
            BOLD, RESET, workspace_name, agent_count, agent_word, YELLOW, attention_count, RESET
        )
    } else {
        format!(
            " {}⬡{} {} | {} {} ",
            BOLD, RESET, workspace_name, agent_count, agent_word
        )
    };

    truncate_to_cols(&full, cols)
}

/// Truncate a string to fit within `cols` visible display columns.
///
/// ANSI escape sequences are zero-width and do not count toward `cols`.
/// If the visible string is longer than `cols`, it is truncated and the last
/// visible character is replaced with `…`.
fn truncate_to_cols(s: &str, cols: usize) -> String {
    if cols == 0 {
        return String::new();
    }

    let vis_len = visible_char_count(s);
    if vis_len <= cols {
        return s.to_string();
    }
    if cols == 1 {
        return "…".to_string();
    }

    // Walk through keeping ANSI escapes but only up to cols-1 visible chars
    let max_visible = cols - 1;
    let mut result = String::with_capacity(s.len());
    let mut visible = 0;
    let mut in_escape = false;
    let mut has_ansi = false;
    for ch in s.chars() {
        if in_escape {
            result.push(ch);
            if ch == 'm' {
                in_escape = false;
            }
        } else if ch == '\x1b' {
            in_escape = true;
            has_ansi = true;
            result.push(ch);
        } else {
            if visible >= max_visible {
                break;
            }
            result.push(ch);
            visible += 1;
        }
    }
    result.push('…');
    if has_ansi {
        result.push_str(RESET);
    }
    result
}

/// Count visible characters in a string, ignoring ANSI escape sequences.
fn visible_char_count(s: &str) -> usize {
    let mut count = 0;
    let mut in_escape = false;
    for ch in s.chars() {
        if in_escape {
            if ch == 'm' {
                in_escape = false;
            }
        } else if ch == '\x1b' {
            in_escape = true;
        } else {
            count += 1;
        }
    }
    count
}

/// Strip ANSI escape sequences from a string.
#[cfg(test)]
#[allow(dead_code)]
fn strip_ansi(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut in_escape = false;
    for ch in s.chars() {
        if in_escape {
            if ch == 'm' {
                in_escape = false;
            }
        } else if ch == '\x1b' {
            in_escape = true;
        } else {
            result.push(ch);
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::status::{AgentKind, AgentStatus, AgentStatusValue};

    fn make_agent(session_id: &str, status: AgentStatusValue) -> AgentStatus {
        let mut agent = AgentStatus {
            version: 1,
            session_id: session_id.to_string(),
            agent: AgentKind::Claude,
            status: status.clone(),
            git_branch: None,
            git_dirty: false,
            working_dir: "/tmp".to_string(),
            last_message: None,
            ports: vec![],
            pr_number: None,
            pr_ci_status: None,
            needs_attention: false,
            updated_at: 1000,
            pane_id: None,
        };
        agent.validate();
        agent
    }

    #[test]
    fn test_empty_agents_shows_workspace_name() {
        let result = render_status_bar(&[], "myproject", 80);
        assert!(result.contains("myproject"));
        assert!(result.contains("⬡"));
        // Should NOT contain "agents" or warning
        assert!(!result.contains("agents"));
        assert!(!result.contains("⚠"));
    }

    #[test]
    fn test_multiple_agents_no_attention() {
        let a1 = make_agent("s1", AgentStatusValue::Thinking);
        let a2 = make_agent("s2", AgentStatusValue::Idle);
        let a3 = make_agent("s3", AgentStatusValue::Thinking);
        let agents: Vec<&AgentStatus> = vec![&a1, &a2, &a3];

        let result = render_status_bar(&agents, "ws", 80);
        assert!(result.contains("ws"));
        assert!(result.contains("3 agents"));
        assert!(!result.contains("⚠"));
    }

    #[test]
    fn test_agents_with_attention() {
        let a1 = make_agent("s1", AgentStatusValue::Thinking);
        let a2 = make_agent("s2", AgentStatusValue::Waiting); // needs_attention = true
        let a3 = make_agent("s3", AgentStatusValue::Waiting); // needs_attention = true
        let agents: Vec<&AgentStatus> = vec![&a1, &a2, &a3];

        let result = render_status_bar(&agents, "ws", 80);
        assert!(result.contains("3 agents"));
        assert!(result.contains("2⚠"));
    }

    #[test]
    fn test_single_agent_attention() {
        let a1 = make_agent("s1", AgentStatusValue::Waiting);
        let agents: Vec<&AgentStatus> = vec![&a1];

        let result = render_status_bar(&agents, "zellai", 80);
        assert!(result.contains("1 agent"));
        assert!(!result.contains("1 agents"));
        assert!(result.contains("1⚠"));
    }

    #[test]
    fn test_single_agent_no_attention() {
        let a1 = make_agent("s1", AgentStatusValue::Thinking);
        let agents: Vec<&AgentStatus> = vec![&a1];

        let result = render_status_bar(&agents, "zellai", 80);
        assert!(result.contains("1 agent"));
        assert!(!result.contains("1 agents"));
        assert!(!result.contains("⚠"));
    }

    #[test]
    fn test_very_narrow_cols_truncation() {
        let a1 = make_agent("s1", AgentStatusValue::Thinking);
        let a2 = make_agent("s2", AgentStatusValue::Thinking);
        let agents: Vec<&AgentStatus> = vec![&a1, &a2];

        let result = render_status_bar(&agents, "my-long-workspace-name", 10);
        // visible_char_count strips ANSI escapes; the visible output must fit in cols
        assert!(visible_char_count(&result) <= 10);
    }

    #[test]
    fn test_zero_cols() {
        let result = render_status_bar(&[], "ws", 0);
        assert_eq!(result, "");
    }

    #[test]
    fn test_one_col() {
        let result = render_status_bar(&[], "ws", 1);
        // With 1 col, should show just the ellipsis or a single char
        assert!(result.chars().count() <= 1);
    }

    #[test]
    fn test_workspace_name_included() {
        let result = render_status_bar(&[], "my-special-project", 80);
        assert!(result.contains("my-special-project"));
    }

    #[test]
    fn test_exact_fit_no_truncation() {
        // Construct a scenario where the string fits exactly
        let result = render_status_bar(&[], "z", 200);
        // Should not contain ellipsis
        assert!(!result.contains('…'));
    }

    #[test]
    fn test_truncate_to_cols_basic() {
        assert_eq!(truncate_to_cols("hello world", 5), "hell…");
        assert_eq!(truncate_to_cols("hi", 5), "hi");
        assert_eq!(truncate_to_cols("hello", 5), "hello");
        assert_eq!(truncate_to_cols("", 5), "");
        assert_eq!(truncate_to_cols("hello", 0), "");
        assert_eq!(truncate_to_cols("hello", 1), "…");
    }
}
