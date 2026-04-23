Title: Implement `zellai attach` to complete workspace management (step 6)
Files: src/bin/zellai/main.rs, src/bin/zellai/workspace_cmd.rs
Issue: none

## Description

Complete step 6 of the roadmap by implementing the `zellai attach <name>` subcommand. This command loads a saved workspace and prints the Zellij commands needed to restore the layout (pane creation + agent launch). Since we cannot directly call the Zellij API from the native CLI binary, `attach` will generate and execute a Zellij action sequence.

### What to build

1. **Add `Attach` variant to `Commands` enum in `src/bin/zellai/main.rs`:**
   ```rust
   /// Attach to (restore) a saved workspace
   Attach {
       /// Workspace name to restore
       name: String,
   },
   ```
   Wire it to `workspace_cmd::cmd_attach(&name)` with the same `#[cfg]` pattern as the other workspace commands.

2. **Implement `cmd_attach` in `src/bin/zellai/workspace_cmd.rs`:**
   - Call `load_workspace(name)` to load the saved workspace JSON
   - If the workspace doesn't exist, return an error: `"workspace '{}' not found"`
   - For each pane in `ws.panes`, generate a Zellij CLI command:
     - First pane: `zellij action new-tab --name <ws.name> --cwd <ws.working_dir>`
     - Subsequent panes: `zellij action new-pane --direction <down|right> --cwd <ws.working_dir> --name <pane.name>`
       - `PaneDirection::Horizontal` → `--direction down`
       - `PaneDirection::Vertical` → `--direction right`
     - After creating each pane, if `pane.command` is non-empty: `zellij action write-chars '<command joined by space>\n'`
   - Execute these commands sequentially using `std::process::Command`
   - Print a summary: `"Attached workspace '{}' ({} panes)"` on success

3. **Add tests:**
   - Test that `cmd_attach` returns an error for a non-existent workspace name
   - Test the logic that maps `PaneDirection` to Zellij direction flags (extract a helper function `pane_direction_flag(dir: &PaneDirection) -> &str`)

### Verification
```sh
cargo build --target wasm32-wasip1 && cargo clippy --target wasm32-wasip1 && cargo test --lib
```

The native binary build should also be verified:
```sh
cargo build && cargo test --lib
```
