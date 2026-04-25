Title: Add Orchestrator Task Board config schema and data model
Files: src/config.rs, src/task_board.rs (new), src/lib.rs
Issue: none

## Description

The vision describes an Orchestrator Task Board: a dedicated pane view showing task-level state across the active team, with Kanban columns (todo/in-progress/review/done/blocked), DAG dependency tree, aggregate stats, and optional cost tracking. Today, the config schema has no task board fields and no data model exists.

This task lays the foundation by adding the config schema and pure-data model. Rendering will come in a future task.

### Implementation

1. **Update `src/config.rs`** — add `OrchestratorConfig` to `TeamsConfig`:
   ```rust
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
   ```
   - Add `pub orchestrator: OrchestratorConfig` field to `TeamsConfig`.
   - Update `TeamsConfig::default()` to include `orchestrator: OrchestratorConfig::default()`.
   - Add tests for parsing the orchestrator config from TOML (both partial and full).

2. **Create `src/task_board.rs`** — pure-data model for task board state:
   ```rust
   /// Status of a task in the orchestrator's task board.
   #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
   #[serde(rename_all = "kebab-case")]
   pub enum TaskStatus {
       Todo,
       InProgress,
       Review,
       Done,
       Blocked,
   }

   /// A single task tracked by the orchestrator.
   #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
   pub struct Task {
       pub id: String,
       pub title: String,
       pub status: TaskStatus,
       /// Agent session_id assigned to this task (if any)
       pub assigned_to: Option<String>,
       /// IDs of tasks this task depends on
       pub depends_on: Vec<String>,
       /// Optional cost/token count
       pub cost: Option<f64>,
       /// Unix epoch seconds when task was created
       pub created_at: u64,
       /// Unix epoch seconds when task was last updated
       pub updated_at: u64,
   }

   /// The full task board state, serialized to/from a JSON file.
   #[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
   pub struct TaskBoard {
       pub tasks: Vec<Task>,
   }
   ```
   - Add helper methods on `TaskBoard`:
     - `pub fn tasks_by_status(&self) -> BTreeMap<&TaskStatus, Vec<&Task>>` — group tasks by status for Kanban rendering
     - `pub fn dependency_levels(&self) -> Vec<Vec<&Task>>` — compute DAG levels (BFS from roots with no dependencies) for the ASCII tree view
     - `pub fn aggregate_stats(&self) -> TaskBoardStats` where `TaskBoardStats` has `total`, `done`, `blocked`, `success_rate: f64`, `total_cost: Option<f64>`
   - Add `pub fn parse_task_board(json: &str) -> Result<TaskBoard, serde_json::Error>`
   - Add comprehensive unit tests: parsing, grouping by status, DAG level computation (including cycles/missing deps handled gracefully), aggregate stats.

3. **Update `src/lib.rs`** — add `pub mod task_board;` to the module list.

### Testing

- Unit tests in `task_board.rs` for all pure-logic methods.
- Unit tests in `config.rs` for orchestrator config parsing.
- Verify: `cargo build --target wasm32-wasip1 && cargo clippy --target wasm32-wasip1 && cargo test --lib`

### Notes

- The task board data file will be stored at `<sessions_dir>/<workspace>/task_board.json`, but file I/O is NOT part of this task — only the data model and parsing.
- The rendering of Kanban columns and ASCII DAG tree will be a separate future task.
- Keep all code pure (no `zellij_tile` imports) so it's unit-testable.
