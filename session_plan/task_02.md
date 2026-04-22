Title: Implement `zellai run` generic wrapper command
Files: src/bin/zellai/main.rs, src/bin/zellai/run.rs, src/bin/zellai/status_writer.rs
Issue: none

## Overview

Implement Milestone 5 (part 1): the `zellai run <command>` generic wrapper. This CLI command wraps any agent process, writes status files on its behalf, and cleans up on exit.

The wrapper is a **native CLI binary** (not WASM). It:
1. Generates a unique session ID (hostname + PID, or from `$ZELLAI_SESSION_ID` env)
2. Writes an initial "thinking" status file to the sessions directory
3. Spawns the child process, forwarding stdin/stdout/stderr
4. Periodically updates the status file (git branch, dirty state, working dir)
5. When the child exits, writes a final "idle" status then deletes the file
6. Forwards signals (SIGINT, SIGTERM) to the child process

## Files to create/modify

### `src/bin/zellai/main.rs` — Add `Run` subcommand
Add to the `Commands` enum:
```rust
/// Run a command with zellai status tracking
Run {
    /// Agent name (default: detect from command, or "unknown")
    #[arg(long, default_value = "unknown")]
    agent: String,

    /// The command and arguments to run
    #[arg(trailing_var_arg = true, required = true)]
    command: Vec<String>,
},
```

### `src/bin/zellai/status_writer.rs` — Status file writer (NEW)
A small module that writes status JSON files atomically (write to `.tmp`, then `mv`). This is the write-side counterpart to the plugin's read-side `status_bridge.rs`.

```rust
pub struct StatusWriter {
    session_id: String,
    agent: String,
    sessions_dir: PathBuf,
}
```

Methods:
- `new(session_id, agent, sessions_dir) -> Self`
- `write_status(status: &str, last_message: Option<&str>, needs_attention: bool) -> io::Result<()>` — writes the full JSON status file atomically. Internally calls `collect_git_info()` and `get_working_dir()`.
- `cleanup(&self) -> io::Result<()>` — removes the status file
- `status_file_path(&self) -> PathBuf`

Helper functions (private):
- `collect_git_info() -> (Option<String>, bool)` — runs `git rev-parse --abbrev-ref HEAD` and `git diff --quiet`
- `detect_agent(command: &str) -> &str` — maps command names to agent kinds: "claude" for claude, "codex" for codex, "gemini" for gemini, "aider" for aider, "opencode" for opencode, else "unknown"
- `generate_session_id() -> String` — reads `$ZELLAI_SESSION_ID` env, or generates `hostname-PID`
- `resolve_sessions_dir() -> PathBuf` — reads `$ZELLAI_SESSIONS_DIR` or `$XDG_DATA_HOME/zellai/sessions` or `$HOME/.local/share/zellai/sessions`

The JSON format must exactly match SCHEMA.md. Use `serde_json` to serialize (import `AgentStatus` from the library's `status` module, or build the JSON manually to avoid coupling). Building JSON manually is preferred to avoid import complexity — use `serde_json::json!()`.

### `src/bin/zellai/run.rs` — Run command implementation (NEW)
```rust
pub fn run(agent: String, command: Vec<String>) -> Result<(), String>
```

Implementation:
1. Resolve session ID and sessions dir via `StatusWriter` helpers
2. Create `StatusWriter`
3. Auto-detect agent from `command[0]` if agent is "unknown"
4. Write initial status: `"thinking"`, no message, `needs_attention: false`
5. Spawn the child process using `std::process::Command`:
   - `command[0]` as the program, `command[1..]` as args
   - Inherit stdin, stdout, stderr (the user interacts with the agent directly)
   - Set `ZELLAI_SESSION_ID` env var on the child (so hooks can use it too)
6. Set up a SIGINT/SIGTERM handler using `ctrlc` crate — NO, avoid new dependencies. Instead, just let the default signal handling kill the child (since it inherits the process group). Use a simple approach: just `child.wait()` and handle the exit.
7. After `child.wait()` returns:
   - Write final status: `"idle"`, last_message = exit code info, `needs_attention: false`  
   - Call `cleanup()` to remove the status file
   - Exit with the child's exit code

For periodic updates (git info refresh), use a background thread that sleeps and rewrites the status file every 5 seconds while the child is running. Use `Arc<AtomicBool>` for the "still running" flag.

## Edge cases

- If the command is empty or not found: print error, exit 1
- If sessions dir can't be created: print error, exit 1
- JSON special chars in working_dir: `serde_json::json!()` handles escaping correctly
- Child killed by signal: detect via `ExitStatus::code()` returning None, write appropriate exit info

## Verification

```sh
cargo build 2>&1  # native build must succeed (this is CLI code, not WASM)
cargo test --lib  # existing tests still pass
cargo clippy --target wasm32-wasip1  # plugin lint still clean
# Manual smoke test:
./target/debug/zellai-cli run -- echo hello
# Should print "hello", write+delete a status file in sessions dir
```
