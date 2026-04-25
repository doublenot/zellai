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
}
