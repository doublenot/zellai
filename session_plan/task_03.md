Title: Add unit tests for status_writer and agent detection
Files: src/bin/zellai/status_writer.rs, src/bin/zellai/run.rs
Issue: none

## Overview

Add comprehensive unit tests for the `StatusWriter` and agent detection logic created in Task 02. Also add a `detect_agent()` function that maps command names to agent kinds, and verify JSON output matches SCHEMA.md exactly.

This task hardens the wrapper code with tests and ensures the status file format is correct. These are native (non-WASM) tests that can use the filesystem.

## Tests to write

### In `src/bin/zellai/status_writer.rs`

1. **`test_generate_session_id_from_env`** — Set `ZELLAI_SESSION_ID` env, verify it's returned
2. **`test_generate_session_id_default`** — Without env var, verify format is `hostname-PID`
3. **`test_resolve_sessions_dir_from_env`** — Set `ZELLAI_SESSIONS_DIR`, verify it's returned
4. **`test_resolve_sessions_dir_xdg`** — Set `XDG_DATA_HOME`, verify path
5. **`test_resolve_sessions_dir_default`** — Without env vars, verify `$HOME/.local/share/zellai/sessions`
6. **`test_detect_agent_claude`** — `detect_agent("claude")` returns `"claude"`; also test `/usr/bin/claude`, `claude-code`
7. **`test_detect_agent_codex`** — `detect_agent("codex")` returns `"codex"`
8. **`test_detect_agent_gemini`** — `detect_agent("gemini")` returns `"gemini"`
9. **`test_detect_agent_aider`** — `detect_agent("aider")` returns `"aider"`
10. **`test_detect_agent_unknown`** — `detect_agent("vim")` returns `"unknown"`
11. **`test_write_status_creates_valid_json`** — Create a `StatusWriter` with a temp dir, call `write_status()`, read the file, parse as JSON, verify all SCHEMA.md fields are present and correctly typed
12. **`test_write_status_atomic`** — After `write_status()`, verify no `.tmp` file remains
13. **`test_cleanup_removes_file`** — After `write_status()` + `cleanup()`, verify file is gone
14. **`test_json_escapes_special_chars`** — Use a working dir with quotes and backslashes, verify valid JSON output (this was Bug #4 from the assessment — the wrapper should handle it correctly since we use serde_json)

### In `src/bin/zellai/run.rs`

15. **`test_detect_agent_from_command`** — Verify the agent auto-detection logic that maps `command[0]` to agent name when `--agent` isn't specified

## Implementation notes

- Use `tempfile` or just `std::env::temp_dir()` + a random subdir for test isolation. Since we want to avoid new deps, use `std::env::temp_dir()` with a UUID-like name (or just PID + test name).
- Tests that set env vars must be careful about parallel execution. Use unique env var names or accept that `cargo test` runs tests in parallel (use `std::sync::Mutex` or `#[serial]` — but again, no new deps). The safest approach: have `generate_session_id` and `resolve_sessions_dir` accept parameters or closures for env lookups so they're testable without actually setting env vars.
- Refactor `generate_session_id()` and `resolve_sessions_dir()` to take an `env_var: impl Fn(&str) -> Option<String>` parameter, defaulting to `std::env::var(...).ok()` in production. This makes tests deterministic.

## Verification

```sh
cargo test --lib  # existing 75 tests still pass
cargo test --bin zellai-cli  # new status_writer tests pass
cargo clippy --target wasm32-wasip1  # plugin lint clean
cargo build --target wasm32-wasip1 --lib  # WASM build still works
```
