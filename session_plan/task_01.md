Title: Add `zellai log` CLI command and per-pane execution logging
Files: src/bin/zellai/main.rs, src/bin/zellai/run.rs, src/bin/zellai/status_writer.rs, src/bin/zellai/log.rs (new)
Issue: none

## Description

The vision specifies per-pane execution logs at `~/.local/share/zellai/sessions/<workspace>/<pane>.log` and a `zellai log <pane>` CLI command. Neither exists today.

### Implementation

1. **Create `src/bin/zellai/log.rs`** — new module implementing the `zellai log` subcommand.
   - `pub fn run(pane_name: &str, workspace: Option<&str>, follow: bool, lines: Option<usize>) -> Result<(), String>`
   - Resolves the log directory: `<sessions_dir>/<workspace>/` where `sessions_dir` defaults to `~/.local/share/zellai/sessions` (use the same `resolve_sessions_dir_with_env` pattern from `status_writer.rs`).
   - If no `--workspace` flag, look for a `ZELLAI_WORKSPACE` env var, then fall back to listing all workspaces and searching for a matching pane name.
   - Reads the log file `<pane>.log` and prints it to stdout.
   - If `--lines N` is given, show only the last N lines (tail behavior).
   - If `--follow` is given, print a note that follow mode is not yet supported (placeholder for future `tail -f` behavior).
   - If the file doesn't exist, print a helpful error: "No log found for pane '<pane>'. Logs are created when agents are run via 'zellai run'."

2. **Update `src/bin/zellai/status_writer.rs`** — add log file writing.
   - Add a `log_file: Option<std::fs::File>` field to `StatusWriter`.
   - In `StatusWriter::new()`, create the workspace log directory `<sessions_dir>/<workspace>/` if it doesn't exist (workspace name from `ZELLAI_WORKSPACE` env var, defaulting to "default").
   - Add a `pub fn write_log_line(&mut self, line: &str)` method that appends a timestamped line to the log file.
   - Call `write_log_line` from the existing status transitions in `write_status()` — log status changes like "Status changed to: thinking (tool: Read)", "Status changed to: waiting", etc.

3. **Update `src/bin/zellai/main.rs`** — add `Log` variant to `Commands` enum:
   ```rust
   Log {
       /// Pane name to show logs for
       pane: String,
       /// Workspace name (default: from ZELLAI_WORKSPACE env or "default")
       #[arg(long)]
       workspace: Option<String>,
       /// Number of lines to show (default: all)
       #[arg(long, short = 'n')]
       lines: Option<usize>,
       /// Follow log output (not yet implemented)
       #[arg(long, short = 'f')]
       follow: bool,
   },
   ```
   - Wire it to `log::run()`.

4. **Add `mod log;` to `main.rs`** module declarations.

### Testing

- Add unit tests in `log.rs` for log directory resolution and path construction.
- Add unit tests in `status_writer.rs` for log line formatting.
- Verify: `cargo build --target wasm32-wasip1 && cargo clippy --target wasm32-wasip1 && cargo test --lib`
