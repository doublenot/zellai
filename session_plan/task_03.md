Title: Wire task board rendering into the WASM plugin as a third render mode
Files: src/lib.rs, src/task_board.rs (minor), src/config.rs (minor)
Issue: none

## Description

Task 01 adds pure rendering functions to `task_board.rs`. This task wires them into the WASM plugin so they can actually be displayed in a Zellij pane. The task board is rendered in a dedicated pane (not the sidebar) — the vision specifies it as an "Orchestrator Task Board panel."

### Implementation

1. **Add a new `PluginMode::TaskBoard` variant** in `src/lib.rs`:
   - Extend the `PluginMode` enum with `TaskBoard`.
   - In `load()`, parse `mode = "task-board"` from the configuration BTreeMap to activate this mode.

2. **Task board file reading** in the update loop:
   - The task board is stored as a JSON file at a configurable path. Default: `<sessions_dir>/task_board.json`.
   - In `load()`, when mode is `TaskBoard`, also parse an optional `task_board_path` from the configuration.
   - In the `Timer` event handler, when mode is `TaskBoard`, issue a `run_command` to `cat` the task board file (same pattern as `read_status`).
   - In `handle_run_command_result`, handle `"read_task_board"` context: parse the JSON with `task_board::parse_task_board()` and store the result in a new `task_board: Option<TaskBoard>` field on `ZellaiPlugin`.

3. **Render the task board** in `render()`:
   - When `mode == TaskBoard`, call `task_board::render_kanban()` (or `render_dag()` based on a `view` config key).
   - Add a `task_board_view` field to `ZellaiPlugin` (default: `"kanban"`, alternative: `"dag"`), parsed from configuration.
   - Also render the stats line at the bottom using `render_stats_line()`.

4. **Add `view` key parsing** in config:
   - Not in `ZellaiConfig` (that's for the TOML file) — this comes from the plugin's BTreeMap configuration passed by Zellij at load time.
   - Parse `view` key: `"kanban"` (default) or `"dag"`.

5. **Key event to toggle view**:
   - In the `Key` event handler, when mode is `TaskBoard`, handle Tab key to toggle between `"kanban"` and `"dag"` views. Return `true` to trigger re-render.

### Fields to add to `ZellaiPlugin`

```rust
/// Parsed task board (only used in TaskBoard mode).
task_board_data: Option<task_board::TaskBoard>,
/// Task board file path.
task_board_path: String,
/// Current view mode: "kanban" or "dag".
task_board_view: String,
```

### Tests

This task primarily modifies WASM-gated code (`#[cfg(target_arch = "wasm32")]`), which cannot be unit-tested. Verification is through successful compilation:

```sh
cargo build --target wasm32-wasip1 && cargo clippy --target wasm32-wasip1 && cargo test --lib
```

All 187+ existing tests must continue to pass. The new rendering tests from Task 01 verify correctness of the rendering functions themselves.

### Depends on

Task 01 must be completed first (provides `render_kanban`, `render_dag`, `render_stats_line`).
