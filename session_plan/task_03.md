Title: Implement `zellai teams` CLI subcommand (step 7 CLI layer)
Files: src/bin/zellai/main.rs, src/bin/zellai/teams_cmd.rs
Issue: none

## Description

Wire the `teams.rs` pure-logic module (from task 02) into a CLI subcommand. This is the user-facing entry point for `zellai teams` — it reads config, generates the layout, and executes Zellij commands to create the panes.

### What to build

1. **Create `src/bin/zellai/teams_cmd.rs`:**

   ```rust
   //! `zellai teams` subcommand — launches a multi-agent team layout.
   ```

   Implement `pub fn cmd_teams(layout: Option<&str>, dir: Option<&str>) -> Result<(), String>`:
   - Load `zellai.toml` from the working directory (or `dir` if specified) using `zellai::config::parse_config`. If no file exists, use `ZellaiConfig::default()`.
     - Look for `zellai.toml` in the working directory. Read it with `std::fs::read_to_string`. If not found, fall back to defaults.
   - If `layout` is provided, override `config.teams.default_layout` with the parsed `TeamsLayout` variant:
     - `"orchestrator-top"` → `OrchestratorTop`
     - `"orchestrator-left"` → `OrchestratorLeft`
     - `"equal-grid"` → `EqualGrid`
     - anything else → return error with valid options
   - Call `zellai::teams::generate_team_layout(&config.teams, &working_dir)` to get the pane list
   - If the pane list is empty, print a message and return (custom layout not yet supported)
   - Execute Zellij commands to create the layout (same pattern as `attach`):
     - First pane: `zellij action new-tab --name team --cwd <working_dir>`
     - Subsequent panes: `zellij action new-pane --direction <dir> --cwd <working_dir> --name <name>`
     - For each pane: `zellij action write-chars '<command>\n'` to start the agent
   - Print summary: `"Launched team: {} orchestrator + {} workers"` or similar

2. **Add `Teams` variant to `Commands` enum in `src/bin/zellai/main.rs`:**
   ```rust
   /// Launch a multi-agent team layout
   Teams {
       /// Layout override (orchestrator-top, orchestrator-left, equal-grid)
       #[arg(long)]
       layout: Option<String>,

       /// Working directory (default: current directory)
       #[arg(long)]
       dir: Option<String>,
   },
   ```
   Wire to `teams_cmd::cmd_teams(layout.as_deref(), dir.as_deref())` with the `#[cfg(not(target_arch = "wasm32"))]` guard.

3. **Add the `mod teams_cmd;` declaration** in `main.rs` with the same `#[cfg]` guard as `workspace_cmd`.

4. **Add tests in `teams_cmd.rs`:**
   - Test layout string parsing helper (extract `parse_teams_layout(s: &str) -> Result<TeamsLayout, String>`)
   - Test that config loading falls back to defaults when no `zellai.toml` exists
   - Test that invalid layout strings produce helpful errors

### Verification
```sh
cargo build --target wasm32-wasip1 && cargo clippy --target wasm32-wasip1 && cargo test --lib
cargo build  # native build
```
