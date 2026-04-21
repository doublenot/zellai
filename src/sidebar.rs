//! Pure-logic sidebar rendering module.
//!
//! Produces `Vec<String>` output for the Zellij plugin's `render()` method.
//! No `zellij_tile` imports — all functions are testable against the host target.

use crate::config::{CardDensity, SidebarConfig};
use crate::status::{AgentStatus, AgentStatusValue};

// ---------------------------------------------------------------------------
// ResolvedDensity — the outcome of adaptive density selection
// ---------------------------------------------------------------------------

/// The resolved card density after evaluating available space.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResolvedDensity {
    /// Every agent gets a single-line compact card.
    AllCompact,
    /// Every agent gets a 3-line detailed card.
    AllDetailed,
    /// Agents needing attention get detailed cards; the rest get compact.
    Mixed,
}

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Number of rows consumed by a compact card.
const COMPACT_ROWS: usize = 1;
/// Number of rows consumed by a detailed card.
const DETAILED_ROWS: usize = 3;
/// Number of rows reserved for chrome (top border + bottom border).
const CHROME_ROWS: usize = 2;

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Render the full sidebar as a `Vec<String>`, one entry per row.
///
/// Each string is exactly `cols` characters wide (padded or truncated).
/// Layout: top border, title bar, agent cards, fill space, bottom border.
pub fn render_sidebar(
    agents: &[&AgentStatus],
    config: &SidebarConfig,
    rows: usize,
    cols: usize,
) -> Vec<String> {
    if rows == 0 || cols == 0 {
        return Vec::new();
    }

    let mut lines: Vec<String> = Vec::with_capacity(rows);

    // Top border: ╭─ zellai ─...─╮
    lines.push(render_top_border(cols));

    // Title row (included in top border for us, so skip if rows < 2)
    if rows < 2 {
        return lines;
    }

    // Available rows for agent cards + fill
    let available = rows.saturating_sub(CHROME_ROWS);

    if agents.is_empty() {
        // Empty state
        let msg = "No agents connected";
        let inner = cols.saturating_sub(2);
        if available > 0 {
            lines.push(render_box_line(&center_text(msg, inner), cols));
        }
        // Fill remaining
        for _ in 1..available {
            lines.push(render_empty_line(cols));
        }
    } else {
        let attention_count = agents.iter().filter(|a| a.needs_attention).count();
        let density = choose_density(
            agents.len(),
            attention_count,
            available,
            &config.card_density,
        );

        let mut used_rows = 0;
        for agent in agents {
            if used_rows >= available {
                break;
            }
            let is_detailed = match density {
                ResolvedDensity::AllDetailed => true,
                ResolvedDensity::AllCompact => false,
                ResolvedDensity::Mixed => agent.needs_attention,
            };

            if is_detailed {
                let card_lines = render_detailed_card(agent, cols);
                for line in card_lines {
                    if used_rows >= available {
                        break;
                    }
                    lines.push(line);
                    used_rows += 1;
                }
            } else {
                let line = render_compact_card(agent, cols);
                lines.push(line);
                used_rows += 1;
            }
        }
        // Fill remaining
        for _ in used_rows..available {
            lines.push(render_empty_line(cols));
        }
    }

    // Bottom border: ╰─...─╯
    if rows > 1 {
        lines.push(render_bottom_border(cols));
    }

    // Ensure exactly `rows` lines (truncate if somehow over)
    lines.truncate(rows);

    lines
}

// ---------------------------------------------------------------------------
// Card rendering
// ---------------------------------------------------------------------------

/// Render a single-line compact card for an agent.
///
/// Format: `│ ● name [branch] status │`
/// The returned string is exactly `width` characters.
pub fn render_compact_card(agent: &AgentStatus, width: usize) -> String {
    let icon = status_icon(&agent.status);
    let name = agent_display_name(agent);
    let status_str = agent.status.to_string().to_lowercase();

    let branch_part = match &agent.git_branch {
        Some(b) => format!(" [{}]", b),
        None => String::new(),
    };

    let content = format!(" {} {}{} {} ", icon, name, branch_part, status_str);
    render_box_line(&content, width)
}

/// Render a 3-line detailed card for an agent.
///
/// Line 1: `│ ● name — status │`
/// Line 2: `│   branch ● /working/dir │`
/// Line 3: `│   last_message… │`
///
/// Each line is exactly `width` characters.
pub fn render_detailed_card(agent: &AgentStatus, width: usize) -> Vec<String> {
    let inner_width = width.saturating_sub(4); // "│ " + " │"
    let icon = status_icon(&agent.status);
    let name = agent_display_name(agent);
    let status_str = agent.status.to_string().to_lowercase();

    // Line 1: icon + name — status
    let line1_content = format!(" {} {} — {} ", icon, name, status_str);
    let line1 = render_box_line(&line1_content, width);

    // Line 2: branch + working dir
    let branch = agent.git_branch.as_deref().unwrap_or("(no branch)");
    let dir_raw = &agent.working_dir;
    let prefix = format!("   {} ● ", branch);
    let prefix_chars = prefix.chars().count();
    let remaining = inner_width.saturating_sub(prefix_chars);
    let dir_display = truncate_with_ellipsis(dir_raw, remaining);
    let line2_content = format!(" {}{} ", prefix, dir_display);
    let line2 = render_box_line(&line2_content, width);

    // Line 3: last message
    let msg = agent.last_message.as_deref().unwrap_or("(no message)");
    let msg_display = truncate_with_ellipsis(msg, inner_width.saturating_sub(3));
    let line3_content = format!("   {} ", msg_display);
    let line3 = render_box_line(&line3_content, width);

    vec![line1, line2, line3]
}

// ---------------------------------------------------------------------------
// Density selection
// ---------------------------------------------------------------------------

/// Choose the resolved density based on agent count, attention count,
/// available rows, and the user's configured preference.
pub fn choose_density(
    agent_count: usize,
    attention_count: usize,
    available_rows: usize,
    config_density: &CardDensity,
) -> ResolvedDensity {
    match config_density {
        CardDensity::Compact => ResolvedDensity::AllCompact,
        CardDensity::Detailed => {
            if agent_count * DETAILED_ROWS <= available_rows {
                ResolvedDensity::AllDetailed
            } else {
                // Fall back to compact if there's not enough space even when forced
                ResolvedDensity::AllCompact
            }
        }
        CardDensity::Adaptive => {
            let detailed_total = agent_count * DETAILED_ROWS;
            let compact_total = agent_count * COMPACT_ROWS;

            if detailed_total <= available_rows {
                // Everything fits in detailed mode
                ResolvedDensity::AllDetailed
            } else if compact_total > available_rows {
                // Even compact won't fit everything — still use compact (truncation happens in render)
                ResolvedDensity::AllCompact
            } else if attention_count == 0 {
                // No attention agents, just use compact
                ResolvedDensity::AllCompact
            } else {
                // Try mixed: attention agents get detailed, rest get compact
                let mixed_total = attention_count * DETAILED_ROWS
                    + (agent_count - attention_count) * COMPACT_ROWS;
                if mixed_total <= available_rows {
                    ResolvedDensity::Mixed
                } else {
                    ResolvedDensity::AllCompact
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Helper functions
// ---------------------------------------------------------------------------

/// Map an agent status to a Unicode icon.
pub fn status_icon(status: &AgentStatusValue) -> &'static str {
    match status {
        AgentStatusValue::Thinking => "◉",
        AgentStatusValue::Waiting => "⚠",
        AgentStatusValue::Idle => "○",
        AgentStatusValue::Error => "✗",
    }
}

/// Truncate a string to `max_len` characters, appending "…" if truncated.
///
/// If `max_len` is 0, returns an empty string.
/// If the string fits within `max_len`, it is returned unchanged.
pub fn truncate_with_ellipsis(s: &str, max_len: usize) -> String {
    if max_len == 0 {
        return String::new();
    }
    let chars: Vec<char> = s.chars().collect();
    if chars.len() <= max_len {
        s.to_string()
    } else if max_len == 1 {
        "…".to_string()
    } else {
        let truncated: String = chars[..max_len - 1].iter().collect();
        format!("{}…", truncated)
    }
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Short display name for an agent (lowercase agent kind).
fn agent_display_name(agent: &AgentStatus) -> String {
    agent.agent.to_string().to_lowercase()
}

/// Render a line inside box-drawing borders: `│<content padded to width>│`
///
/// `content` is the inner text (without borders). The result is exactly `width` chars.
fn render_box_line(content: &str, width: usize) -> String {
    let inner_width = width.saturating_sub(2); // "│" on each side
    let char_count: usize = content.chars().count();
    if char_count >= inner_width {
        // Truncate content to fit
        let truncated: String = content.chars().take(inner_width).collect();
        format!("│{}│", truncated)
    } else {
        let padding = inner_width - char_count;
        // Use explicit space repetition — format! width specifier counts bytes, not chars
        format!("│{}{}│", content, " ".repeat(padding))
    }
}

/// Render an empty line inside box borders.
fn render_empty_line(width: usize) -> String {
    let inner = width.saturating_sub(2);
    format!("│{}│", " ".repeat(inner))
}

/// Render the top border with title: `╭─ zellai ─...─╮`
fn render_top_border(width: usize) -> String {
    let inner = width.saturating_sub(2); // "╭" + "╮"
    if inner < 10 {
        // Too narrow for title — just draw a plain border
        format!("╭{}╮", "─".repeat(inner))
    } else {
        let title = "─ zellai ─";
        let title_chars = title.chars().count();
        let remaining = inner.saturating_sub(title_chars);
        format!("╭{}{}╮", title, "─".repeat(remaining))
    }
}

/// Render the bottom border: `╰─...─╯`
fn render_bottom_border(width: usize) -> String {
    let inner = width.saturating_sub(2);
    format!("╰{}╯", "─".repeat(inner))
}

/// Center text within a given width, padding with spaces.
fn center_text(text: &str, width: usize) -> String {
    let text_len = text.chars().count();
    if text_len >= width {
        text.chars().take(width).collect()
    } else {
        let left_pad = (width - text_len) / 2;
        let right_pad = width - text_len - left_pad;
        format!("{}{}{}", " ".repeat(left_pad), text, " ".repeat(right_pad),)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::status::{AgentKind, AgentStatusValue};

    /// Helper to create an AgentStatus for testing.
    fn make_agent(
        session_id: &str,
        agent: AgentKind,
        status: AgentStatusValue,
        branch: Option<&str>,
        working_dir: &str,
        last_message: Option<&str>,
        needs_attention: bool,
    ) -> AgentStatus {
        AgentStatus {
            version: 1,
            session_id: session_id.to_string(),
            agent,
            status,
            git_branch: branch.map(|s| s.to_string()),
            git_dirty: false,
            working_dir: working_dir.to_string(),
            last_message: last_message.map(|s| s.to_string()),
            ports: vec![],
            pr_number: None,
            pr_ci_status: None,
            needs_attention,
            updated_at: 1000,
        }
    }

    fn default_sidebar_config() -> SidebarConfig {
        SidebarConfig::default()
    }

    // -----------------------------------------------------------------------
    // status_icon
    // -----------------------------------------------------------------------

    #[test]
    fn test_status_icon_thinking() {
        assert_eq!(status_icon(&AgentStatusValue::Thinking), "◉");
    }

    #[test]
    fn test_status_icon_waiting() {
        assert_eq!(status_icon(&AgentStatusValue::Waiting), "⚠");
    }

    #[test]
    fn test_status_icon_idle() {
        assert_eq!(status_icon(&AgentStatusValue::Idle), "○");
    }

    #[test]
    fn test_status_icon_error() {
        assert_eq!(status_icon(&AgentStatusValue::Error), "✗");
    }

    // -----------------------------------------------------------------------
    // truncate_with_ellipsis
    // -----------------------------------------------------------------------

    #[test]
    fn test_truncate_short_string_unchanged() {
        assert_eq!(truncate_with_ellipsis("hello", 10), "hello");
    }

    #[test]
    fn test_truncate_exact_length_unchanged() {
        assert_eq!(truncate_with_ellipsis("hello", 5), "hello");
    }

    #[test]
    fn test_truncate_long_string() {
        assert_eq!(truncate_with_ellipsis("hello world", 8), "hello w…");
    }

    #[test]
    fn test_truncate_max_len_1() {
        assert_eq!(truncate_with_ellipsis("hello", 1), "…");
    }

    #[test]
    fn test_truncate_max_len_0() {
        assert_eq!(truncate_with_ellipsis("hello", 0), "");
    }

    #[test]
    fn test_truncate_empty_string() {
        assert_eq!(truncate_with_ellipsis("", 5), "");
    }

    // -----------------------------------------------------------------------
    // render_compact_card
    // -----------------------------------------------------------------------

    #[test]
    fn test_render_compact_card_with_branch() {
        let agent = make_agent(
            "sess-1",
            AgentKind::Claude,
            AgentStatusValue::Thinking,
            Some("feat/auth"),
            "/home/user/app",
            Some("Reading file"),
            false,
        );
        let line = render_compact_card(&agent, 40);
        assert!(line.starts_with('│'));
        assert!(line.ends_with('│'));
        assert_eq!(line.chars().count(), 40);
        assert!(line.contains("◉"));
        assert!(line.contains("claude"));
        assert!(line.contains("[feat/auth]"));
        assert!(line.contains("thinking"));
    }

    #[test]
    fn test_render_compact_card_no_branch() {
        let agent = make_agent(
            "sess-2",
            AgentKind::Codex,
            AgentStatusValue::Idle,
            None,
            "/tmp",
            None,
            false,
        );
        let line = render_compact_card(&agent, 30);
        assert_eq!(line.chars().count(), 30);
        assert!(line.contains("○"));
        assert!(line.contains("codex"));
        assert!(line.contains("idle"));
        // No branch brackets
        assert!(!line.contains('['));
    }

    // -----------------------------------------------------------------------
    // render_detailed_card
    // -----------------------------------------------------------------------

    #[test]
    fn test_render_detailed_card_returns_3_lines() {
        let agent = make_agent(
            "sess-1",
            AgentKind::Claude,
            AgentStatusValue::Thinking,
            Some("feat/auth"),
            "/home/user/app",
            Some("Reading src/auth.ts…"),
            false,
        );
        let lines = render_detailed_card(&agent, 50);
        assert_eq!(lines.len(), 3);
        for line in &lines {
            assert_eq!(line.chars().count(), 50);
            assert!(line.starts_with('│'));
            assert!(line.ends_with('│'));
        }
        // Line 1: icon + name — status
        assert!(lines[0].contains("◉"));
        assert!(lines[0].contains("claude"));
        assert!(lines[0].contains("thinking"));
        // Line 2: branch + dir
        assert!(lines[1].contains("feat/auth"));
        // Line 3: last message
        assert!(lines[2].contains("Reading"));
    }

    #[test]
    fn test_render_detailed_card_no_message() {
        let agent = make_agent(
            "sess-2",
            AgentKind::Gemini,
            AgentStatusValue::Error,
            None,
            "/tmp",
            None,
            false,
        );
        let lines = render_detailed_card(&agent, 40);
        assert_eq!(lines.len(), 3);
        assert!(lines[2].contains("(no message)"));
    }

    // -----------------------------------------------------------------------
    // choose_density
    // -----------------------------------------------------------------------

    #[test]
    fn test_choose_density_all_detailed_when_space() {
        // 3 agents, 3*3=9 rows needed, 10 available
        let result = choose_density(3, 0, 10, &CardDensity::Adaptive);
        assert_eq!(result, ResolvedDensity::AllDetailed);
    }

    #[test]
    fn test_choose_density_all_compact_when_tight() {
        // 5 agents, 5*3=15 rows needed (too many), 5*1=5 needed, 6 available
        let result = choose_density(5, 0, 6, &CardDensity::Adaptive);
        assert_eq!(result, ResolvedDensity::AllCompact);
    }

    #[test]
    fn test_choose_density_mixed_when_attention() {
        // 4 agents, 1 needs attention
        // Mixed: 1*3 + 3*1 = 6 rows, available=8 → fits
        // AllDetailed: 4*3 = 12 > 8 → doesn't fit
        let result = choose_density(4, 1, 8, &CardDensity::Adaptive);
        assert_eq!(result, ResolvedDensity::Mixed);
    }

    #[test]
    fn test_choose_density_compact_override() {
        // User forces compact
        let result = choose_density(3, 1, 100, &CardDensity::Compact);
        assert_eq!(result, ResolvedDensity::AllCompact);
    }

    #[test]
    fn test_choose_density_detailed_override_fits() {
        // User forces detailed and it fits
        let result = choose_density(3, 0, 10, &CardDensity::Detailed);
        assert_eq!(result, ResolvedDensity::AllDetailed);
    }

    #[test]
    fn test_choose_density_detailed_override_no_fit() {
        // User forces detailed but it doesn't fit → falls back to compact
        let result = choose_density(5, 0, 6, &CardDensity::Detailed);
        assert_eq!(result, ResolvedDensity::AllCompact);
    }

    #[test]
    fn test_choose_density_mixed_no_fit_falls_to_compact() {
        // Mixed would need 1*3 + 3*1 = 6, but only 4 available
        let result = choose_density(4, 1, 4, &CardDensity::Adaptive);
        assert_eq!(result, ResolvedDensity::AllCompact);
    }

    // -----------------------------------------------------------------------
    // render_sidebar — empty state
    // -----------------------------------------------------------------------

    #[test]
    fn test_render_sidebar_empty_agents() {
        let config = default_sidebar_config();
        let agents: Vec<&AgentStatus> = vec![];
        let lines = render_sidebar(&agents, &config, 10, 40);
        assert_eq!(lines.len(), 10);
        // First line: top border
        assert!(lines[0].starts_with('╭'));
        assert!(lines[0].ends_with('╮'));
        // Last line: bottom border
        assert!(lines[9].starts_with('╰'));
        assert!(lines[9].ends_with('╯'));
        // Check for "No agents connected" message somewhere
        let all = lines.join("\n");
        assert!(all.contains("No agents connected"));
    }

    // -----------------------------------------------------------------------
    // render_sidebar — with agents
    // -----------------------------------------------------------------------

    #[test]
    fn test_render_sidebar_correct_row_count() {
        let a1 = make_agent(
            "s1",
            AgentKind::Claude,
            AgentStatusValue::Thinking,
            Some("main"),
            "/tmp",
            Some("Working..."),
            false,
        );
        let a2 = make_agent(
            "s2",
            AgentKind::Codex,
            AgentStatusValue::Idle,
            None,
            "/tmp",
            None,
            false,
        );
        let agents: Vec<&AgentStatus> = vec![&a1, &a2];
        let config = default_sidebar_config();
        let lines = render_sidebar(&agents, &config, 20, 40);
        assert_eq!(lines.len(), 20);
        // All lines should be exactly 40 chars
        for (i, line) in lines.iter().enumerate() {
            assert_eq!(
                line.chars().count(),
                40,
                "line {} has wrong width: '{}'",
                i,
                line
            );
        }
    }

    #[test]
    fn test_render_sidebar_zero_rows() {
        let config = default_sidebar_config();
        let agents: Vec<&AgentStatus> = vec![];
        let lines = render_sidebar(&agents, &config, 0, 40);
        assert!(lines.is_empty());
    }

    #[test]
    fn test_render_sidebar_zero_cols() {
        let config = default_sidebar_config();
        let agents: Vec<&AgentStatus> = vec![];
        let lines = render_sidebar(&agents, &config, 10, 0);
        assert!(lines.is_empty());
    }

    #[test]
    fn test_render_sidebar_with_attention_agent() {
        let a1 = make_agent(
            "s1",
            AgentKind::Claude,
            AgentStatusValue::Waiting,
            Some("main"),
            "/home/user/app",
            Some("Need input"),
            true,
        );
        let a2 = make_agent(
            "s2",
            AgentKind::Codex,
            AgentStatusValue::Thinking,
            Some("dev"),
            "/tmp",
            Some("Working"),
            false,
        );
        let a3 = make_agent(
            "s3",
            AgentKind::Gemini,
            AgentStatusValue::Idle,
            None,
            "/tmp",
            None,
            false,
        );
        let agents: Vec<&AgentStatus> = vec![&a1, &a2, &a3];
        // Give enough space for mixed (1*3 + 2*1 = 5 agent rows + 3 chrome = 8)
        let mut config = default_sidebar_config();
        config.card_density = CardDensity::Adaptive;
        let lines = render_sidebar(&agents, &config, 10, 50);
        assert_eq!(lines.len(), 10);
        // The waiting agent should appear somewhere (it needs attention)
        let all = lines.join("\n");
        assert!(all.contains("⚠")); // waiting icon for attention agent
    }

    // -----------------------------------------------------------------------
    // Internal helpers
    // -----------------------------------------------------------------------

    #[test]
    fn test_center_text() {
        let result = center_text("hi", 10);
        assert_eq!(result.chars().count(), 10);
        assert!(result.contains("hi"));
    }

    #[test]
    fn test_render_top_border_width() {
        let border = render_top_border(30);
        assert_eq!(border.chars().count(), 30);
        assert!(border.starts_with('╭'));
        assert!(border.ends_with('╮'));
        assert!(border.contains("zellai"));
    }

    #[test]
    fn test_render_bottom_border_width() {
        let border = render_bottom_border(30);
        assert_eq!(border.chars().count(), 30);
        assert!(border.starts_with('╰'));
        assert!(border.ends_with('╯'));
    }

    #[test]
    fn test_render_empty_line_width() {
        let line = render_empty_line(20);
        assert_eq!(line.chars().count(), 20);
        assert!(line.starts_with('│'));
        assert!(line.ends_with('│'));
    }
}
