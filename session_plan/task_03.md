Title: Add workspace CLI commands (`zellai new`, `zellai list`, `zellai kill`)
Files: src/bin/zellai/main.rs, src/bin/zellai/workspace_cmd.rs
Issue: none

## Context

This task adds the CLI subcommands for workspace management, building on the `workspace.rs` data model from task_02. It does NOT implement `zellai attach` (which requires Zellij session interaction — deferred to a later session).

## Implementation

### New file: `src/bin/zellai/workspace_cmd.rs`

This module implements the workspace subcommands for the CLI binary.

#### `zellai new <name>`

```
zellai new my-project
zellai new my-project --template team
zellai new my-project --template single-agent --dir /path/to/project
```

- Creates a new workspace definition using the specified template (default: `single-agent`)
- Sets `working_dir` to `--dir` value or current directory (`std::env::current_dir()`)
- Saves the workspace file via `workspace::save_workspace()`
- Prints confirmation: `Created workspace 'my-project' (template: single-agent)`
- If workspace already exists, error unless `--force` is passed

#### `zellai list`

```
zellai list
```

- Calls `workspace::list_workspaces()`
- For each workspace, loads it and prints a summary line:
  ```
  my-project    single-agent    /home/user/projects/app    2 panes    saved 2h ago
  team-auth     team            /home/user/projects/auth   3 panes    saved 1d ago
  ```
- If no workspaces exist, print: `No saved workspaces. Create one with: zellai new <name>`

#### `zellai kill <name>`

```
zellai kill my-project
```

- Calls `workspace::delete_workspace(name)`
- Prints confirmation: `Deleted workspace 'my-project'`
- If workspace doesn't exist, print error and exit 1

### Modify: `src/bin/zellai/main.rs`

Add the new subcommands to the `Commands` enum:

```rust
/// Create a new workspace
New {
    /// Workspace name
    name: String,
    /// Workspace template
    #[arg(long, default_value = "single-agent")]
    template: String,
    /// Working directory (default: current directory)
    #[arg(long)]
    dir: Option<String>,
    /// Overwrite existing workspace
    #[arg(long)]
    force: bool,
},
/// List saved workspaces
List,
/// Delete a saved workspace
Kill {
    /// Workspace name to delete
    name: String,
},
```

Add `mod workspace_cmd;` and dispatch to the new module functions.

### Human-readable time formatting

Add a helper function `fn format_relative_time(epoch_secs: u64) -> String` in `workspace_cmd.rs` that converts a Unix timestamp to a relative string like "2h ago", "1d ago", "just now". Use `std::time::SystemTime::now()` for the current time.

### Unit tests

- Test `format_relative_time` with various deltas
- Test template name parsing (string → `WorkspaceTemplate` enum)

## Verification

```sh
cargo build --target wasm32-wasip1 && cargo clippy --target wasm32-wasip1 && cargo test --lib
```

Also test the native binary:
```sh
cargo build && cargo clippy
```

Manually verify (if possible):
```sh
cargo run --bin zellai-cli -- new test-workspace --template single-agent
cargo run --bin zellai-cli -- list
cargo run --bin zellai-cli -- kill test-workspace
```
