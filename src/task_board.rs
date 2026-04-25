//! Pure-data model for the Orchestrator Task Board.
//!
//! The task board tracks task-level state across the active team, with Kanban
//! columns (todo/in-progress/review/done/blocked), DAG dependency tree,
//! aggregate stats, and optional cost tracking.
//!
//! This module contains only data types and pure-logic helpers — no file I/O
//! and no `zellij_tile` imports — so everything is unit-testable on the host.

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap, HashSet, VecDeque};

// ---------------------------------------------------------------------------
// Enums
// ---------------------------------------------------------------------------

/// Status of a task in the orchestrator's task board.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TaskStatus {
    Todo,
    InProgress,
    Review,
    Done,
    Blocked,
}

// ---------------------------------------------------------------------------
// Task
// ---------------------------------------------------------------------------

/// A single task tracked by the orchestrator.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Task {
    pub id: String,
    pub title: String,
    pub status: TaskStatus,
    /// Agent session_id assigned to this task (if any).
    pub assigned_to: Option<String>,
    /// IDs of tasks this task depends on.
    pub depends_on: Vec<String>,
    /// Optional cost/token count.
    pub cost: Option<f64>,
    /// Unix epoch seconds when task was created.
    pub created_at: u64,
    /// Unix epoch seconds when task was last updated.
    pub updated_at: u64,
}

// ---------------------------------------------------------------------------
// TaskBoard
// ---------------------------------------------------------------------------

/// The full task board state, serialized to/from a JSON file.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct TaskBoard {
    pub tasks: Vec<Task>,
}

/// Aggregate statistics for the task board.
#[derive(Debug, Clone, PartialEq)]
pub struct TaskBoardStats {
    pub total: usize,
    pub done: usize,
    pub blocked: usize,
    /// Fraction of tasks that are done (0.0–1.0). `0.0` when `total == 0`.
    pub success_rate: f64,
    /// Sum of all task costs, or `None` if no task has cost data.
    pub total_cost: Option<f64>,
}

impl TaskBoard {
    /// Group tasks by status for Kanban rendering.
    ///
    /// Returns a `BTreeMap` keyed by `&TaskStatus` (ordered) with vectors of
    /// task references belonging to that status. Empty statuses are not
    /// included — the caller can check against the configured columns.
    pub fn tasks_by_status(&self) -> BTreeMap<&TaskStatus, Vec<&Task>> {
        let mut map: BTreeMap<&TaskStatus, Vec<&Task>> = BTreeMap::new();
        for task in &self.tasks {
            map.entry(&task.status).or_default().push(task);
        }
        map
    }

    /// Compute DAG levels (BFS from roots) for the ASCII tree view.
    ///
    /// Level 0 contains tasks with no dependencies (or whose dependencies are
    /// all missing from the board). Subsequent levels contain tasks whose
    /// dependencies are all in earlier levels.
    ///
    /// Tasks involved in dependency cycles are placed in a final catch-all
    /// level so they are never silently dropped.
    pub fn dependency_levels(&self) -> Vec<Vec<&Task>> {
        if self.tasks.is_empty() {
            return Vec::new();
        }

        // Build lookup: id → &Task
        let task_map: HashMap<&str, &Task> =
            self.tasks.iter().map(|t| (t.id.as_str(), t)).collect();

        // Build in-degree map (only count deps that exist on the board)
        let all_ids: HashSet<&str> = task_map.keys().copied().collect();
        let mut in_degree: HashMap<&str, usize> = HashMap::new();
        // dependents: dep_id → list of task ids that depend on dep_id
        let mut dependents: HashMap<&str, Vec<&str>> = HashMap::new();

        for task in &self.tasks {
            let relevant_deps: usize = task
                .depends_on
                .iter()
                .filter(|d| all_ids.contains(d.as_str()))
                .count();
            in_degree.insert(task.id.as_str(), relevant_deps);

            for dep in &task.depends_on {
                if all_ids.contains(dep.as_str()) {
                    dependents
                        .entry(dep.as_str())
                        .or_default()
                        .push(task.id.as_str());
                }
            }
        }

        let mut levels: Vec<Vec<&Task>> = Vec::new();
        let mut placed: HashSet<&str> = HashSet::new();

        // BFS by levels: start with in-degree == 0
        let mut queue: VecDeque<&str> = in_degree
            .iter()
            .filter(|(_, deg)| **deg == 0)
            .map(|(&id, _)| id)
            .collect();

        while !queue.is_empty() {
            let mut level: Vec<&Task> = Vec::new();
            let mut next_queue: VecDeque<&str> = VecDeque::new();

            for id in &queue {
                if let Some(&task) = task_map.get(id) {
                    level.push(task);
                    placed.insert(id);
                }
            }

            for id in &queue {
                if let Some(deps) = dependents.get(id) {
                    for &dep_id in deps {
                        if let Some(deg) = in_degree.get_mut(dep_id) {
                            *deg = deg.saturating_sub(1);
                            if *deg == 0 && !placed.contains(dep_id) {
                                next_queue.push_back(dep_id);
                            }
                        }
                    }
                }
            }

            if !level.is_empty() {
                // Sort within level by id for deterministic output
                level.sort_by(|a, b| a.id.cmp(&b.id));
                levels.push(level);
            }

            queue = next_queue;
        }

        // Catch-all: tasks in cycles (never reached in-degree 0)
        let mut cycled: Vec<&Task> = self
            .tasks
            .iter()
            .filter(|t| !placed.contains(t.id.as_str()))
            .collect();
        if !cycled.is_empty() {
            cycled.sort_by(|a, b| a.id.cmp(&b.id));
            levels.push(cycled);
        }

        levels
    }

    /// Compute aggregate statistics for the task board.
    pub fn aggregate_stats(&self) -> TaskBoardStats {
        let total = self.tasks.len();
        let done = self
            .tasks
            .iter()
            .filter(|t| t.status == TaskStatus::Done)
            .count();
        let blocked = self
            .tasks
            .iter()
            .filter(|t| t.status == TaskStatus::Blocked)
            .count();
        let success_rate = if total > 0 {
            done as f64 / total as f64
        } else {
            0.0
        };

        let mut has_any_cost = false;
        let mut cost_sum = 0.0;
        for task in &self.tasks {
            if let Some(c) = task.cost {
                has_any_cost = true;
                cost_sum += c;
            }
        }
        let total_cost = if has_any_cost { Some(cost_sum) } else { None };

        TaskBoardStats {
            total,
            done,
            blocked,
            success_rate,
            total_cost,
        }
    }
}

// ---------------------------------------------------------------------------
// Parsing
// ---------------------------------------------------------------------------

/// Parse a JSON string into a [`TaskBoard`].
///
/// The task board file is stored at
/// `<sessions_dir>/<workspace>/task_board.json`, but this function does not
/// perform any file I/O — the caller supplies the raw JSON content.
pub fn parse_task_board(json: &str) -> Result<TaskBoard, serde_json::Error> {
    serde_json::from_str(json)
}

// ---------------------------------------------------------------------------
// ANSI color constants (local to avoid coupling with sidebar.rs)
// ---------------------------------------------------------------------------

const RESET: &str = "\x1b[0m";
const BOLD: &str = "\x1b[1m";
const DIM: &str = "\x1b[2m";
const GREEN: &str = "\x1b[32m";
const YELLOW: &str = "\x1b[33m";
const RED: &str = "\x1b[31m";

// ---------------------------------------------------------------------------
// Rendering helpers
// ---------------------------------------------------------------------------

/// All statuses in the canonical Kanban column order.
const KANBAN_COLUMNS: &[TaskStatus] = &[
    TaskStatus::Todo,
    TaskStatus::InProgress,
    TaskStatus::Review,
    TaskStatus::Done,
    TaskStatus::Blocked,
];

/// Display name for a task status.
fn status_display_name(status: &TaskStatus) -> &'static str {
    match status {
        TaskStatus::Todo => "Todo",
        TaskStatus::InProgress => "In Progress",
        TaskStatus::Review => "Review",
        TaskStatus::Done => "Done",
        TaskStatus::Blocked => "Blocked",
    }
}

/// Unicode icon for a task status.
fn status_icon(status: &TaskStatus) -> &'static str {
    match status {
        TaskStatus::Todo => "○",
        TaskStatus::InProgress => "◐",
        TaskStatus::Review => "◑",
        TaskStatus::Done => "●",
        TaskStatus::Blocked => "✕",
    }
}

/// ANSI color for a task status.
fn status_color(status: &TaskStatus) -> &'static str {
    match status {
        TaskStatus::Todo => DIM,
        TaskStatus::InProgress => YELLOW,
        TaskStatus::Review => YELLOW,
        TaskStatus::Done => GREEN,
        TaskStatus::Blocked => RED,
    }
}

/// Count visible characters, skipping ANSI escape sequences.
fn visible_len(s: &str) -> usize {
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

/// Strip ANSI escape sequences, returning only visible text.
#[cfg(test)]
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

/// Pad or truncate a string (with ANSI awareness) to exactly `width` visible chars.
fn pad_to_width(s: &str, width: usize) -> String {
    let vis = visible_len(s);
    if vis >= width {
        // Truncate: walk chars, count visible, stop at width
        truncate_ansi(s, width)
    } else {
        let padding = width - vis;
        format!("{}{}", s, " ".repeat(padding))
    }
}

/// Truncate a string with ANSI sequences to `max` visible characters.
/// Adds RESET at end to avoid leaking escape codes.
fn truncate_ansi(s: &str, max: usize) -> String {
    if max == 0 {
        return String::new();
    }
    let mut result = String::new();
    let mut vis_count = 0;
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
            if vis_count >= max {
                break;
            }
            result.push(ch);
            vis_count += 1;
        }
    }
    // Ensure RESET is appended so we don't leak colors
    if result.contains('\x1b') {
        result.push_str(RESET);
    }
    result
}

/// Truncate plain text with ellipsis if needed.
fn truncate_plain(s: &str, max: usize) -> String {
    if max == 0 {
        return String::new();
    }
    let chars: Vec<char> = s.chars().collect();
    if chars.len() <= max {
        s.to_string()
    } else if max == 1 {
        "…".to_string()
    } else {
        let truncated: String = chars[..max - 1].iter().collect();
        format!("{}…", truncated)
    }
}

// ---------------------------------------------------------------------------
// Public rendering API
// ---------------------------------------------------------------------------

/// Render a horizontal Kanban board with columns for each status.
///
/// Columns: Todo | In Progress | Review | Done | Blocked.
/// Each column header shows the status name and task count.
/// Output is exactly `rows` lines, each padded to `cols` visible characters.
pub fn render_kanban(board: &TaskBoard, rows: usize, cols: usize) -> Vec<String> {
    if rows == 0 || cols == 0 {
        return Vec::new();
    }

    let by_status = board.tasks_by_status();
    let num_cols = KANBAN_COLUMNS.len();

    // Column widths: distribute evenly, give leftover to rightmost columns
    let col_width = cols.checked_div(num_cols).unwrap_or(cols);
    let remainder = cols.checked_rem(num_cols).unwrap_or(0);

    let mut col_widths: Vec<usize> = vec![col_width; num_cols];
    // Distribute remainder to rightmost columns
    for i in 0..remainder {
        col_widths[num_cols - 1 - i] += 1;
    }

    let mut lines: Vec<String> = Vec::with_capacity(rows);

    // Row 0: top border  ┬ separated
    let mut top_border = String::new();
    for (i, &w) in col_widths.iter().enumerate() {
        if i == 0 {
            top_border.push('┌');
        } else {
            top_border.push('┬');
        }
        for _ in 0..w.saturating_sub(1) {
            top_border.push('─');
        }
    }
    // Close the last column
    if !top_border.is_empty() {
        // Replace last dash area — actually just add closing corner
        top_border.push('┐');
    }
    lines.push(pad_to_width(&top_border, cols));

    if rows < 2 {
        return lines;
    }

    // Row 1: column headers
    let mut header = String::new();
    for (i, status) in KANBAN_COLUMNS.iter().enumerate() {
        let w = col_widths[i];
        let inner = w.saturating_sub(1); // 1 char for the leading │
        let count = by_status.get(status).map_or(0, |v| v.len());
        let label = format!(
            "{}{} ({}){}",
            BOLD,
            status_display_name(status),
            count,
            RESET
        );
        let label_plain = format!("{} ({})", status_display_name(status), count);
        let display = if label_plain.len() > inner {
            let trunc = truncate_plain(&label_plain, inner);
            format!("{}{}{}", BOLD, trunc, RESET)
        } else {
            label.clone()
        };
        header.push('│');
        header.push_str(&pad_to_width(&display, inner));
    }
    header.push('│');
    lines.push(pad_to_width(&header, cols));

    if rows < 3 {
        return pad_lines(lines, rows, cols);
    }

    // Row 2: separator under headers
    let mut sep = String::new();
    for (i, &w) in col_widths.iter().enumerate() {
        if i == 0 {
            sep.push('├');
        } else {
            sep.push('┼');
        }
        for _ in 0..w.saturating_sub(1) {
            sep.push('─');
        }
    }
    sep.push('┤');
    lines.push(pad_to_width(&sep, cols));

    if rows < 4 {
        return pad_lines(lines, rows, cols);
    }

    // Content rows: task list in each column
    let content_rows = rows.saturating_sub(4); // top border + header + sep + bottom border
    for row_idx in 0..content_rows {
        let mut line = String::new();
        for (col_idx, status) in KANBAN_COLUMNS.iter().enumerate() {
            let w = col_widths[col_idx];
            let inner = w.saturating_sub(1);
            let tasks = by_status.get(status);
            let cell = if let Some(tasks) = tasks {
                if row_idx < tasks.len() {
                    let task = tasks[row_idx];
                    let icon = status_icon(status);
                    let color = status_color(status);
                    // icon + space + title
                    let prefix_plain = format!("{} ", icon);
                    let title_space = inner.saturating_sub(prefix_plain.chars().count());
                    let title = truncate_plain(&task.title, title_space);
                    format!("{}{} {}{}", color, icon, title, RESET)
                } else {
                    String::new()
                }
            } else {
                String::new()
            };
            line.push('│');
            line.push_str(&pad_to_width(&cell, inner));
        }
        line.push('│');
        lines.push(pad_to_width(&line, cols));
    }

    // Bottom border
    let mut bottom = String::new();
    for (i, &w) in col_widths.iter().enumerate() {
        if i == 0 {
            bottom.push('└');
        } else {
            bottom.push('┴');
        }
        for _ in 0..w.saturating_sub(1) {
            bottom.push('─');
        }
    }
    bottom.push('┘');
    lines.push(pad_to_width(&bottom, cols));

    pad_lines(lines, rows, cols)
}

/// Render the dependency DAG as a level-grouped ASCII tree.
///
/// Uses `dependency_levels()` to get tasks organized by level.
/// Each level is labeled: `Level 0 (roots):`, `Level 1:`, etc.
/// Tasks within a level show a status icon and optional assigned agent.
/// Output is exactly `rows` lines.
pub fn render_dag(board: &TaskBoard, rows: usize, cols: usize) -> Vec<String> {
    if rows == 0 || cols == 0 {
        return Vec::new();
    }

    let levels = board.dependency_levels();

    let mut content_lines: Vec<String> = Vec::new();

    if levels.is_empty() {
        content_lines.push(format!("{}(no tasks){}", DIM, RESET));
    } else {
        let total_levels = levels.len();
        for (level_idx, tasks) in levels.iter().enumerate() {
            // Check if this is the cycle catch-all level
            // Cycles are in the last level if any tasks were not placed by BFS
            let is_cycle_level = level_idx == total_levels - 1 && {
                // Detect: a level is the cycle level if it contains tasks whose
                // dependencies are within the same level (not all resolved in earlier levels).
                // Simple heuristic: if level_idx > 0 and all tasks in this level have
                // deps pointing to each other, it's cycles. But we can just check if
                // any task has a dep on another task in the same level.
                let level_ids: HashSet<&str> = tasks.iter().map(|t| t.id.as_str()).collect();
                tasks
                    .iter()
                    .any(|t| t.depends_on.iter().any(|d| level_ids.contains(d.as_str())))
            };

            // Level header
            let header = if is_cycle_level {
                format!("{}{}Cycle (unresolved):{}", BOLD, RED, RESET)
            } else if level_idx == 0 {
                format!("{}Level 0 (roots):{}", BOLD, RESET)
            } else {
                format!("{}Level {}:{}", BOLD, level_idx, RESET)
            };
            content_lines.push(truncate_ansi(&header, cols));

            // Tasks within this level
            for task in tasks {
                let icon = status_icon(&task.status);
                let color = status_color(&task.status);
                let agent_suffix = if let Some(ref agent) = task.assigned_to {
                    format!(" {}[{}]{}", DIM, agent, RESET)
                } else {
                    String::new()
                };
                let line = format!(
                    "  {}{} {}{}{}",
                    color, icon, task.title, RESET, agent_suffix
                );
                content_lines.push(truncate_ansi(&line, cols));
            }

            // Blank line between levels (not after the last)
            if level_idx + 1 < total_levels {
                content_lines.push(String::new());
            }
        }
    }

    // Truncate to rows, pad if needed
    let mut lines: Vec<String> = Vec::with_capacity(rows);
    for (i, line) in content_lines.into_iter().enumerate() {
        if i >= rows {
            break;
        }
        lines.push(pad_to_width(&line, cols));
    }
    // Pad remaining rows
    while lines.len() < rows {
        lines.push(" ".repeat(cols));
    }
    lines
}

/// Render a single-line stats summary.
///
/// Format: `Tasks: N | Done: D (P%) | Blocked: B | Cost: $X.XX`
/// Omits cost section if `total_cost` is `None`. Truncated to fit `cols`.
pub fn render_stats_line(stats: &TaskBoardStats, cols: usize) -> String {
    if cols == 0 {
        return String::new();
    }

    let pct = if stats.total > 0 {
        (stats.success_rate * 100.0).round() as u32
    } else {
        0
    };

    let mut parts: Vec<String> = vec![
        format!("Tasks: {}", stats.total),
        format!("Done: {} ({}%)", stats.done, pct),
        format!("Blocked: {}", stats.blocked),
    ];

    if let Some(cost) = stats.total_cost {
        parts.push(format!("Cost: ${:.2}", cost));
    }

    let full = parts.join(" | ");
    truncate_plain(&full, cols)
}

/// Pad/truncate a `Vec<String>` to exactly `rows` lines, each `cols` wide.
fn pad_lines(mut lines: Vec<String>, rows: usize, cols: usize) -> Vec<String> {
    lines.truncate(rows);
    while lines.len() < rows {
        lines.push(" ".repeat(cols));
    }
    lines
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper: create a minimal task with sensible defaults.
    fn make_task(id: &str, title: &str, status: TaskStatus) -> Task {
        Task {
            id: id.to_string(),
            title: title.to_string(),
            status,
            assigned_to: None,
            depends_on: Vec::new(),
            cost: None,
            created_at: 1000,
            updated_at: 1000,
        }
    }

    // -----------------------------------------------------------------------
    // Parsing tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_parse_empty_board() {
        let json = r#"{"tasks":[]}"#;
        let board = parse_task_board(json).expect("should parse");
        assert!(board.tasks.is_empty());
    }

    #[test]
    fn test_parse_single_task() {
        let json = r#"{
            "tasks": [{
                "id": "t1",
                "title": "Setup project",
                "status": "todo",
                "assigned_to": null,
                "depends_on": [],
                "cost": null,
                "created_at": 1000,
                "updated_at": 1000
            }]
        }"#;
        let board = parse_task_board(json).expect("should parse");
        assert_eq!(board.tasks.len(), 1);
        assert_eq!(board.tasks[0].id, "t1");
        assert_eq!(board.tasks[0].status, TaskStatus::Todo);
    }

    #[test]
    fn test_parse_all_statuses() {
        let json = r#"{
            "tasks": [
                {"id":"a","title":"A","status":"todo","assigned_to":null,"depends_on":[],"cost":null,"created_at":1,"updated_at":1},
                {"id":"b","title":"B","status":"in-progress","assigned_to":"agent-1","depends_on":["a"],"cost":0.5,"created_at":2,"updated_at":3},
                {"id":"c","title":"C","status":"review","assigned_to":null,"depends_on":[],"cost":null,"created_at":1,"updated_at":1},
                {"id":"d","title":"D","status":"done","assigned_to":null,"depends_on":[],"cost":1.25,"created_at":1,"updated_at":1},
                {"id":"e","title":"E","status":"blocked","assigned_to":null,"depends_on":["b"],"cost":null,"created_at":1,"updated_at":1}
            ]
        }"#;
        let board = parse_task_board(json).expect("should parse");
        assert_eq!(board.tasks.len(), 5);
        assert_eq!(board.tasks[0].status, TaskStatus::Todo);
        assert_eq!(board.tasks[1].status, TaskStatus::InProgress);
        assert_eq!(board.tasks[1].assigned_to, Some("agent-1".to_string()));
        assert_eq!(board.tasks[1].cost, Some(0.5));
        assert_eq!(board.tasks[2].status, TaskStatus::Review);
        assert_eq!(board.tasks[3].status, TaskStatus::Done);
        assert_eq!(board.tasks[4].status, TaskStatus::Blocked);
    }

    #[test]
    fn test_parse_invalid_json() {
        let result = parse_task_board("not json");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_missing_field() {
        // Missing required "title" field
        let json = r#"{"tasks":[{"id":"t1","status":"todo"}]}"#;
        let result = parse_task_board(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_roundtrip_serialization() {
        let board = TaskBoard {
            tasks: vec![
                make_task("t1", "First", TaskStatus::Todo),
                make_task("t2", "Second", TaskStatus::Done),
            ],
        };
        let json = serde_json::to_string(&board).expect("should serialize");
        let parsed = parse_task_board(&json).expect("should deserialize");
        assert_eq!(board, parsed);
    }

    // -----------------------------------------------------------------------
    // tasks_by_status tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_tasks_by_status_empty() {
        let board = TaskBoard::default();
        let grouped = board.tasks_by_status();
        assert!(grouped.is_empty());
    }

    #[test]
    fn test_tasks_by_status_grouping() {
        let board = TaskBoard {
            tasks: vec![
                make_task("t1", "A", TaskStatus::Todo),
                make_task("t2", "B", TaskStatus::Todo),
                make_task("t3", "C", TaskStatus::Done),
                make_task("t4", "D", TaskStatus::InProgress),
            ],
        };
        let grouped = board.tasks_by_status();
        assert_eq!(grouped.get(&TaskStatus::Todo).unwrap().len(), 2);
        assert_eq!(grouped.get(&TaskStatus::Done).unwrap().len(), 1);
        assert_eq!(grouped.get(&TaskStatus::InProgress).unwrap().len(), 1);
        assert!(grouped.get(&TaskStatus::Blocked).is_none());
        assert!(grouped.get(&TaskStatus::Review).is_none());
    }

    // -----------------------------------------------------------------------
    // dependency_levels tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_dependency_levels_empty() {
        let board = TaskBoard::default();
        let levels = board.dependency_levels();
        assert!(levels.is_empty());
    }

    #[test]
    fn test_dependency_levels_no_deps() {
        let board = TaskBoard {
            tasks: vec![
                make_task("t1", "A", TaskStatus::Todo),
                make_task("t2", "B", TaskStatus::Todo),
            ],
        };
        let levels = board.dependency_levels();
        assert_eq!(levels.len(), 1);
        assert_eq!(levels[0].len(), 2);
    }

    #[test]
    fn test_dependency_levels_chain() {
        // t1 → t2 → t3 (linear chain)
        let mut t2 = make_task("t2", "B", TaskStatus::InProgress);
        t2.depends_on = vec!["t1".to_string()];
        let mut t3 = make_task("t3", "C", TaskStatus::Todo);
        t3.depends_on = vec!["t2".to_string()];

        let board = TaskBoard {
            tasks: vec![make_task("t1", "A", TaskStatus::Done), t2, t3],
        };
        let levels = board.dependency_levels();
        assert_eq!(levels.len(), 3);
        assert_eq!(levels[0][0].id, "t1");
        assert_eq!(levels[1][0].id, "t2");
        assert_eq!(levels[2][0].id, "t3");
    }

    #[test]
    fn test_dependency_levels_diamond() {
        // t1 → t2, t1 → t3, t2 → t4, t3 → t4
        let mut t2 = make_task("t2", "B", TaskStatus::Todo);
        t2.depends_on = vec!["t1".to_string()];
        let mut t3 = make_task("t3", "C", TaskStatus::Todo);
        t3.depends_on = vec!["t1".to_string()];
        let mut t4 = make_task("t4", "D", TaskStatus::Todo);
        t4.depends_on = vec!["t2".to_string(), "t3".to_string()];

        let board = TaskBoard {
            tasks: vec![make_task("t1", "A", TaskStatus::Done), t2, t3, t4],
        };
        let levels = board.dependency_levels();
        assert_eq!(levels.len(), 3);
        assert_eq!(levels[0].len(), 1); // t1
        assert_eq!(levels[1].len(), 2); // t2, t3
        assert_eq!(levels[2].len(), 1); // t4
        assert_eq!(levels[2][0].id, "t4");
    }

    #[test]
    fn test_dependency_levels_missing_dep() {
        // t2 depends on "phantom" which doesn't exist on the board
        let mut t2 = make_task("t2", "B", TaskStatus::Todo);
        t2.depends_on = vec!["phantom".to_string()];

        let board = TaskBoard {
            tasks: vec![make_task("t1", "A", TaskStatus::Done), t2],
        };
        let levels = board.dependency_levels();
        // Both should be at level 0 since "phantom" isn't on the board
        assert_eq!(levels.len(), 1);
        assert_eq!(levels[0].len(), 2);
    }

    #[test]
    fn test_dependency_levels_cycle() {
        // t1 → t2 → t1 (cycle)
        let mut t1 = make_task("t1", "A", TaskStatus::Todo);
        t1.depends_on = vec!["t2".to_string()];
        let mut t2 = make_task("t2", "B", TaskStatus::Todo);
        t2.depends_on = vec!["t1".to_string()];

        let board = TaskBoard {
            tasks: vec![t1, t2],
        };
        let levels = board.dependency_levels();
        // Both tasks are in a cycle, so they go in the catch-all level
        assert_eq!(levels.len(), 1);
        assert_eq!(levels[0].len(), 2);
    }

    #[test]
    fn test_dependency_levels_cycle_plus_roots() {
        // t1 has no deps (root), t2 → t3 → t2 (cycle)
        let mut t2 = make_task("t2", "B", TaskStatus::Todo);
        t2.depends_on = vec!["t3".to_string()];
        let mut t3 = make_task("t3", "C", TaskStatus::Todo);
        t3.depends_on = vec!["t2".to_string()];

        let board = TaskBoard {
            tasks: vec![make_task("t1", "A", TaskStatus::Done), t2, t3],
        };
        let levels = board.dependency_levels();
        // Level 0: t1 (root). Catch-all level: t2, t3 (cycle).
        assert_eq!(levels.len(), 2);
        assert_eq!(levels[0].len(), 1);
        assert_eq!(levels[0][0].id, "t1");
        assert_eq!(levels[1].len(), 2);
    }

    // -----------------------------------------------------------------------
    // aggregate_stats tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_aggregate_stats_empty() {
        let board = TaskBoard::default();
        let stats = board.aggregate_stats();
        assert_eq!(stats.total, 0);
        assert_eq!(stats.done, 0);
        assert_eq!(stats.blocked, 0);
        assert_eq!(stats.success_rate, 0.0);
        assert_eq!(stats.total_cost, None);
    }

    #[test]
    fn test_aggregate_stats_basic() {
        let board = TaskBoard {
            tasks: vec![
                make_task("t1", "A", TaskStatus::Done),
                make_task("t2", "B", TaskStatus::Done),
                make_task("t3", "C", TaskStatus::InProgress),
                make_task("t4", "D", TaskStatus::Blocked),
                make_task("t5", "E", TaskStatus::Todo),
            ],
        };
        let stats = board.aggregate_stats();
        assert_eq!(stats.total, 5);
        assert_eq!(stats.done, 2);
        assert_eq!(stats.blocked, 1);
        assert!((stats.success_rate - 0.4).abs() < f64::EPSILON);
        assert_eq!(stats.total_cost, None);
    }

    #[test]
    fn test_aggregate_stats_with_costs() {
        let mut t1 = make_task("t1", "A", TaskStatus::Done);
        t1.cost = Some(1.50);
        let mut t2 = make_task("t2", "B", TaskStatus::InProgress);
        t2.cost = Some(0.75);
        let t3 = make_task("t3", "C", TaskStatus::Todo); // no cost

        let board = TaskBoard {
            tasks: vec![t1, t2, t3],
        };
        let stats = board.aggregate_stats();
        assert_eq!(stats.total, 3);
        assert_eq!(stats.done, 1);
        assert!((stats.total_cost.unwrap() - 2.25).abs() < f64::EPSILON);
    }

    #[test]
    fn test_aggregate_stats_all_done() {
        let board = TaskBoard {
            tasks: vec![
                make_task("t1", "A", TaskStatus::Done),
                make_task("t2", "B", TaskStatus::Done),
            ],
        };
        let stats = board.aggregate_stats();
        assert_eq!(stats.success_rate, 1.0);
    }

    // -----------------------------------------------------------------------
    // TaskStatus serde tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_task_status_serde_kebab_case() {
        assert_eq!(
            serde_json::to_string(&TaskStatus::Todo).unwrap(),
            r#""todo""#
        );
        assert_eq!(
            serde_json::to_string(&TaskStatus::InProgress).unwrap(),
            r#""in-progress""#
        );
        assert_eq!(
            serde_json::to_string(&TaskStatus::Review).unwrap(),
            r#""review""#
        );
        assert_eq!(
            serde_json::to_string(&TaskStatus::Done).unwrap(),
            r#""done""#
        );
        assert_eq!(
            serde_json::to_string(&TaskStatus::Blocked).unwrap(),
            r#""blocked""#
        );
    }

    // -----------------------------------------------------------------------
    // Rendering tests — Kanban
    // -----------------------------------------------------------------------

    #[test]
    fn test_render_kanban_empty_board() {
        let board = TaskBoard { tasks: vec![] };
        let lines = render_kanban(&board, 10, 60);
        assert_eq!(lines.len(), 10);
        // Header row should contain "(0)" for each column
        let header_plain = strip_ansi(&lines[1]);
        assert!(header_plain.contains("Todo (0)"));
        assert!(header_plain.contains("Done (0)"));
        assert!(header_plain.contains("Blocked (0)"));
    }

    #[test]
    fn test_render_kanban_various_statuses() {
        let board = TaskBoard {
            tasks: vec![
                make_task("t1", "Setup", TaskStatus::Todo),
                make_task("t2", "Build", TaskStatus::InProgress),
                make_task("t3", "Ship", TaskStatus::Done),
                make_task("t4", "Test", TaskStatus::Review),
                make_task("t5", "Fix", TaskStatus::Blocked),
            ],
        };
        let lines = render_kanban(&board, 10, 80);
        assert_eq!(lines.len(), 10);
        // Check that the header shows correct counts
        let header_plain = strip_ansi(&lines[1]);
        assert!(header_plain.contains("Todo (1)"));
        assert!(header_plain.contains("In Progress (1)"));
        assert!(header_plain.contains("Review (1)"));
        assert!(header_plain.contains("Done (1)"));
        assert!(header_plain.contains("Blocked (1)"));
    }

    #[test]
    fn test_render_kanban_respects_rows() {
        let board = TaskBoard {
            tasks: vec![
                make_task("t1", "A", TaskStatus::Todo),
                make_task("t2", "B", TaskStatus::Todo),
            ],
        };
        let lines = render_kanban(&board, 5, 60);
        assert_eq!(lines.len(), 5);
    }

    #[test]
    fn test_render_kanban_respects_cols() {
        let board = TaskBoard {
            tasks: vec![make_task("t1", "A", TaskStatus::Todo)],
        };
        let lines = render_kanban(&board, 8, 40);
        assert_eq!(lines.len(), 8);
        // Each line's visible length should not exceed 40
        for line in &lines {
            assert!(
                visible_len(line) <= 40,
                "line too wide: visible_len={}, line={:?}",
                visible_len(line),
                strip_ansi(line)
            );
        }
    }

    #[test]
    fn test_render_kanban_truncates_long_titles() {
        let board = TaskBoard {
            tasks: vec![make_task(
                "t1",
                "This is a very long task title that should be truncated",
                TaskStatus::Todo,
            )],
        };
        // Very narrow: 25 cols → 5 per column
        let lines = render_kanban(&board, 8, 25);
        assert_eq!(lines.len(), 8);
        // The task title should be truncated (contain ellipsis or be short)
        // Just verify no line exceeds cols in visible width
        for line in &lines {
            assert!(
                visible_len(line) <= 25,
                "line too wide: visible_len={}, line={:?}",
                visible_len(line),
                strip_ansi(line)
            );
        }
    }

    #[test]
    fn test_render_kanban_zero_size() {
        let board = TaskBoard { tasks: vec![] };
        assert!(render_kanban(&board, 0, 60).is_empty());
        assert!(render_kanban(&board, 10, 0).is_empty());
    }

    // -----------------------------------------------------------------------
    // Rendering tests — DAG
    // -----------------------------------------------------------------------

    #[test]
    fn test_render_dag_empty_board() {
        let board = TaskBoard { tasks: vec![] };
        let lines = render_dag(&board, 10, 60);
        assert_eq!(lines.len(), 10);
        let first_plain = strip_ansi(&lines[0]);
        assert!(first_plain.contains("(no tasks)"));
    }

    #[test]
    fn test_render_dag_simple_chain() {
        // A → B → C (3 levels)
        let t1 = make_task("t1", "A", TaskStatus::Done);
        let mut t2 = make_task("t2", "B", TaskStatus::InProgress);
        t2.depends_on = vec!["t1".to_string()];
        let mut t3 = make_task("t3", "C", TaskStatus::Todo);
        t3.depends_on = vec!["t2".to_string()];

        let board = TaskBoard {
            tasks: vec![t1, t2, t3],
        };
        let lines = render_dag(&board, 15, 60);
        assert_eq!(lines.len(), 15);

        // Should have 3 level headers
        let plain_text: String = lines
            .iter()
            .map(|l| strip_ansi(l))
            .collect::<Vec<_>>()
            .join("\n");
        assert!(plain_text.contains("Level 0 (roots):"));
        assert!(plain_text.contains("Level 1:"));
        assert!(plain_text.contains("Level 2:"));
    }

    #[test]
    fn test_render_dag_diamond_dependency() {
        // A → B, A → C, B → D, C → D (diamond)
        let t_a = make_task("a", "Alpha", TaskStatus::Done);
        let mut t_b = make_task("b", "Beta", TaskStatus::InProgress);
        t_b.depends_on = vec!["a".to_string()];
        let mut t_c = make_task("c", "Gamma", TaskStatus::InProgress);
        t_c.depends_on = vec!["a".to_string()];
        let mut t_d = make_task("d", "Delta", TaskStatus::Todo);
        t_d.depends_on = vec!["b".to_string(), "c".to_string()];

        let board = TaskBoard {
            tasks: vec![t_a, t_b, t_c, t_d],
        };
        let lines = render_dag(&board, 15, 60);
        let plain_text: String = lines
            .iter()
            .map(|l| strip_ansi(l))
            .collect::<Vec<_>>()
            .join("\n");
        // Level 0: Alpha, Level 1: Beta + Gamma, Level 2: Delta
        assert!(plain_text.contains("Level 0 (roots):"));
        assert!(plain_text.contains("Alpha"));
        assert!(plain_text.contains("Level 1:"));
        assert!(plain_text.contains("Beta"));
        assert!(plain_text.contains("Gamma"));
        assert!(plain_text.contains("Level 2:"));
        assert!(plain_text.contains("Delta"));
    }

    #[test]
    fn test_render_dag_with_cycle() {
        // A → B → A (cycle)
        let mut t1 = make_task("t1", "Cyclic A", TaskStatus::Todo);
        t1.depends_on = vec!["t2".to_string()];
        let mut t2 = make_task("t2", "Cyclic B", TaskStatus::Todo);
        t2.depends_on = vec!["t1".to_string()];

        let board = TaskBoard {
            tasks: vec![t1, t2],
        };
        let lines = render_dag(&board, 10, 60);
        let plain_text: String = lines
            .iter()
            .map(|l| strip_ansi(l))
            .collect::<Vec<_>>()
            .join("\n");
        // The cycle tasks should appear in a "Cycle" level
        assert!(plain_text.contains("Cycle (unresolved):"));
        assert!(plain_text.contains("Cyclic A"));
        assert!(plain_text.contains("Cyclic B"));
    }

    #[test]
    fn test_render_dag_shows_assigned_agent() {
        let mut t1 = make_task("t1", "Deploy", TaskStatus::InProgress);
        t1.assigned_to = Some("agent-42".to_string());

        let board = TaskBoard { tasks: vec![t1] };
        let lines = render_dag(&board, 10, 60);
        let plain_text: String = lines
            .iter()
            .map(|l| strip_ansi(l))
            .collect::<Vec<_>>()
            .join("\n");
        assert!(plain_text.contains("agent-42"));
    }

    #[test]
    fn test_render_dag_respects_rows() {
        // Many tasks but small rows — should truncate
        let mut tasks = Vec::new();
        for i in 0..20 {
            tasks.push(make_task(
                &format!("t{}", i),
                &format!("Task {}", i),
                TaskStatus::Todo,
            ));
        }
        let board = TaskBoard { tasks };
        let lines = render_dag(&board, 5, 60);
        assert_eq!(lines.len(), 5);
    }

    // -----------------------------------------------------------------------
    // Rendering tests — stats line
    // -----------------------------------------------------------------------

    #[test]
    fn test_render_stats_line_basic() {
        let stats = TaskBoardStats {
            total: 8,
            done: 3,
            blocked: 1,
            success_rate: 3.0 / 8.0,
            total_cost: None,
        };
        let line = render_stats_line(&stats, 80);
        assert!(line.contains("Tasks: 8"));
        assert!(line.contains("Done: 3"));
        assert!(line.contains("Blocked: 1"));
        assert!(line.contains("38%")); // 3/8 = 37.5%, rounds to 38
        assert!(!line.contains("Cost"));
    }

    #[test]
    fn test_render_stats_line_with_costs() {
        let stats = TaskBoardStats {
            total: 5,
            done: 2,
            blocked: 0,
            success_rate: 0.4,
            total_cost: Some(4.2),
        };
        let line = render_stats_line(&stats, 80);
        assert!(line.contains("Tasks: 5"));
        assert!(line.contains("Cost: $4.20"));
    }

    #[test]
    fn test_render_stats_line_without_costs() {
        let stats = TaskBoardStats {
            total: 3,
            done: 1,
            blocked: 0,
            success_rate: 1.0 / 3.0,
            total_cost: None,
        };
        let line = render_stats_line(&stats, 80);
        assert!(!line.contains("Cost"));
    }

    #[test]
    fn test_render_stats_line_zero_tasks() {
        let stats = TaskBoardStats {
            total: 0,
            done: 0,
            blocked: 0,
            success_rate: 0.0,
            total_cost: None,
        };
        let line = render_stats_line(&stats, 80);
        assert!(line.contains("Tasks: 0"));
        assert!(line.contains("Done: 0 (0%)"));
    }

    #[test]
    fn test_render_stats_line_truncated() {
        let stats = TaskBoardStats {
            total: 100,
            done: 50,
            blocked: 10,
            success_rate: 0.5,
            total_cost: Some(1234.56),
        };
        let line = render_stats_line(&stats, 20);
        // Should be truncated to 20 chars
        assert!(line.chars().count() <= 20);
    }
}
