Title: Add example `zellai.toml` and improve README with CLI documentation
Files: zellai.example.toml (new), README.md, SCHEMA.md
Issue: none

## Description

Users have no example config file to copy and no CLI command reference in the README. This task creates both, significantly improving the first-run developer experience.

### Implementation

1. **Create `zellai.example.toml`** — a fully-commented example config file showing all available options with their defaults:
   ```toml
   # zellai configuration
   # Copy this to zellai.toml in your project root or ~/.config/zellai/zellai.toml
   
   [sidebar]
   # position = "left"              # left | right | bottom
   # card_density = "adaptive"      # compact | detailed | adaptive
   # attention_animation = true     # set false to disable pulsing indicators
   
   [teams]
   # default_layout = "orchestrator-top"  # orchestrator-top | orchestrator-left | equal-grid | custom
   # orchestrator_agent = "claude"        # agent for the orchestrator pane
   # worker_agent = "claude"              # default agent for worker panes
   # worker_count = 2                     # number of worker panes
   
   [teams.orchestrator]
   # task_board = false                     # enable the task board pane
   # task_board_columns = ["todo", "in-progress", "review", "done", "blocked"]
   # show_cost_tracking = false
   # dag_view = true
   
   [bridge]
   # sessions_dir = "~/.local/share/zellai/sessions"
   # poll_interval_ms = 500
   # stale_threshold_s = 60
   
   [keybindings]
   # next_attention = "Ctrl a"      # cycle to next pane needing attention
   # dismiss = "Ctrl d"             # dismiss current notification
   # jump_to = "Ctrl g"             # open pane picker (future)
   ```
   Note: Include the `[teams.orchestrator]` section only if task_02 has been completed (the OrchestratorConfig). If not, omit that section — the agent implementing this task should check whether `OrchestratorConfig` exists in `config.rs` before including it.

2. **Update `README.md`** — add comprehensive documentation:
   - **Installation** section: prerequisites (Rust 1.85+, Zellij), build instructions, `cargo install` from source
   - **Quick Start** section: `zellai init`, `zellai run`, loading the plugin in Zellij
   - **CLI Commands** section documenting all 11 subcommands with usage examples:
     - `zellai init` — install Claude Code hooks
     - `zellai run <command>` — wrap any agent with status tracking
     - `zellai wrap --agent <name> -- <command>` — named agent wrapper
     - `zellai new <name>` — create a workspace
     - `zellai list` — list workspaces
     - `zellai attach <name>` — restore a workspace
     - `zellai kill <name>` — delete a workspace
     - `zellai teams` — launch multi-agent layout
     - `zellai log <pane>` — view pane execution logs (only if task_01 was implemented)
     - `zellai doctor` — environment diagnostics
     - `zellai completions <shell>` — generate shell completions
   - **Configuration** section: link to `zellai.example.toml`, explain config file locations
   - **Plugin Loading** section: how to load the sidebar and status bar plugins in Zellij
   - Keep the existing screenshot SVG embed
   - Keep any existing content that's still accurate

3. **Update `SCHEMA.md`** — add a reference to the example config file in the Config Schema section:
   - Add a line: "See [`zellai.example.toml`](zellai.example.toml) for a fully-commented example configuration."

### Testing

- Verify the example TOML parses correctly: add a test in `config.rs` that reads `zellai.example.toml` from the repo root (use `include_str!`) and asserts it parses to the default config.
- Verify: `cargo build --target wasm32-wasip1 && cargo clippy --target wasm32-wasip1 && cargo test --lib`

### Notes

- The README should be practical, not verbose. Focus on what a new user needs to get started.
- All commented-out values in the example TOML should match the compiled defaults exactly.
- Check other task files to see what new commands/config exist before finalizing the README.
