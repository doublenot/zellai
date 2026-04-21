Title: Wire status bridge into plugin event loop
Files: src/lib.rs, src/status_bridge.rs (minor addition)
Issue: none

## Description

Update `src/lib.rs` to integrate the `StatusBridge` from task_01 into the Zellij plugin event loop. The plugin will use `run_command` to read status files and `RunCommandResult` events to feed data into the bridge.

### Context: How Zellij run_command works

`run_command` is an async Zellij host API call. You call it with a command + args + a context map. The result arrives later as an `Event::RunCommandResult` containing:
- `exit_code: Option<i32>`
- `stdout: Vec<u8>` 
- `stderr: Vec<u8>`
- `context: BTreeMap<String, String>` — the same context map you passed in

We use the context map to tag commands so we know what to do with the result.

### What to build

#### In `src/lib.rs`:

1. **Add fields to `ZellaiPlugin`:**
   - `bridge: status_bridge::StatusBridge`
   - `config: config::ZellaiConfig`
   - Remove the unused `loading: bool` field

2. **In `load()`:**
   - Parse configuration from the `_configuration` BTreeMap (look for a `config` key containing TOML, fall back to `ZellaiConfig::default()`)
   - Initialize `StatusBridge` with `config.bridge.sessions_dir` and `config.bridge.stale_threshold_s`

3. **In the `Timer` handler:**
   - Call `run_command` to list status files: `run_command(&["ls", "-1"], BTreeMap::from([("zellai_cmd".to_string(), "list_sessions".to_string()), ("sessions_dir".to_string(), self.bridge.sessions_dir.clone())]))`
   - Actually: use `ls` on the sessions directory. The command should be `["ls", "-1", &self.bridge.sessions_dir]` with context tag `"list_sessions"`.
   - Also call `self.bridge.mark_stale(now)` — but we don't have `now`. Use a tick counter instead: add `tick_count: u64` to the plugin, increment on each timer. This gives relative time for stale detection. Alternatively, issue a `date +%s` command. **Simplest approach**: add a `tick_count` and compute approximate epoch from ticks * 0.5s. But this is unreliable. **Better**: skip mark_stale in this task — the timer just triggers the list command. Stale marking will work once we have real epoch timestamps from status files. For now, just trigger the file listing.

4. **Add a `RunCommandResult` handler:**
   - Match on `context.get("zellai_cmd")`:
     - `"list_sessions"` — parse stdout as newline-separated filenames. For each `.json` file, issue a `run_command` to `cat` the file: `run_command(&["cat", &format!("{}/{}", sessions_dir, filename)], BTreeMap::from([("zellai_cmd", "read_status"), ("session_file", filename)]))`
     - `"read_status"` — if exit_code == 0, call `self.bridge.update_from_json(session_id, &stdout_str)` where session_id is derived from the filename (strip `.json` extension). If exit_code != 0, call `self.bridge.remove_session(session_id)`.
   - Return `true` to trigger re-render after processing.

5. **Handle `FileSystemCreate`/`FileSystemUpdate`/`FileSystemDelete` events:**
   - For now, these just return `true` to trigger a re-render (the timer will pick up changes). Full reactive file watching can be refined later.

6. **In `render()`:**
   - If `self.bridge.has_agents()` is false, keep the current placeholder rendering.
   - If agents exist, render a simple list for now: iterate `self.bridge.agents_sorted()` and print one line per agent: `"│ {icon} {session_id}: {status} │"`. Proper sidebar rendering is task_03.

#### In `src/status_bridge.rs` (minor):

- Add `pub fn sessions_dir(&self) -> &str` getter if `sessions_dir` is private
- Make sure `sessions_dir` field is `pub` or provide access

### Important constraints

- **No `std::fs`** — all file I/O goes through `run_command`
- **No blocking** — `run_command` is async; results come back via events
- The `render()` function must still return immediately
- Use `BTreeMap` for command context (Zellij API requirement)

### Verification

```sh
cargo build --target wasm32-wasip1 && cargo clippy --target wasm32-wasip1 && cargo test --lib
```

All existing tests must pass. The unit tests from task_01 must still pass. This task adds no new unit tests (the new code all touches Zellij API and can only be tested by loading the WASM).
