Title: Task board rendering — Kanban and DAG ASCII views
Files: src/task_board.rs (extend with rendering functions + tests)
Issue: none

## Description

The `task_board.rs` module has a complete data model (TaskBoard, Task, TaskStatus, TaskBoardStats) with Kanban grouping (`tasks_by_status`), DAG levels (`dependency_levels`), stats (`aggregate_stats`), and JSON parsing — but **zero rendering functions**. The vision specifies "Task Board views: Kanban and dependency-aware DAG tree (ASCII, level-grouped)" as a dedicated pane view in the orchestrator layout.

Add pure rendering functions to `task_board.rs` that produce `Vec<String>` output (same pattern as `sidebar.rs`). No Zellij API imports — everything must be testable on the host target.

### Functions to implement

1. **`render_kanban(board: &TaskBoard, rows: usize, cols: usize) -> Vec<String>`**
   - Renders a horizontal Kanban board with columns for each status (Todo | In Progress | Review | Done | Blocked).
   - Each column header shows the status name and task count.
   - Tasks are listed vertically within each column, truncated to fit.
   - Uses box-drawing characters for borders (same style as sidebar: `│`, `─`, `┬`, etc.).
   - Columns are distributed evenly across `cols`.
   - Output is exactly `rows` lines (pad with empty lines if needed).

2. **`render_dag(board: &TaskBoard, rows: usize, cols: usize) -> Vec<String>`**
   - Renders the dependency DAG as a level-grouped ASCII tree.
   - Uses `dependency_levels()` to get tasks organized by level.
   - Each level is labeled: `Level 0 (roots):`, `Level 1:`, etc.
   - Tasks within a level are grouped by status (using a status icon: ○ todo, ◐ in-progress, ◑ review, ● done, ✕ blocked).
   - Assigned agent shown in dim after the task title.
   - Truncate/scroll if content exceeds `rows`.
   - Output is exactly `rows` lines.

3. **`render_stats_line(stats: &TaskBoardStats, cols: usize) -> String`**
   - Single-line summary: `Tasks: 8 | Done: 3 (37%) | Blocked: 1 | Cost: $4.20`
   - Omit cost section if `total_cost` is `None`.
   - Truncate to fit `cols`.

### ANSI colors

- Use the same constants from sidebar.rs pattern (define locally or make shared):
  - GREEN for done, YELLOW for in-progress/review, RED for blocked, DIM for metadata
  - BOLD for headers, RESET after each colored segment

### Tests to add (aim for 10-15)

- `render_kanban` with empty board → shows headers with zero counts
- `render_kanban` with tasks in various statuses → correct column grouping
- `render_kanban` respects `rows` and `cols` bounds
- `render_kanban` truncates long task titles
- `render_dag` with empty board → empty/minimal output
- `render_dag` with simple chain (A→B→C) → 3 levels
- `render_dag` with diamond dependency → correct level grouping
- `render_dag` with cycle → cycle tasks appear at end
- `render_dag` shows assigned agent
- `render_stats_line` with basic stats
- `render_stats_line` with costs
- `render_stats_line` without costs (omits cost section)
- `render_stats_line` with zero tasks

### Verification

```sh
cargo build --target wasm32-wasip1 && cargo clippy --target wasm32-wasip1 && cargo test --lib
```

All existing 20 task_board tests must continue to pass. New rendering tests must pass.
