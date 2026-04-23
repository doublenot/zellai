Title: Create `teams.rs` module with team layout generation logic (step 7 foundation)
Files: src/teams.rs, src/lib.rs
Issue: none

## Description

Begin step 7 by creating the `src/teams.rs` module referenced in SCHEMA.md. This module contains the pure-logic layer for generating team pane layouts from config. It does NOT contain CLI or Zellij API calls — just the data transformation from `TeamsConfig` + optional `zellai.toml` overrides into a list of pane specifications.

### What to build

1. **Create `src/teams.rs` with the following types and functions:**

   ```rust
   //! Teams layout generation — transforms TeamsConfig into pane specifications.

   use crate::config::{TeamsConfig, TeamsLayout};
   use crate::workspace::{PaneConfig, PaneDirection};

   /// Generate a list of PaneConfigs from a TeamsConfig and working directory.
   ///
   /// The returned Vec represents the panes to create, in order.
   /// The first pane is always the orchestrator (or first worker in equal-grid).
   pub fn generate_team_layout(config: &TeamsConfig, working_dir: &str) -> Vec<PaneConfig> { ... }
   ```

   Layout rules by `TeamsLayout`:
   - **`OrchestratorTop`**: One orchestrator pane (horizontal, full width at top). Then `worker_count` worker panes split vertically below.
   - **`OrchestratorLeft`**: One orchestrator pane (vertical, full height on left). Then `worker_count` worker panes split horizontally to the right.
   - **`EqualGrid`**: No special orchestrator. `worker_count + 1` panes in alternating horizontal/vertical splits to approximate a grid.
   - **`Custom`**: Return an empty Vec (custom layouts come from `[[teams.layout]]` in TOML, which is a future addition).

   Pane naming convention:
   - Orchestrator: `name = "orchestrator"`, `agent = config.orchestrator_agent`
   - Workers: `name = "worker-1"`, `name = "worker-2"`, etc., `agent = config.worker_agent`
   - Command for each pane: `["zellai", "run", "--agent", "<agent>", "--", "<agent>"]`

2. **Uncomment the `teams` module in `src/lib.rs`:**
   Change `// mod teams;` to `pub mod teams;`

3. **Write thorough unit tests in `teams.rs`:**
   - `test_orchestrator_top_layout` — verify pane count is `1 + worker_count`, first pane is orchestrator, workers are vertical splits
   - `test_orchestrator_left_layout` — first pane is orchestrator, workers are horizontal splits
   - `test_equal_grid_layout` — all panes are workers/equal, count is `worker_count + 1`
   - `test_custom_returns_empty` — Custom layout returns empty vec
   - `test_default_config_layout` — using `TeamsConfig::default()` produces expected 3-pane layout
   - `test_zero_workers` — `worker_count = 0` produces only orchestrator (or 1 pane for equal-grid)
   - `test_pane_commands` — verify generated command arrays are correct

### Verification
```sh
cargo build --target wasm32-wasip1 && cargo clippy --target wasm32-wasip1 && cargo test --lib
```
