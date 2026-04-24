//! Pure-logic sidebar rendering module.
//!
//! Produces `Vec<String>` output for the Zellij plugin's `render()` method.
//! No `zellij_tile` imports — all functions are testable against the host target.

use crate::config::{CardDensity, SidebarConfig};
use crate::status::{AgentStatus, AgentStatusValue, CiStatus};

// ---------------------------------------------------------------------------
// ANSI color constants
// ---------------------------------------------------------------------------

/// ANSI reset — clears all attributes.
const RESET: &str = "\x1b[0m";
/// ANSI bold.
const BOLD: &str = "\x1b[1m";
/// ANSI dim (faint).
const DIM: &str = "\x1b[2m";
/// ANSI green (for thinking).
const GREEN: &str = "\x1b[32m";
/// ANSI yellow (for waiting / attention).
const YELLOW: &str = "\x1b[33m";
/// ANSI red (for error).
const RED: &str = "\x1b[31m";
/// ANSI cyan (for branch names).
const CYAN: &str = "\x1b[36m";

/// Map an `AgentStatusValue` to its ANSI color escape code.
pub fn status_color(status: &AgentStatusValue) -> &'static str {
    match status {
        AgentStatusValue::Thinking => GREEN,
        AgentStatusValue::Waiting => YELLOW,
        AgentStatusValue::Idle => DIM,
        AgentStatusValue::Error => RED,
    }
}

/// Count visible characters in a string, ignoring ANSI escape sequences.
///
/// ANSI escape sequences matching `\x1b\[[0-9;]*m` are treated as zero-width.
pub fn visible_char_count(s: &str) -> usize {
    let mut count = 0;
    let mut in_escape = false;
    for ch in s.chars() {
        if in_escape {
            if ch == 'm' {
                in_escape = false;
            }
            // skip all chars inside the escape sequence
        } else if ch == '\x1b' {
            in_escape = true;
        } else {
            count += 1;
        }
    }
    count
}

/// Strip ANSI escape sequences from a string, returning only visible text.
pub fn strip_ansi(s: &str) -> String {
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
const DETAILED_ROWS: usize = 4;
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
/// Format: `│ ●icon name [branch] status │`
/// The icon and status text are colored by agent status.
/// Branch name is colored in cyan.
/// The returned string has a visible width of exactly `width` characters.
pub fn render_compact_card(agent: &AgentStatus, width: usize) -> String {
    let icon = status_icon(&agent.status);
    let name = agent_display_name(agent);
    let status_str = agent.status.to_string().to_lowercase();
    let color = status_color(&agent.status);

    let branch_part = match &agent.git_branch {
        Some(b) => format!(" {}[{}]{}", CYAN, b, RESET),
        None => String::new(),
    };

    let content = format!(
        " {color}{icon}{reset} {name}{branch} {color}{status}{reset} ",
        color = color,
        icon = icon,
        reset = RESET,
        name = name,
        branch = branch_part,
        status = status_str,
    );
    render_box_line(&content, width)
}

/// Render a 4-line detailed card for an agent.
///
/// Line 1: `│ ● name — status │`  (icon + status colored)
/// Line 2: `│   branch ● /working/dir │`  (branch in cyan)
/// Line 3: `│   🔌 :3000 | PR #42 ✓ passing │`  (ports + PR/CI, or dim separator)
/// Line 4: `│   last_message… │`
///
/// Each line has a visible width of exactly `width` characters.
pub fn render_detailed_card(agent: &AgentStatus, width: usize) -> Vec<String> {
    let inner_width = width.saturating_sub(4); // "│ " + " │"
    let icon = status_icon(&agent.status);
    let name = agent_display_name(agent);
    let status_str = agent.status.to_string().to_lowercase();
    let color = status_color(&agent.status);

    // Line 1: icon + name — status (icon and status colored)
    let line1_content = format!(
        " {color}{icon}{reset} {name} — {color}{status}{reset} ",
        color = color,
        icon = icon,
        reset = RESET,
        name = name,
        status = status_str,
    );
    let line1 = render_box_line(&line1_content, width);

    // Line 2: branch + working dir (branch in cyan)
    let branch = agent.git_branch.as_deref().unwrap_or("(no branch)");
    let dir_raw = &agent.working_dir;
    // Calculate visible prefix length for truncation: "   branch ● "
    let prefix_plain = format!("   {} ● ", branch);
    let prefix_chars = prefix_plain.chars().count();
    let remaining = inner_width.saturating_sub(prefix_chars);
    let dir_display = truncate_with_ellipsis(dir_raw, remaining);
    let line2_content = format!(
        "   {cyan}{branch}{reset} ● {dir} ",
        cyan = CYAN,
        branch = branch,
        reset = RESET,
        dir = dir_display,
    );
    let line2 = render_box_line(&line2_content, width);

    // Line 3: ports + PR/CI metadata (new)
    let line3_content = render_metadata_line(agent, inner_width);
    let line3 = render_box_line(&line3_content, width);

    // Line 4: last message (default color)
    let msg = agent.last_message.as_deref().unwrap_or("(no message)");
    let msg_display = truncate_with_ellipsis(msg, inner_width.saturating_sub(3));
    let line4_content = format!("   {} ", msg_display);
    let line4 = render_box_line(&line4_content, width);

    vec![line1, line2, line3, line4]
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

/// Map a CI status to a Unicode icon.
pub fn ci_status_icon(status: &CiStatus) -> &'static str {
    match status {
        CiStatus::Passing => "✓",
        CiStatus::Failing => "✗",
        CiStatus::Pending => "⏳",
    }
}

/// Map a CI status to its descriptive label.
fn ci_status_label(status: &CiStatus) -> &'static str {
    match status {
        CiStatus::Passing => "passing",
        CiStatus::Failing => "failing",
        CiStatus::Pending => "pending",
    }
}

/// Render the metadata line (line 3) for a detailed card.
///
/// Formats ports and PR/CI info. If the agent has neither, returns a dim
/// separator line. The returned string is the inner content (without box borders).
pub fn render_metadata_line(agent: &AgentStatus, inner_width: usize) -> String {
    let mut parts: Vec<String> = Vec::new();

    // Ports section
    if !agent.ports.is_empty() {
        let port_strs: Vec<String> = agent.ports.iter().map(|p| format!(":{}", p)).collect();
        parts.push(format!("🔌 {}", port_strs.join(" ")));
    }

    // PR/CI section
    if let Some(pr_num) = agent.pr_number {
        let ci_part = match &agent.pr_ci_status {
            Some(ci) => {
                let icon = ci_status_icon(ci);
                let label = ci_status_label(ci);
                let color = match ci {
                    CiStatus::Passing => GREEN,
                    CiStatus::Failing => RED,
                    CiStatus::Pending => YELLOW,
                };
                format!(
                    "PR #{pr} {color}{icon} {label}{reset}",
                    pr = pr_num,
                    color = color,
                    icon = icon,
                    label = label,
                    reset = RESET,
                )
            }
            None => format!("PR #{}", pr_num),
        };
        parts.push(ci_part);
    }

    if parts.is_empty() {
        // No metadata — render a dim separator
        let sep_width = inner_width.saturating_sub(6); // "   " prefix + " " suffix + some margin
        format!(
            "   {dim}{sep}{reset} ",
            dim = DIM,
            sep = "─".repeat(sep_width),
            reset = RESET
        )
    } else {
        let joined = parts.join(" | ");
        let available = inner_width.saturating_sub(4); // "   " prefix + " " suffix padding
        let display = truncate_with_ellipsis(&strip_ansi(&joined), available);
        // Re-render with ANSI if not truncated
        if strip_ansi(&joined).len() <= available {
            format!("   {} ", joined)
        } else {
            format!("   {} ", display)
        }
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
/// `content` is the inner text (without borders). The visible width of the result
/// is exactly `width` characters. ANSI escape codes in `content` are treated as
/// zero-width for padding/truncation calculations.
fn render_box_line(content: &str, width: usize) -> String {
    let inner_width = width.saturating_sub(2); // "│" on each side
    let vis_count = visible_char_count(content);
    if vis_count >= inner_width {
        // Truncate visible chars to fit, preserving ANSI sequences
        let truncated = truncate_ansi_aware(content, inner_width);
        format!("{}│{}{}│{}", DIM, RESET, truncated, DIM)
    } else {
        let padding = inner_width - vis_count;
        format!(
            "{}│{}{}{}{}│{}",
            DIM,
            RESET,
            content,
            " ".repeat(padding),
            DIM,
            RESET
        )
    }
}

/// Truncate a string with ANSI escapes to `max_visible` visible characters.
///
/// Preserves ANSI escape sequences that appear before the cutoff point, and
/// appends a RESET at the end to avoid color bleed.
fn truncate_ansi_aware(s: &str, max_visible: usize) -> String {
    let mut result = String::with_capacity(s.len());
    let mut visible = 0;
    let mut in_escape = false;
    for ch in s.chars() {
        if in_escape {
            result.push(ch);
            if ch == 'm' {
                in_escape = false;
            }
        } else if ch == '\x1b' {
            in_escape = true;
            result.push(ch);
        } else {
            if visible >= max_visible {
                break;
            }
            result.push(ch);
            visible += 1;
        }
    }
    result.push_str(RESET);
    result
}

/// Render an empty line inside box borders.
///
/// Border characters are dim.
fn render_empty_line(width: usize) -> String {
    let inner = width.saturating_sub(2);
    format!("{}│{}{}{}│{}", DIM, RESET, " ".repeat(inner), DIM, RESET)
}

/// Render the top border with title: `╭─ zellai ─...─╮`
///
/// Border characters are dim, "zellai" is bold.
fn render_top_border(width: usize) -> String {
    let inner = width.saturating_sub(2); // "╭" + "╮"
    if inner < 10 {
        // Too narrow for title — just draw a plain dim border
        format!("{}╭{}╮{}", DIM, "─".repeat(inner), RESET)
    } else {
        let title_plain = "─ zellai ─";
        let title_chars = title_plain.chars().count();
        let remaining = inner.saturating_sub(title_chars);
        format!(
            "{}╭─ {}{}zellai{}{} ─{}╮{}",
            DIM,
            RESET,
            BOLD,
            RESET,
            DIM,
            "─".repeat(remaining),
            RESET
        )
    }
}

/// Render the bottom border: `╰─...─╯`
///
/// Border characters are dim.
fn render_bottom_border(width: usize) -> String {
    let inner = width.saturating_sub(2);
    format!("{}╰{}╯{}", DIM, "─".repeat(inner), RESET)
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
    // ANSI helpers
    // -----------------------------------------------------------------------

    #[test]
    fn test_visible_char_count_plain() {
        assert_eq!(visible_char_count("hello"), 5);
    }

    #[test]
    fn test_visible_char_count_with_ansi() {
        let s = format!("{}hello{}", GREEN, RESET);
        assert_eq!(visible_char_count(&s), 5);
    }

    #[test]
    fn test_visible_char_count_multiple_sequences() {
        let s = format!("{}a{}b{}c{}", RED, RESET, CYAN, RESET);
        assert_eq!(visible_char_count(&s), 3);
    }

    #[test]
    fn test_visible_char_count_empty() {
        assert_eq!(visible_char_count(""), 0);
    }

    #[test]
    fn test_visible_char_count_only_ansi() {
        let s = format!("{}{}{}", BOLD, DIM, RESET);
        assert_eq!(visible_char_count(&s), 0);
    }

    #[test]
    fn test_strip_ansi_plain() {
        assert_eq!(strip_ansi("hello"), "hello");
    }

    #[test]
    fn test_strip_ansi_removes_codes() {
        let s = format!("{}hello{} {}world{}", GREEN, RESET, CYAN, RESET);
        assert_eq!(strip_ansi(&s), "hello world");
    }

    #[test]
    fn test_strip_ansi_empty() {
        assert_eq!(strip_ansi(""), "");
    }

    // -----------------------------------------------------------------------
    // status_color
    // -----------------------------------------------------------------------

    #[test]
    fn test_status_color_thinking() {
        assert_eq!(status_color(&AgentStatusValue::Thinking), GREEN);
    }

    #[test]
    fn test_status_color_waiting() {
        assert_eq!(status_color(&AgentStatusValue::Waiting), YELLOW);
    }

    #[test]
    fn test_status_color_idle() {
        assert_eq!(status_color(&AgentStatusValue::Idle), DIM);
    }

    #[test]
    fn test_status_color_error() {
        assert_eq!(status_color(&AgentStatusValue::Error), RED);
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
        let plain = strip_ansi(&line);
        assert!(plain.starts_with('│'));
        assert!(plain.ends_with('│'));
        assert_eq!(visible_char_count(&line), 40);
        assert!(plain.contains("◉"));
        assert!(plain.contains("claude"));
        assert!(plain.contains("[feat/auth]"));
        assert!(plain.contains("thinking"));
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
        let plain = strip_ansi(&line);
        assert_eq!(visible_char_count(&line), 30);
        assert!(plain.contains("○"));
        assert!(plain.contains("codex"));
        assert!(plain.contains("idle"));
        // No branch brackets
        assert!(!plain.contains('['));
    }

    #[test]
    fn test_render_compact_card_has_color_codes() {
        let agent = make_agent(
            "sess-1",
            AgentKind::Claude,
            AgentStatusValue::Thinking,
            Some("main"),
            "/tmp",
            None,
            false,
        );
        let line = render_compact_card(&agent, 40);
        // Should contain ANSI green for thinking
        assert!(line.contains(GREEN));
        // Should contain ANSI cyan for branch
        assert!(line.contains(CYAN));
        // Should contain ANSI reset
        assert!(line.contains(RESET));
    }

    #[test]
    fn test_render_compact_card_correct_visible_width() {
        // Test with various statuses to ensure ANSI-aware width is correct
        for status in &[
            AgentStatusValue::Thinking,
            AgentStatusValue::Waiting,
            AgentStatusValue::Idle,
            AgentStatusValue::Error,
        ] {
            let agent = make_agent(
                "s1",
                AgentKind::Claude,
                status.clone(),
                Some("main"),
                "/tmp",
                None,
                matches!(status, AgentStatusValue::Waiting),
            );
            let line = render_compact_card(&agent, 50);
            assert_eq!(
                visible_char_count(&line),
                50,
                "Wrong visible width for status {:?}",
                status
            );
        }
    }

    // -----------------------------------------------------------------------
    // render_detailed_card
    // -----------------------------------------------------------------------

    #[test]
    fn test_render_detailed_card_returns_4_lines() {
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
        assert_eq!(lines.len(), 4);
        for line in &lines {
            let plain = strip_ansi(line);
            assert_eq!(
                visible_char_count(line),
                50,
                "Wrong visible width: '{}'",
                plain
            );
            assert!(plain.starts_with('│'));
            assert!(plain.ends_with('│'));
        }
        // Line 1: icon + name — status
        let plain0 = strip_ansi(&lines[0]);
        assert!(plain0.contains("◉"));
        assert!(plain0.contains("claude"));
        assert!(plain0.contains("thinking"));
        // Line 2: branch + dir
        let plain1 = strip_ansi(&lines[1]);
        assert!(plain1.contains("feat/auth"));
        // Line 3: metadata (no ports/PR → dim separator)
        let plain2 = strip_ansi(&lines[2]);
        assert!(plain2.contains("─")); // dim separator
        // Line 4: last message
        let plain3 = strip_ansi(&lines[3]);
        assert!(plain3.contains("Reading"));
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
        assert_eq!(lines.len(), 4);
        let plain3 = strip_ansi(&lines[3]);
        assert!(plain3.contains("(no message)"));
    }

    #[test]
    fn test_render_detailed_card_has_color_codes() {
        let agent = make_agent(
            "sess-1",
            AgentKind::Claude,
            AgentStatusValue::Error,
            Some("main"),
            "/tmp",
            Some("Crashed"),
            false,
        );
        let lines = render_detailed_card(&agent, 50);
        // Line 1 should have RED for error status
        assert!(lines[0].contains(RED));
        // Line 2 should have CYAN for branch
        assert!(lines[1].contains(CYAN));
    }

    #[test]
    fn test_render_detailed_card_correct_visible_width() {
        for status in &[
            AgentStatusValue::Thinking,
            AgentStatusValue::Waiting,
            AgentStatusValue::Idle,
            AgentStatusValue::Error,
        ] {
            let agent = make_agent(
                "s1",
                AgentKind::Claude,
                status.clone(),
                Some("feature-branch"),
                "/home/user/project",
                Some("Some message text here"),
                matches!(status, AgentStatusValue::Waiting),
            );
            let lines = render_detailed_card(&agent, 50);
            for (i, line) in lines.iter().enumerate() {
                assert_eq!(
                    visible_char_count(line),
                    50,
                    "Wrong visible width for status {:?}, line {}",
                    status,
                    i
                );
            }
        }
    }

    // -----------------------------------------------------------------------
    // choose_density
    // -----------------------------------------------------------------------

    #[test]
    fn test_choose_density_all_detailed_when_space() {
        // 3 agents, 3*4=12 rows needed, 12 available
        let result = choose_density(3, 0, 12, &CardDensity::Adaptive);
        assert_eq!(result, ResolvedDensity::AllDetailed);
    }

    #[test]
    fn test_choose_density_all_compact_when_tight() {
        // 5 agents, 5*4=20 rows needed (too many), 5*1=5 needed, 6 available
        let result = choose_density(5, 0, 6, &CardDensity::Adaptive);
        assert_eq!(result, ResolvedDensity::AllCompact);
    }

    #[test]
    fn test_choose_density_mixed_when_attention() {
        // 4 agents, 1 needs attention
        // Mixed: 1*4 + 3*1 = 7 rows, available=8 → fits
        // AllDetailed: 4*4 = 16 > 8 → doesn't fit
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
        // User forces detailed and it fits: 3 agents, 3*4=12, 12 available
        let result = choose_density(3, 0, 12, &CardDensity::Detailed);
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
        // Mixed would need 1*4 + 3*1 = 7, but only 4 available
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
        let plain0 = strip_ansi(&lines[0]);
        assert!(plain0.starts_with('╭'));
        assert!(plain0.ends_with('╮'));
        // Last line: bottom border
        let plain9 = strip_ansi(&lines[9]);
        assert!(plain9.starts_with('╰'));
        assert!(plain9.ends_with('╯'));
        // Check for "No agents connected" message somewhere
        let all = lines.join("\n");
        let plain_all = strip_ansi(&all);
        assert!(plain_all.contains("No agents connected"));
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
        // All lines should have visible width of exactly 40 chars
        for (i, line) in lines.iter().enumerate() {
            assert_eq!(
                visible_char_count(line),
                40,
                "line {} has wrong visible width: '{}'",
                i,
                strip_ansi(line)
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
        // Give enough space for mixed (1*4 + 2*1 = 6 agent rows + 2 chrome = 8)
        let mut config = default_sidebar_config();
        config.card_density = CardDensity::Adaptive;
        let lines = render_sidebar(&agents, &config, 10, 50);
        assert_eq!(lines.len(), 10);
        // The waiting agent should appear somewhere (it needs attention)
        let all = lines.join("\n");
        let plain_all = strip_ansi(&all);
        assert!(plain_all.contains("⚠")); // waiting icon for attention agent
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
        let plain = strip_ansi(&border);
        assert_eq!(visible_char_count(&border), 30);
        assert!(plain.starts_with('╭'));
        assert!(plain.ends_with('╮'));
        assert!(plain.contains("zellai"));
    }

    #[test]
    fn test_render_bottom_border_width() {
        let border = render_bottom_border(30);
        let plain = strip_ansi(&border);
        assert_eq!(visible_char_count(&border), 30);
        assert!(plain.starts_with('╰'));
        assert!(plain.ends_with('╯'));
    }

    #[test]
    fn test_render_empty_line_width() {
        let line = render_empty_line(20);
        let plain = strip_ansi(&line);
        assert_eq!(visible_char_count(&line), 20);
        assert!(plain.starts_with('│'));
        assert!(plain.ends_with('│'));
    }

    // -----------------------------------------------------------------------
    // Box line with ANSI — padding correctness
    // -----------------------------------------------------------------------

    #[test]
    fn test_render_box_line_with_ansi_padding() {
        // Content with ANSI codes should still result in correct visible width
        let content = format!(" {}hello{} ", GREEN, RESET);
        let line = render_box_line(&content, 20);
        assert_eq!(
            visible_char_count(&line),
            20,
            "box line has wrong visible width"
        );
        let plain = strip_ansi(&line);
        assert!(plain.contains("hello"));
        assert!(plain.starts_with('│'));
        assert!(plain.ends_with('│'));
    }

    #[test]
    fn test_render_box_line_plain_padding() {
        let line = render_box_line(" hi ", 10);
        assert_eq!(visible_char_count(&line), 10);
        let plain = strip_ansi(&line);
        assert!(plain.starts_with('│'));
        assert!(plain.ends_with('│'));
    }

    // -----------------------------------------------------------------------
    // ci_status_icon
    // -----------------------------------------------------------------------

    #[test]
    fn test_ci_status_icon_passing() {
        assert_eq!(ci_status_icon(&CiStatus::Passing), "✓");
    }

    #[test]
    fn test_ci_status_icon_failing() {
        assert_eq!(ci_status_icon(&CiStatus::Failing), "✗");
    }

    #[test]
    fn test_ci_status_icon_pending() {
        assert_eq!(ci_status_icon(&CiStatus::Pending), "⏳");
    }

    // -----------------------------------------------------------------------
    // render_metadata_line + detailed card with ports/PR
    // -----------------------------------------------------------------------

    /// Helper to create an AgentStatus with ports and PR metadata for testing.
    fn make_agent_with_metadata(
        session_id: &str,
        agent: AgentKind,
        status: AgentStatusValue,
        branch: Option<&str>,
        working_dir: &str,
        last_message: Option<&str>,
        needs_attention: bool,
        ports: Vec<u16>,
        pr_number: Option<u32>,
        pr_ci_status: Option<CiStatus>,
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
            ports,
            pr_number,
            pr_ci_status,
            needs_attention,
            updated_at: 1000,
        }
    }

    #[test]
    fn test_detailed_card_with_ports_only() {
        let agent = make_agent_with_metadata(
            "sess-p",
            AgentKind::Claude,
            AgentStatusValue::Thinking,
            Some("main"),
            "/tmp",
            Some("Working"),
            false,
            vec![3000, 5173],
            None,
            None,
        );
        let lines = render_detailed_card(&agent, 50);
        assert_eq!(lines.len(), 4);
        let plain2 = strip_ansi(&lines[2]);
        assert!(
            plain2.contains(":3000"),
            "should show port 3000: '{}'",
            plain2
        );
        assert!(
            plain2.contains(":5173"),
            "should show port 5173: '{}'",
            plain2
        );
        assert!(plain2.contains("🔌"), "should show plug icon: '{}'", plain2);
        // Ensure each line has correct visible width
        for line in &lines {
            assert_eq!(visible_char_count(line), 50);
        }
    }

    #[test]
    fn test_detailed_card_with_pr_only() {
        let agent = make_agent_with_metadata(
            "sess-pr",
            AgentKind::Codex,
            AgentStatusValue::Idle,
            Some("fix/bug"),
            "/tmp",
            Some("Done"),
            false,
            vec![],
            Some(42),
            Some(CiStatus::Passing),
        );
        let lines = render_detailed_card(&agent, 50);
        assert_eq!(lines.len(), 4);
        let plain2 = strip_ansi(&lines[2]);
        assert!(
            plain2.contains("PR #42"),
            "should show PR number: '{}'",
            plain2
        );
        assert!(
            plain2.contains("✓"),
            "should show passing icon: '{}'",
            plain2
        );
        assert!(
            plain2.contains("passing"),
            "should show 'passing': '{}'",
            plain2
        );
        for line in &lines {
            assert_eq!(visible_char_count(line), 50);
        }
    }

    #[test]
    fn test_detailed_card_with_ports_and_pr() {
        let agent = make_agent_with_metadata(
            "sess-both",
            AgentKind::Gemini,
            AgentStatusValue::Thinking,
            Some("dev"),
            "/tmp",
            Some("Building"),
            false,
            vec![3000],
            Some(99),
            Some(CiStatus::Failing),
        );
        let lines = render_detailed_card(&agent, 60);
        assert_eq!(lines.len(), 4);
        let plain2 = strip_ansi(&lines[2]);
        assert!(plain2.contains(":3000"), "should show port: '{}'", plain2);
        assert!(plain2.contains("PR #99"), "should show PR: '{}'", plain2);
        assert!(
            plain2.contains("✗"),
            "should show failing icon: '{}'",
            plain2
        );
        assert!(plain2.contains("|"), "should have separator: '{}'", plain2);
        for line in &lines {
            assert_eq!(visible_char_count(line), 60);
        }
    }

    #[test]
    fn test_detailed_card_with_neither_ports_nor_pr() {
        let agent = make_agent(
            "sess-none",
            AgentKind::Aider,
            AgentStatusValue::Idle,
            None,
            "/tmp",
            None,
            false,
        );
        let lines = render_detailed_card(&agent, 40);
        assert_eq!(lines.len(), 4);
        // Line 3 should be a dim separator (─)
        let plain2 = strip_ansi(&lines[2]);
        assert!(plain2.contains("─"), "should show separator: '{}'", plain2);
        assert!(!plain2.contains("🔌"), "should not show plug icon");
        assert!(!plain2.contains("PR"), "should not show PR");
        for line in &lines {
            assert_eq!(visible_char_count(line), 40);
        }
    }

    #[test]
    fn test_detailed_card_pr_without_ci_status() {
        let agent = make_agent_with_metadata(
            "sess-pr-no-ci",
            AgentKind::Claude,
            AgentStatusValue::Thinking,
            Some("main"),
            "/tmp",
            Some("Working"),
            false,
            vec![],
            Some(7),
            None,
        );
        let lines = render_detailed_card(&agent, 50);
        assert_eq!(lines.len(), 4);
        let plain2 = strip_ansi(&lines[2]);
        assert!(
            plain2.contains("PR #7"),
            "should show PR number: '{}'",
            plain2
        );
        // No CI icon since pr_ci_status is None
        assert!(!plain2.contains("✓"));
        assert!(!plain2.contains("✗"));
        for line in &lines {
            assert_eq!(visible_char_count(line), 50);
        }
    }

    #[test]
    fn test_detailed_card_pr_pending_ci() {
        let agent = make_agent_with_metadata(
            "sess-pr-pending",
            AgentKind::Claude,
            AgentStatusValue::Thinking,
            Some("main"),
            "/tmp",
            Some("Working"),
            false,
            vec![],
            Some(15),
            Some(CiStatus::Pending),
        );
        let lines = render_detailed_card(&agent, 50);
        assert_eq!(lines.len(), 4);
        let plain2 = strip_ansi(&lines[2]);
        assert!(
            plain2.contains("PR #15"),
            "should show PR number: '{}'",
            plain2
        );
        assert!(
            plain2.contains("⏳"),
            "should show pending icon: '{}'",
            plain2
        );
        assert!(
            plain2.contains("pending"),
            "should show 'pending': '{}'",
            plain2
        );
        for line in &lines {
            assert_eq!(visible_char_count(line), 50);
        }
    }

    #[test]
    fn test_render_metadata_line_ports_only() {
        let agent = make_agent_with_metadata(
            "test",
            AgentKind::Claude,
            AgentStatusValue::Thinking,
            None,
            "/tmp",
            None,
            false,
            vec![8080],
            None,
            None,
        );
        let content = render_metadata_line(&agent, 40);
        let plain = strip_ansi(&content);
        assert!(plain.contains(":8080"));
        assert!(plain.contains("🔌"));
    }

    #[test]
    fn test_render_metadata_line_empty() {
        let agent = make_agent(
            "test",
            AgentKind::Claude,
            AgentStatusValue::Thinking,
            None,
            "/tmp",
            None,
            false,
        );
        let content = render_metadata_line(&agent, 40);
        let plain = strip_ansi(&content);
        assert!(plain.contains("─"), "empty metadata should show separator");
    }
}
